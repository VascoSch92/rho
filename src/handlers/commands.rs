//! Command execution — processes AppCommands into API calls and state changes.

use anyhow::Result;
use tracing::{debug, error, info, warn};

use super::AppCommand;
use crate::cli::Args;
use crate::client::{
    AgentConfig, AgentServerClient, EventStream, ExecutionStatus, LLMConfig, LocalWorkspace,
    SecurityAnalyzer, ServerConfirmationPolicy, StartConversationRequest,
};
use crate::state::{AppState, ConfirmationPolicy, DisplayMessage, InputMode, Notification};

/// Process a command and return true if should exit
pub async fn process_command(
    state: &mut AppState,
    client: &AgentServerClient,
    event_stream: &mut Option<EventStream>,
    command: AppCommand,
    args: &Args,
    llm_config: &LLMConfig,
) -> Result<bool> {
    match command {
        AppCommand::SendMessage(message) => {
            // Queue the message if the agent is busy — don't display yet,
            // it will be displayed when sent from the queue so the order is
            // PROMPT_1, ANSWER_1, PROMPT_2, ANSWER_2
            if state.conversation_id.is_some() && state.is_running() {
                state.message_queue.push_back(message);
                info!(
                    "Agent busy, queued message ({} in queue)",
                    state.message_queue.len()
                );
                return Ok(false);
            }

            // (User message was already displayed by the input handler
            // before this async command was invoked, so it appears instantly
            // even though conversation creation may take a moment.)

            // Ensure we have a conversation
            if state.conversation_id.is_none() {
                // Build workspace config
                let workspace_dir = args.workspace.clone().unwrap_or_else(|| {
                    std::env::current_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| ".".to_string())
                });

                // Build conversation request with default development tools
                // Convert client-side policy to server-side policy
                let server_policy = match state.confirmation_policy {
                    ConfirmationPolicy::NeverConfirm => ServerConfirmationPolicy::NeverConfirm,
                    ConfirmationPolicy::AlwaysConfirm => ServerConfirmationPolicy::AlwaysConfirm,
                    ConfirmationPolicy::ConfirmRisky => ServerConfirmationPolicy::ConfirmRisky,
                };

                // Start conversation without initial_message so we can
                // connect the WebSocket first and receive the user
                // MessageEvent (which carries activated_skills).
                let request = StartConversationRequest {
                    agent: AgentConfig::with_default_tools(llm_config.clone()),
                    workspace: LocalWorkspace::new(workspace_dir),
                    initial_message: None,
                    conversation_id: None,
                    confirmation_policy: Some(server_policy),
                    security_analyzer: Some(SecurityAnalyzer::LLMSecurityAnalyzer),
                };

                match client.start_conversation(request).await {
                    Ok(info) => {
                        state.conversation_id = Some(info.id);
                        state.conversation_title = info.title;
                        info!("Started conversation: {}", info.id);

                        // Connect WebSocket BEFORE sending the message
                        let ws_url = client.conversation_websocket_url(info.id);
                        match EventStream::connect(&ws_url).await {
                            Ok(stream) => {
                                *event_stream = Some(stream);
                                info!("Connected to WebSocket");
                            }
                            Err(e) => {
                                error!("Failed to connect WebSocket: {}", e);
                                state.notify(Notification::error(
                                    "WebSocket Error",
                                    "Failed to connect for real-time updates",
                                ));
                            }
                        }

                        // Now send the message so the WebSocket receives
                        // the echoed user event with activated_skills.
                        state.start_timer();
                        state.randomize_spinner();
                        state.execution_status = ExecutionStatus::Running;

                        if let Err(e) = client.send_message(info.id, &message, true).await {
                            error!("Failed to send initial message: {}", e);
                            state.add_message(DisplayMessage::error(format!(
                                "Failed to send message: {}",
                                e
                            )));
                            state.execution_status = ExecutionStatus::Idle;
                            return Ok(false);
                        }
                    }
                    Err(e) => {
                        error!("Failed to start conversation: {}", e);
                        state.add_message(DisplayMessage::error(format!(
                            "Failed to start conversation: {}",
                            e
                        )));
                        return Ok(false);
                    }
                }
            } else {
                // Existing conversation - send message with run=true
                let conv_id = state.conversation_id.unwrap();

                // Send message with run=true to start processing
                state.start_timer();
                state.randomize_spinner();
                state.execution_status = ExecutionStatus::Running;
                let mut send_result = client.send_message(conv_id, &message, true).await;

                // If the send failed (server lost conversation, transient error),
                // re-load the conversation on the server and retry once.
                if send_result.is_err() {
                    info!("Send failed, re-loading conversation on server and retrying...");
                    let workspace_dir = args.workspace.clone().unwrap_or_else(|| {
                        std::env::current_dir()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| ".".to_string())
                    });
                    let server_policy = match state.confirmation_policy {
                        ConfirmationPolicy::NeverConfirm => ServerConfirmationPolicy::NeverConfirm,
                        ConfirmationPolicy::AlwaysConfirm => {
                            ServerConfirmationPolicy::AlwaysConfirm
                        }
                        ConfirmationPolicy::ConfirmRisky => ServerConfirmationPolicy::ConfirmRisky,
                    };
                    let request = StartConversationRequest {
                        agent: AgentConfig::with_default_tools(llm_config.clone()),
                        workspace: LocalWorkspace::new(workspace_dir),
                        initial_message: None,
                        conversation_id: Some(conv_id),
                        confirmation_policy: Some(server_policy),
                        security_analyzer: Some(SecurityAnalyzer::LLMSecurityAnalyzer),
                    };
                    let _ = client.start_conversation(request).await;
                    // Retry the send
                    send_result = client.send_message(conv_id, &message, true).await;
                }

                if let Err(e) = send_result {
                    error!("Failed to send message: {}", e);
                    state.add_message(DisplayMessage::error(format!("Failed to send: {}", e)));
                    state.execution_status = ExecutionStatus::Idle;
                    return Ok(false);
                }
            }
        }

        AppCommand::AskAgent(question) => {
            if let Some(conv_id) = state.conversation_id {
                state.add_message(DisplayMessage::btw(&question, "Asking agent..."));
                if let Some(tx) = state.btw_sender.clone() {
                    let client = client.clone();
                    let q = question.clone();
                    tokio::spawn(async move {
                        let result = match client.ask_agent(conv_id, &q).await {
                            Ok(response) => Ok(response),
                            Err(e) => Err(format!("{}", e)),
                        };
                        let _ = tx.send((q, result));
                    });
                }
            }
        }

        AppCommand::RunBashCommand(cmd) => {
            // Run bash command locally and display as terminal message
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
            {
                Ok(output) => {
                    let mut result = String::new();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    if !stdout.is_empty() {
                        result.push_str(stdout.trim_end());
                    }
                    if !stderr.is_empty() {
                        if !result.is_empty() {
                            result.push('\n');
                        }
                        result.push_str(stderr.trim_end());
                    }
                    if !output.status.success() {
                        if !result.is_empty() {
                            result.push('\n');
                        }
                        result.push_str(&format!(
                            "Exit code: {}",
                            output.status.code().unwrap_or(-1)
                        ));
                    }
                    if result.is_empty() {
                        result.push_str("(no output)");
                    }
                    state.add_message(DisplayMessage::terminal(&cmd, result));
                }
                Err(e) => {
                    state.add_message(DisplayMessage::terminal(
                        &cmd,
                        format!("Failed to run command: {}", e),
                    ));
                }
            }
        }

        AppCommand::NewConversation => {
            *event_stream = None;
            state.reset_conversation();
            state.notify(Notification::info("New Conversation", "Starting fresh"));
        }

        AppCommand::ResumeConversation(conv_id) => {
            *event_stream = None;
            state.reset_conversation();

            let connected =
                resume_conversation(state, client, event_stream, conv_id, llm_config, args).await;
            let short_id = &conv_id.to_string()[..8.min(conv_id.to_string().len())];
            if connected {
                state.notify(Notification::info(
                    "Resumed",
                    format!("Conversation {}", short_id),
                ));
            } else {
                // History is still displayed from the replay — just stay idle.
                // The next message the user sends will re-attach the server to
                // this conversation (the files exist on disk).
                state.notify(Notification::info(
                    "Resumed (offline)",
                    format!("Conversation {} — send a message to reconnect", short_id),
                ));
            }
        }

        AppCommand::Pause => {
            if let Some(conv_id) = state.conversation_id {
                if let Err(e) = client.pause_conversation(conv_id).await {
                    error!("Failed to pause: {}", e);
                    state.notify(Notification::error("Pause Failed", e.to_string()));
                } else {
                    state.execution_status = ExecutionStatus::Paused;
                    state.notify(Notification::info("Paused", "Conversation paused"));
                }
            }
        }

        AppCommand::ConfirmYes => {
            // Accept current action — advance to next, or send batch accept when all reviewed
            if !state.pending_actions.is_empty() {
                // Mark current action as accepted in the message list
                let current = &state.pending_actions[0];
                for msg in state.messages.iter_mut() {
                    if msg.role == crate::state::MessageRole::Action {
                        if let Some(ref msg_id) = msg.id {
                            if msg_id == &current.tool_call_id {
                                msg.accepted = true;
                                break;
                            }
                        }
                    }
                }
                state.pending_actions.remove(0);

                if state.pending_actions.is_empty() {
                    // All actions reviewed — send batch accept to server
                    if let Some(conv_id) = state.conversation_id {
                        info!("All actions accepted, sending batch accept");
                        if let Err(e) = client.accept_pending_actions(conv_id).await {
                            error!("Failed to accept actions: {}", e);
                            state.notify(Notification::error("Accept Failed", e.to_string()));
                        }
                    }
                    state.input_mode = InputMode::Normal;
                    state.randomize_spinner();
                    state.execution_status = ExecutionStatus::Running;
                }
                // Otherwise, stay in Confirmation mode — next action shown automatically
            }
        }

        AppCommand::ConfirmNo => {
            // Reject: tell the server to reject the pending actions
            if let Some(conv_id) = state.conversation_id {
                info!("User rejected pending actions - calling reject API");
                if let Err(e) = client
                    .reject_pending_actions(conv_id, Some("User rejected the action"))
                    .await
                {
                    // If reject API fails, try to just run to clear the state
                    warn!("Reject API failed ({}), trying to continue anyway", e);
                }
                state.add_message(DisplayMessage::system("Action rejected"));
                state.clear_pending_actions();
                // Set to idle - the server should handle the rejection
                state.execution_status = ExecutionStatus::Idle;
            }
        }

        AppCommand::ConfirmAll => {
            // Always accept: change policy and accept current pending actions
            state.confirmation_policy = ConfirmationPolicy::NeverConfirm;
            state.notify(Notification::info(
                "Policy Changed",
                "Auto-approving all future actions",
            ));

            // Accept current pending actions
            if let Some(conv_id) = state.conversation_id {
                if let Err(e) = client.accept_pending_actions(conv_id).await {
                    warn!("Failed to accept actions: {}", e);
                }
                state.clear_pending_actions();
                state.randomize_spinner();
                state.execution_status = ExecutionStatus::Running;
            }
        }

        AppCommand::ConfirmDefer => {
            state.clear_pending_actions();
            state.execution_status = ExecutionStatus::Paused;
            state.notify(Notification::info(
                "Deferred",
                "Actions deferred, agent paused",
            ));
        }

        AppCommand::RenameConversation(new_name) => {
            state.conversation_title = Some(new_name.clone());

            // Persist to meta.json so /resume shows the new title
            if let Some(conv_id) = state.conversation_id {
                let id_str = conv_id.as_simple().to_string();
                if let Err(e) = crate::state::conversations::update_title(&id_str, &new_name) {
                    warn!("Failed to persist title: {}", e);
                }
            }

            state.notify(Notification::info(
                "Renamed",
                format!("Conversation renamed to \"{}\"", new_name),
            ));
        }

        AppCommand::SetPolicy(policy) => {
            state.confirmation_policy = policy;
            state.notify(Notification::info(
                "Policy Changed",
                format!("Confirmation policy: {}", policy),
            ));
        }

        AppCommand::LoadSkills => {
            state.skills_modal.loading = true;
            state.skills_modal.error = None;
            match client.list_skills(build_skills_request(state)).await {
                Ok(resp) => {
                    info!(
                        "Loaded {} skills from {} sources",
                        resp.skills.len(),
                        resp.sources.len()
                    );
                    state.skills_modal.skills = resp.skills;
                    state.skills_modal.loading = false;
                    state.skills_modal.selected = 0;
                }
                Err(e) => {
                    error!("Failed to load skills: {}", e);
                    state.skills_modal.error = Some(format!("{}", e));
                    state.skills_modal.loading = false;
                }
            }
        }

        AppCommand::SyncSkills => {
            state.skills_modal.loading = true;
            state.skills_modal.error = None;
            match client.sync_skills().await {
                Ok(()) => {
                    state.notify(Notification::info("Skills", "Public marketplace synced"));
                    // Reload skills after sync
                    if let Ok(resp) = client.list_skills(build_skills_request(state)).await {
                        state.skills_modal.skills = resp.skills;
                    }
                    state.skills_modal.loading = false;
                }
                Err(e) => {
                    error!("Failed to sync skills: {}", e);
                    state.skills_modal.error = Some(format!("{}", e));
                    state.skills_modal.loading = false;
                }
            }
        }

        AppCommand::LoadTools => {
            // Show the tools configured on the agent, not the server's full list.
            // The agent always includes FinishTool and ThinkTool as defaults.
            let mut tools: Vec<String> = AgentConfig::with_default_tools(llm_config.clone())
                .tools
                .unwrap_or_default()
                .iter()
                .map(|t| t.name.clone())
                .collect();
            tools.push("finish".to_string());
            tools.push("think".to_string());
            debug!("Agent tools: {:?}", tools);
            state.tools_list = tools;
        }

        AppCommand::ForceQuit => {
            state.should_exit = true;
            return Ok(true);
        }

        AppCommand::CancelQuit => {
            state.exit_confirmation_pending = false;
        }
    }

    Ok(false)
}

