//! Event processing — converts server events into state changes.

use crate::client::ExecutionStatus;
use crate::events::{ActionEvent, Event, SecurityRisk};

use super::types::{ConfirmationPolicy, DisplayMessage, InputMode, MessageRole, PendingAction};
use super::AppState;

impl AppState {
    /// Process an incoming server event into state changes.
    ///
    /// Dispatches by event kind:
    /// - `MessageEvent` → adds user/assistant/system messages (skips user msgs unless replaying)
    /// - `ActionEvent` → adds tool call display, may trigger confirmation dialog
    /// - `ObservationEvent` → marks the matching action as completed (checkmark)
    /// - `AgentErrorEvent` → displays error with optional detail
    /// - `ConversationStateUpdateEvent` → updates execution status, title, metrics
    /// - `PauseEvent` / `SystemPromptEvent` / `Condensation` → system messages
    pub fn process_event(&mut self, event: Event) {
        match event {
            Event::MessageEvent(msg) => {
                // Show activated skills for user messages (even during live mode
                // where the user message itself is already displayed locally).
                if msg.base.source.as_deref() == Some("user") {
                    tracing::debug!(
                        "User message event: activated_skills={:?}",
                        msg.activated_skills
                    );
                    if !msg.activated_skills.is_empty() {
                        let skills = msg.activated_skills.join(", ");
                        self.add_message(DisplayMessage::system(format!(
                            "Activated skills: {}",
                            skills
                        )));
                    }
                }

                // Skip user messages during live operation (already displayed locally).
                // During replay, include them to rebuild history.
                if !self.replaying && msg.base.source.as_deref() == Some("user") {
                    return;
                }

                if let Some(text) = msg.get_text() {
                    if let Some(ref llm_msg) = msg.llm_message {
                        tracing::debug!("Agent message [{}]: {}", llm_msg.role, text);
                        let display_msg = match llm_msg.role.as_str() {
                            "user" => DisplayMessage::user(text),
                            "assistant" => DisplayMessage::assistant(text),
                            _ => DisplayMessage::system(text),
                        };
                        self.add_message(display_msg);
                    }
                }
            }
            Event::ActionEvent(action) => {
                tracing::debug!(
                    "Agent action: tool={} summary={:?} thought={:?}",
                    action.tool_name,
                    action.summary,
                    action.thought
                );

                // Finish tool: display the message as a normal assistant response
                if action.tool_name == "finish" {
                    if let Some(message) = action
                        .action
                        .get("message")
                        .and_then(|v| v.as_str())
                    {
                        self.add_message(DisplayMessage::assistant(message));
                    }
                    return;
                }

                let msg = DisplayMessage::action(&action);
                self.add_message(msg);

                // Check if confirmation is needed (skip during replay)
                if !self.replaying && self.needs_confirmation(&action) {
                    self.request_confirmation(&action);
                }
            }
            Event::ObservationEvent(obs) => {
                tracing::debug!(
                    "Agent observation: tool={} result={}",
                    obs.tool_name,
                    obs.observation
                );
                // Mark the corresponding action as accepted (shows checkmark).
                for msg in self.messages.iter_mut() {
                    if msg.role == MessageRole::Action {
                        if let Some(ref msg_id) = msg.id {
                            if msg_id == &obs.tool_call_id {
                                msg.accepted = true;
                                break;
                            }
                        }
                    }
                }
            }
            Event::AgentErrorEvent(err) => {
                let error_text = if let Some(ref detail) = err.detail {
                    format!("{}\n{}", err.error, detail)
                } else {
                    err.error.clone()
                };
                self.add_message(DisplayMessage::error(&error_text));
            }
            Event::ConversationStateUpdateEvent(update) => {
                tracing::debug!("State update key='{}' value={}", update.key, update.value);
                match update.key.as_str() {
                    "execution_status" => {
                        if let Ok(status) = serde_json::from_value::<ExecutionStatus>(update.value)
                        {
                            let was_running = self.execution_status == ExecutionStatus::Running;
                            self.execution_status = status;
                            if was_running && status == ExecutionStatus::Finished {
                                self.needs_stats_refresh = true;
                            }
                            if status == ExecutionStatus::Error {
                                let has_recent_error = self
                                    .messages
                                    .iter()
                                    .rev()
                                    .take(3)
                                    .any(|m| m.role == MessageRole::Error);
                                if !has_recent_error {
                                    self.add_message(DisplayMessage::error(
                                        "Agent encountered an error. Check logs for details.",
                                    ));
                                }
                            }
                        }
                    }
                    "title" => {
                        if let Some(title) = update.value.as_str() {
                            self.conversation_title = Some(title.to_string());
                        }
                    }
                    "metrics" | "stats" => {
                        self.metrics.parse(&update.value);
                    }
                    "full_state" => {
                        if let Some(stats) = update.value.get("stats") {
                            self.metrics.parse(stats);
                        }
                    }
                    _ => {}
                }
            }
            Event::PauseEvent(_) => {
                self.add_message(DisplayMessage::system("Conversation paused"));
                self.execution_status = ExecutionStatus::Paused;
            }
            Event::UserRejectObservation(reject) => {
                self.add_message(DisplayMessage::system(format!(
                    "Action rejected: {}",
                    reject.rejection_reason
                )));
            }
            Event::SystemPromptEvent(prompt) => {
                if let Some(tools) = prompt.tools {
                    self.add_message(DisplayMessage::system(format!(
                        "Loaded {} tools",
                        tools.len()
                    )));
                }
            }
            Event::Condensation(cond) => {
                if let Some(summary) = cond.summary {
                    self.add_message(DisplayMessage::system(format!(
                        "History condensed: {}",
                        summary
                    )));
                }
            }
            Event::TokenEvent(_) => {}
            Event::Unknown => {}
        }
    }

    /// Check if an action needs confirmation based on policy
    fn needs_confirmation(&self, action: &ActionEvent) -> bool {
        match self.confirmation_policy {
            ConfirmationPolicy::NeverConfirm => false,
            ConfirmationPolicy::AlwaysConfirm => true,
            ConfirmationPolicy::ConfirmRisky => {
                let risk = action.effective_risk();
                matches!(risk, SecurityRisk::Medium | SecurityRisk::High)
            }
        }
    }

    /// Build a PendingAction and enter confirmation mode.
    fn request_confirmation(&mut self, action: &ActionEvent) {
        tracing::info!(
            "Action requires confirmation: {} (risk: {:?})",
            action.tool_name,
            action.security_risk
        );
        let args = action
            .tool_call
            .as_ref()
            .and_then(|tc| tc.arguments.as_deref())
            .and_then(|a| serde_json::from_str::<serde_json::Value>(a).ok())
            .and_then(|val| {
                val.as_object().map(|obj| {
                    obj.iter()
                        .filter(|(k, _)| *k != "security_risk" && *k != "summary")
                        .map(|(k, v)| {
                            let v_str = match v {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            format!("{}: {}", k, v_str)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
            })
            .unwrap_or_default();

        self.pending_actions.push(PendingAction {
            tool_call_id: action.tool_call_id.clone(),
            tool_name: action.tool_name.clone(),
            args,
            summary: action
                .summary
                .clone()
                .unwrap_or_else(|| action.tool_name.clone()),
            security_risk: action.effective_risk(),
        });
        self.input_mode = InputMode::Confirmation;
        self.execution_status = ExecutionStatus::WaitingForConfirmation;
    }
}
