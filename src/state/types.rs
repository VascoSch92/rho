//! Core types: ConfirmationPolicy, InputMode, DisplayMessage, PendingAction, Notification.

use std::time::{Duration, Instant};

use crate::events::{ActionEvent, SecurityRisk};

/// Confirmation policy for actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum ConfirmationPolicy {
    /// Always confirm actions before execution
    #[default]
    #[value(alias = "always")]
    AlwaysConfirm,
    /// Never confirm, auto-approve all actions
    #[value(alias = "never")]
    NeverConfirm,
    /// Confirm only risky actions (MEDIUM and above)
    #[value(alias = "risky")]
    ConfirmRisky,
}

impl std::fmt::Display for ConfirmationPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfirmationPolicy::AlwaysConfirm => write!(f, "   Always Confirm"),
            ConfirmationPolicy::NeverConfirm => write!(f, "    Auto-Approve"),
            ConfirmationPolicy::ConfirmRisky => write!(f, "    Confirm Risky"),
        }
    }
}

/// Input mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal mode - user can type messages
    #[default]
    Normal,
    /// Confirmation mode - waiting for user to confirm actions
    Confirmation,
}

/// A display message in the conversation
#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub id: Option<String>,
    pub role: MessageRole,
    pub content: String,
    pub collapsed: bool,
    pub tool_name: Option<String>,
    pub security_risk: Option<SecurityRisk>,
    pub accepted: bool,
    pub thought: Option<String>,
    /// Skills activated by this message (populated for user messages).
    pub activated_skills: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Action,
    Error,
    Terminal,
    Btw,
}

impl DisplayMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: None,
            role: MessageRole::User,
            content: content.into(),
            collapsed: false,
            tool_name: None,
            security_risk: None,
            accepted: false,
            thought: None,
            activated_skills: Vec::new(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: None,
            role: MessageRole::Assistant,
            content: content.into(),
            collapsed: false,
            tool_name: None,
            security_risk: None,
            accepted: false,
            thought: None,
            activated_skills: Vec::new(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: None,
            role: MessageRole::System,
            content: content.into(),
            collapsed: true,
            tool_name: None,
            security_risk: None,
            accepted: false,
            thought: None,
            activated_skills: Vec::new(),
        }
    }

    pub fn action(event: &ActionEvent) -> Self {
        let summary = event
            .summary
            .clone()
            .unwrap_or_else(|| event.tool_name.clone());

        let args_display = event
            .tool_call
            .as_ref()
            .and_then(|tc| tc.arguments.as_deref())
            .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
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

        let content = format!("{}\n{}", args_display, summary);

        let thought = event
            .thought
            .clone()
            .or_else(|| event.reasoning_content.clone());

        Self {
            id: Some(event.tool_call_id.clone()),
            role: MessageRole::Action,
            content,
            collapsed: true,
            tool_name: Some(event.tool_name.clone()),
            security_risk: Some(event.effective_risk()),
            accepted: false,
            thought,
            activated_skills: Vec::new(),
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            id: None,
            role: MessageRole::Error,
            content: content.into(),
            collapsed: false,
            tool_name: None,
            security_risk: None,
            accepted: false,
            thought: None,
            activated_skills: Vec::new(),
        }
    }

    pub fn terminal(command: &str, output: impl Into<String>) -> Self {
        Self {
            id: None,
            role: MessageRole::Terminal,
            content: format!("$ {}\n{}", command, output.into()),
            collapsed: false,
            tool_name: None,
            security_risk: None,
            accepted: false,
            thought: None,
            activated_skills: Vec::new(),
        }
    }

    pub fn btw(question: &str, answer: impl Into<String>) -> Self {
        Self {
            id: None,
            role: MessageRole::Btw,
            content: format!("{}\n{}", question, answer.into()),
            collapsed: false,
            tool_name: None,
            security_risk: None,
            accepted: false,
            thought: None,
            activated_skills: Vec::new(),
        }
    }
}

/// A task from the task_tracker tool.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TaskItem {
    pub title: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub status: String,
}

/// Pending action awaiting confirmation
#[derive(Debug, Clone)]
pub struct PendingAction {
    pub tool_call_id: String,
    pub tool_name: String,
    pub args: String,
    pub summary: String,
    pub security_risk: SecurityRisk,
}

/// Notification to display temporarily
#[derive(Debug, Clone)]
pub struct Notification {
    pub title: String,
    pub message: String,
    pub severity: NotificationSeverity,
    pub created_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
}

impl Notification {
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            severity: NotificationSeverity::Info,
            created_at: Instant::now(),
        }
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            severity: NotificationSeverity::Warning,
            created_at: Instant::now(),
        }
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            severity: NotificationSeverity::Error,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self, duration: Duration) -> bool {
        self.created_at.elapsed() > duration
    }
}