/// Build a default `SkillsRequest` from the current state.
///
/// Loads user, project, and public skills — org loading is disabled. The
/// project directory is the current workspace path.
fn build_skills_request(state: &AppState) -> crate::client::SkillsRequest {
    crate::client::SkillsRequest {
        load_public: true,
        load_user: true,
        load_project: true,
        load_org: false,
        project_dir: Some(state.workspace_path.clone()),
    }
}

/// Resume an existing conversation: replay stored events, connect the
/// WebSocket, and fetch title/metrics.
///
/// This is shared between the `--resume` CLI flag (at startup in `main.rs`)
/// and the `ResumeConversation` command triggered from the resume modal.
///
/// Callers are responsible for:
/// - calling `state.reset_conversation()` beforehand if needed
/// - showing any user-facing notification on success/failure
///
/// Returns `true` if the WebSocket connected successfully, `false` otherwise.
pub async fn resume_conversation(
    state: &mut AppState,
    client: &AgentServerClient,
    event_stream: &mut Option<EventStream>,
    conv_id: uuid::Uuid,
    llm_config: &LLMConfig,
    args: &crate::cli::Args,
) -> bool {
    state.conversation_id = Some(conv_id);

    // Replay stored events to rebuild message history
    let conv_id_str = conv_id.as_simple().to_string();
    let events = crate::state::conversations::load_events(&conv_id_str);
    info!(
        "Replaying {} events from conversation {}",
        events.len(),
        conv_id_str
    );
    state.replaying = true;
    for event in events {
        state.process_event(event);
    }
    state.replaying = false;
    state.execution_status = ExecutionStatus::Idle;
    state.pending_actions.clear();
    state.input_mode = crate::state::InputMode::Normal;

    // Tell the agent server to load this conversation from disk.
    // POST to /api/conversations with conversation_id set and no initial_message.
    let workspace_dir = args.workspace.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });
    let server_policy = match state.confirmation_policy {
        ConfirmationPolicy::NeverConfirm => ServerConfirmationPolicy::NeverConfirm,
        ConfirmationPolicy::AlwaysConfirm => ServerConfirmationPolicy::AlwaysConfirm,
        ConfirmationPolicy::ConfirmRisky => ServerConfirmationPolicy::ConfirmRisky,
    };
    let request = StartConversationRequest {
        agent: AgentConfig::with_default_tools(llm_config.clone()),
        workspace: LocalWorkspace::new(workspace_dir),
        initial_message: None,
        conversation_id: Some(conv_id),
        confirmation_policy: Some(server_policy),
        security_analyzer: Some(SecurityAnalyzer::LLMSecurityAnalyzer),
    };
    match client.start_conversation(request).await {
        Ok(info) => {
            info!(
                "Server loaded conversation {} (status: {:?})",
                info.id, info.execution_status
            );
        }
        Err(e) => {
            warn!("Failed to load conversation on server: {}", e);
        }
    }

    // Connect WebSocket
    if let Some(stream) = crate::client::try_connect_event_stream(client, conv_id, "resume").await {
        *event_stream = Some(stream);
        state.connected = true;

        // Fetch title/metrics
        if let Ok(full_state) = client.get_conversation_state(conv_id).await {
            if let Some(title) = full_state.get("title").and_then(|v| v.as_str()) {
                state.conversation_title = Some(title.to_string());
            }
            if let Some(stats) = full_state.get("stats") {
                state.parse_metrics(stats);
            }
        }
        true
    } else {
        false
    }
}
