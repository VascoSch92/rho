//! Headless runner — executes a task without the TUI.
//!
//! Reuses the existing `client` module (HTTP + WebSocket) and streams events
//! to stdout in either human-readable text or JSON Lines format.

use std::time::Duration;

use anyhow::{bail, Result};
use tracing::{error, info};

use crate::cli::HeadlessArgs;
use crate::client::{
    AgentConfig, AgentServerClient, EventStream, LLMConfig, LocalWorkspace, SecurityAnalyzer,
    ServerConfirmationPolicy, StartConversationRequest,
};
use crate::events::Event;

/// Exit codes for headless mode.
pub mod exit {
    pub const SUCCESS: i32 = 0;
    pub const TASK_ERROR: i32 = 1;
    pub const TIMEOUT: i32 = 2;
    pub const CONNECTION_ERROR: i32 = 3;
}

/// Run a task headlessly: start conversation, stream events, print output, exit.
pub async fn run_headless(args: &HeadlessArgs) -> Result<i32> {
    // Resolve the task text
    let task = match (&args.task, &args.file) {
        (Some(t), _) => t.clone(),
        (_, Some(path)) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read task file {}: {}", path.display(), e))?,
        _ => bail!("Either --task or --file is required"),
    };

    if task.trim().is_empty() {
        bail!("Task cannot be empty");
    }

    // Resolve LLM settings from config
    let rho_config = crate::config::RhoConfig::load();
    let config_llm = rho_config.llm;

    // Apply env var overrides only with --override-with-envs
    let env_model = if args.override_with_envs {
        std::env::var("LLM_MODEL").ok()
    } else {
        None
    };
    let env_api_key = if args.override_with_envs {
        std::env::var("LLM_API_KEY").ok()
    } else {
        None
    };
    let env_base_url = if args.override_with_envs {
        std::env::var("LLM_BASE_URL").ok()
    } else {
        None
    };

    let effective_model = env_model
        .or(config_llm.model)
        .unwrap_or_else(|| "anthropic/claude-sonnet-4-5-20250929".to_string());
    let effective_base_url = env_base_url.or(config_llm.base_url);
    let llm_api_key = env_api_key
        .or(config_llm.api_key.filter(|k| !k.is_empty()))
        .ok_or_else(|| anyhow::anyhow!("LLM_API_KEY is required. Set via --override-with-envs + LLM_API_KEY env, or /settings."))?;

    // Persist env overrides
    if args.override_with_envs {
        let _ = crate::config::save_llm(
            &effective_model,
            &llm_api_key,
            effective_base_url.as_deref(),
        );
    }

    // Build LLM config
    let llm_config = {
        let config = LLMConfig::new(&effective_model, &llm_api_key);
        if let Some(ref base_url) = effective_base_url {
            config.with_base_url(base_url)
        } else {
            config
        }
    };

    // Create client and check server health
    let client = AgentServerClient::new(&args.server, args.session_api_key.clone());

    if !args.json {
        eprintln!("Connecting to {}...", args.server);
    }

    if let Err(e) = client.health().await {
        if args.json {
            println!(
                "{}",
                serde_json::json!({"type": "error", "message": format!("Server not reachable: {}", e)})
            );
        } else {
            eprintln!("Error: Server not reachable at {}: {}", args.server, e);
        }
        return Ok(exit::CONNECTION_ERROR);
    }

    // Build workspace
    let workspace_dir = args.workspace.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    // Confirmation policy
    let server_policy = if args.auto_approve {
        ServerConfirmationPolicy::NeverConfirm
    } else {
        ServerConfirmationPolicy::AlwaysConfirm
    };

    // Start conversation
    let request = StartConversationRequest {
        agent: AgentConfig::with_default_tools(llm_config),
        workspace: LocalWorkspace::new(workspace_dir),
        initial_message: Some(crate::client::SendMessageRequest::user(&task).with_run()),
        conversation_id: None,
        confirmation_policy: Some(server_policy),
        security_analyzer: Some(SecurityAnalyzer::LLMSecurityAnalyzer),
    };

    let conv_info = match client.start_conversation(request).await {
        Ok(info) => {
            info!("Started conversation: {}", info.id);
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({"type": "started", "conversation_id": info.id.to_string(), "task": task})
                );
            } else {
                eprintln!("Conversation: {}", info.id);
                eprintln!("Task: {}", task);
                eprintln!("---");
            }
            info
        }
        Err(e) => {
            error!("Failed to start conversation: {}", e);
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({"type": "error", "message": format!("Failed to start: {}", e)})
                );
            } else {
                eprintln!("Error: Failed to start conversation: {}", e);
            }
            return Ok(exit::CONNECTION_ERROR);
        }
    };

    // Connect WebSocket
    let ws_url = client.conversation_websocket_url(conv_info.id);
    let mut event_stream = match EventStream::connect(&ws_url).await {
        Ok(s) => s,
        Err(e) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({"type": "error", "message": format!("WebSocket failed: {}", e)})
                );
            } else {
                eprintln!("Error: WebSocket connection failed: {}", e);
            }
            return Ok(exit::CONNECTION_ERROR);
        }
    };

    // Event loop with optional timeout
    let timeout_duration = if args.timeout > 0 {
        Some(Duration::from_secs(args.timeout))
    } else {
        None
    };
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(100);
    let mut exit_code = exit::SUCCESS;
    let mut finished = false;

    while !finished {
        // Check timeout
        if let Some(timeout) = timeout_duration {
            if start.elapsed() > timeout {
                if args.json {
                    println!(
                        "{}",
                        serde_json::json!({"type": "timeout", "seconds": args.timeout})
                    );
                } else {
                    eprintln!("Timeout after {}s", args.timeout);
                }
                exit_code = exit::TIMEOUT;
                break;
            }
        }

        // Poll for events
        tokio::time::sleep(poll_interval).await;

        while let Some(event) = event_stream.try_recv() {
            match &event {
                Event::MessageEvent(msg) => {
                    if let Some(text) = msg.get_text() {
                        let role = msg
                            .llm_message
                            .as_ref()
                            .map(|m| m.role.as_str())
                            .unwrap_or("unknown");
                        if args.json {
                            println!(
                                "{}",
                                serde_json::json!({"type": "message", "role": role, "text": text})
                            );
                        } else if role == "assistant" {
                            println!("{}", text);
                        }
                    }
                }
                Event::ActionEvent(action) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "type": "action",
                                "tool_name": action.tool_name,
                                "tool_call_id": action.tool_call_id,
                                "summary": action.summary,
                                "security_risk": format!("{:?}", action.security_risk),
                            })
                        );
                    } else {
                        let summary = action.summary.as_deref().unwrap_or(&action.tool_name);
                        eprintln!("[action] {} — {}", action.tool_name, summary);
                    }

                    // Auto-approve if requested
                    if args.auto_approve {
                        let _ = client.accept_pending_actions(conv_info.id).await;
                    }
                }
                Event::ObservationEvent(obs) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "type": "observation",
                                "tool_name": obs.tool_name,
                                "tool_call_id": obs.tool_call_id,
                            })
                        );
                    }
                }
                Event::AgentErrorEvent(err) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::json!({"type": "error", "message": err.error, "detail": err.detail})
                        );
                    } else {
                        eprintln!("[error] {}", err.error);
                        if let Some(ref detail) = err.detail {
                            eprintln!("  {}", detail);
                        }
                    }
                    exit_code = exit::TASK_ERROR;
                }
                Event::ConversationStateUpdateEvent(state_evt) => {
                    // Check for execution_status changes
                    if state_evt.key == "execution_status" {
                        if let Some(status_str) = state_evt.value.as_str() {
                            match status_str {
                                "finished" | "idle" => {
                                    if args.json {
                                        println!(
                                            "{}",
                                            serde_json::json!({"type": "finished", "status": status_str})
                                        );
                                    }
                                    finished = true;
                                }
                                "error" => {
                                    if args.json {
                                        println!(
                                            "{}",
                                            serde_json::json!({"type": "finished", "status": "error"})
                                        );
                                    } else {
                                        eprintln!("[status] Error");
                                    }
                                    exit_code = exit::TASK_ERROR;
                                    finished = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    // Check for full_state with execution_status
                    if state_evt.key == "full_state" {
                        if let Some(status_str) = state_evt
                            .value
                            .get("execution_status")
                            .and_then(|v| v.as_str())
                        {
                            match status_str {
                                "finished" | "idle" => {
                                    if args.json {
                                        println!(
                                            "{}",
                                            serde_json::json!({"type": "finished", "status": status_str})
                                        );
                                    }
                                    finished = true;
                                }
                                "error" => {
                                    exit_code = exit::TASK_ERROR;
                                    finished = true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Event::PauseEvent(_) => {
                    if args.json {
                        println!("{}", serde_json::json!({"type": "paused"}));
                    } else {
                        eprintln!("[status] Paused");
                    }
                    finished = true;
                }
                _ => {}
            }
        }

        // Check if WebSocket disconnected
        if !event_stream.is_connected() && !finished {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({"type": "error", "message": "WebSocket disconnected"})
                );
            } else {
                eprintln!("Error: WebSocket disconnected");
            }
            exit_code = exit::CONNECTION_ERROR;
            break;
        }
    }

    if !args.json {
        eprintln!("---");
        eprintln!(
            "Done (exit code: {}, elapsed: {:.1}s)",
            exit_code,
            start.elapsed().as_secs_f64()
        );
    }

    Ok(exit_code)
}
