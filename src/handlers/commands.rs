//! Command execution — processes AppCommands into API calls and state changes.

use anyhow::Result;
use tracing::{error, info, warn};

use super::AppCommand;
use crate::cli::Args;
use crate::client::{
    AgentConfig, AgentServerClient, EventStream, ExecutionStatus, LLMConfig, LocalWorkspace,
    SendMessageRequest, ServerConfirmationPolicy, StartConversationRequest,
};
use crate::state::{AppState, ConfirmationPolicy, DisplayMessage, Notification};

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
            // Add user message to display
            state.add_message(DisplayMessage::user(&message));

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

                let request = StartConversationRequest {
                    agent: AgentConfig::with_default_tools(llm_config.clone()),
                    workspace: LocalWorkspace::new(workspace_dir),
                    initial_message: Some(SendMessageRequest::user(&message).with_run()),
                    conversation_id: None,
                    confirmation_policy: Some(server_policy),
                };

                match client.start_conversation(request).await {
                    Ok(info) => {
                        state.conversation_id = Some(info.id);
                        state.conversation_title = info.title;
                        info!("Started conversation: {}", info.id);

                        // Connect to WebSocket for events
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

                        // Conversation starts running automatically with initial_message
                        state.start_timer();
                        state.randomize_spinner();
                        state.execution_status = ExecutionStatus::Running;
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
                if let Err(e) = client.send_message(conv_id, &message, true).await {
                    error!("Failed to send message: {}", e);
                    state.add_message(DisplayMessage::error(format!("Failed to send: {}", e)));
                    state.execution_status = ExecutionStatus::Idle;
                    return Ok(false);
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
            // Disconnect existing stream
            *event_stream = None;
            state.conversation_id = None;
            state.messages.clear();
            state.pending_actions.clear();
            state.execution_status = ExecutionStatus::Idle;
            state.conversation_title = None;
            state.elapsed_seconds = 0;
            state.elapsed_base = 0;
            state.start_time = None;
            state.notify(Notification::info("New Conversation", "Starting fresh"));
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
            // Accept: tell the server to accept the pending actions
            if let Some(conv_id) = state.conversation_id {
                info!("User accepted pending actions");
                if let Err(e) = client.accept_pending_actions(conv_id).await {
                    error!("Failed to accept actions: {}", e);
                    state.notify(Notification::error("Accept Failed", e.to_string()));
                }
                state.clear_pending_actions();
                state.randomize_spinner();
                state.execution_status = ExecutionStatus::Running;
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

        AppCommand::SetPolicy(policy) => {
            state.confirmation_policy = policy;
            state.notify(Notification::info(
                "Policy Changed",
                format!("Confirmation policy: {}", policy),
            ));
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
