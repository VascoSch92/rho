//! Application state management.
//!
//! Manages the TUI state including conversation data, UI mode, and user input.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::client::ExecutionStatus;
use crate::config::theme::{SpinnerStyle, Theme, ThemeName};
use crate::events::{ActionEvent, Event, SecurityRisk};

/// Maximum number of messages to keep in history for display
const MAX_DISPLAY_MESSAGES: usize = 1000;

/// Confirmation policy for actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmationPolicy {
    /// Always confirm actions before execution
    #[default]
    AlwaysConfirm,
    /// Never confirm, auto-approve all actions
    NeverConfirm,
    /// Confirm only risky actions (MEDIUM and above)
    ConfirmRisky,
}

impl std::fmt::Display for ConfirmationPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfirmationPolicy::AlwaysConfirm => write!(f, "Always Confirm"),
            ConfirmationPolicy::NeverConfirm => write!(f, "Auto-Approve"),
            ConfirmationPolicy::ConfirmRisky => write!(f, "Confirm Risky"),
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
    pub id: Option<String>, // Event ID (UUID or ULID)
    pub role: MessageRole,
    pub content: String,
    pub collapsed: bool,
    pub tool_name: Option<String>,
    pub security_risk: Option<SecurityRisk>,
    pub accepted: bool, // Whether this action was accepted (shows checkmark)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Action,
    Error,
    Terminal,
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
        }
    }

    pub fn action(event: &ActionEvent) -> Self {
        let summary = event
            .summary
            .clone()
            .unwrap_or_else(|| event.tool_name.clone());

        // Format args as "key: value, key: value" (excluding security_risk and summary)
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

        // Content: "args_display\nsummary" — tool_name and risk are rendered separately by UI
        let content = format!("{}\n{}", args_display, summary);

        Self {
            // Store tool_call_id (not base.id) - this is what observations reference
            id: Some(event.tool_call_id.clone()),
            role: MessageRole::Action,
            content,
            collapsed: true,
            tool_name: Some(event.tool_name.clone()),
            security_risk: event.security_risk,
            accepted: false,
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
        }
    }
}

/// Pending action awaiting confirmation
#[derive(Debug, Clone)]
pub struct PendingAction {
    pub tool_call_id: String,
    pub tool_name: String,
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

/// CLI version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// LLM Provider
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LlmProvider {
    OpenHands,
    #[default]
    Anthropic,
    OpenAI,
    Mistral,
    Google,
    DeepSeek,
    Other(String),
}

impl LlmProvider {
    pub fn display_name(&self) -> &str {
        match self {
            LlmProvider::OpenHands => "OpenHands",
            LlmProvider::Anthropic => "Anthropic",
            LlmProvider::OpenAI => "OpenAI",
            LlmProvider::Mistral => "Mistral",
            LlmProvider::Google => "Google",
            LlmProvider::DeepSeek => "DeepSeek",
            LlmProvider::Other(s) => s,
        }
    }

    pub fn all() -> Vec<LlmProvider> {
        vec![
            LlmProvider::OpenHands,
            LlmProvider::Anthropic,
            LlmProvider::OpenAI,
            LlmProvider::Mistral,
            LlmProvider::Google,
            LlmProvider::DeepSeek,
        ]
    }

    pub fn models(&self) -> Vec<&'static str> {
        match self {
            LlmProvider::OpenHands => vec![
                "claude-sonnet-4-5-20250929",
                "claude-opus-4-6",
                "gpt-5.2",
                "gpt-5.1",
                "deepseek-chat",
            ],
            LlmProvider::Anthropic => vec![
                "claude-sonnet-4-5-20250929",
                "claude-opus-4-6",
                "claude-sonnet-4-6",
                "claude-3-5-sonnet-20241022",
                "claude-3-opus-20240229",
                "claude-3-haiku-20240307",
            ],
            LlmProvider::OpenAI => vec![
                "gpt-5.2",
                "gpt-5.1",
                "gpt-4o",
                "gpt-4o-mini",
                "o4-mini",
                "o3",
            ],
            LlmProvider::Mistral => vec![
                "devstral-medium-2512",
                "devstral-2512",
                "devstral-small-2507",
            ],
            LlmProvider::Google => vec!["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"],
            LlmProvider::DeepSeek => vec!["deepseek-chat", "deepseek-reasoner"],
            LlmProvider::Other(_) => vec![],
        }
    }
}

/// Main application state
pub struct AppState {
    // Connection state
    pub connected: bool,
    pub conversation_id: Option<Uuid>,
    pub execution_status: ExecutionStatus,

    // UI state
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub scroll_offset: usize,

    // Conversation state
    pub messages: VecDeque<DisplayMessage>,
    pub conversation_title: Option<String>,
    pub confirmation_policy: ConfirmationPolicy,

    // Pending confirmations
    pub pending_actions: Vec<PendingAction>,

    // Notifications
    pub notifications: Vec<Notification>,

    // Metrics
    pub elapsed_seconds: u64,
    pub start_time: Option<Instant>,
    pub total_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_cost: f64,
    pub context_window: u64, // Max context size for the model

    // Exit flag
    pub should_exit: bool,
    pub exit_confirmation_pending: bool,

    // Modals
    pub show_token_modal: bool,
    pub show_help_modal: bool,
    pub show_policy_modal: bool,
    pub show_settings_modal: bool,
    pub settings_field: usize, // 0=Provider, 1=Model, 2=API Key, 3=Base URL
    pub settings_editing: bool, // Whether currently editing a field
    pub settings_edit_buffer: String, // Buffer for editing text fields

    // LLM Settings
    pub llm_provider: LlmProvider,
    pub llm_model: String,
    pub llm_api_key: String,
    pub llm_base_url: Option<String>,

    // Command menu state
    pub show_command_menu: bool,
    pub command_menu_selected: usize,

    // Confirmation dialog state (arrow key navigation)
    pub confirmation_selected: usize,

    // Animation state
    pub spinner_tick: usize,
    pub spinner_style: SpinnerStyle,
    pub fun_fact_index: usize,

    // Workspace info
    pub workspace_path: String,

    // Flag to request stats refresh (set when execution finishes)
    pub needs_stats_refresh: bool,

    // Server startup state
    pub server_starting: bool,
    pub server_starting_tick: usize,

    // Theme
    pub theme: Theme,
    pub theme_name: ThemeName,
    pub show_theme_modal: bool,
    pub theme_selected: usize,
    pub theme_before_preview: Option<ThemeName>,
}

impl Default for AppState {
    fn default() -> Self {
        let provider = LlmProvider::Anthropic;
        let default_model = provider
            .models()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self {
            connected: false,
            conversation_id: None,
            execution_status: ExecutionStatus::Idle,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            messages: VecDeque::new(),
            conversation_title: None,
            confirmation_policy: ConfirmationPolicy::AlwaysConfirm,
            pending_actions: Vec::new(),
            notifications: Vec::new(),
            elapsed_seconds: 0,
            start_time: None,
            total_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_cost: 0.0,
            context_window: 200000, // Default, will be updated from server
            should_exit: false,
            exit_confirmation_pending: false,
            show_token_modal: false,
            show_help_modal: false,
            show_policy_modal: false,
            show_settings_modal: false,
            settings_field: 0,
            settings_editing: false,
            settings_edit_buffer: String::new(),
            llm_provider: provider,
            llm_model: default_model,
            llm_api_key: String::new(),
            llm_base_url: None,
            show_command_menu: false,
            command_menu_selected: 0,
            confirmation_selected: 0,
            spinner_tick: 0,
            spinner_style: SpinnerStyle::Braille,
            fun_fact_index: 0,
            workspace_path: ".".to_string(),
            needs_stats_refresh: false,
            server_starting: false,
            server_starting_tick: 0,
            theme: Theme::default(),
            theme_name: ThemeName::Rho,
            show_theme_modal: false,
            theme_selected: 0,
            theme_before_preview: None,
        }
    }
}

impl AppState {
    /// Add a message to the conversation
    pub fn add_message(&mut self, message: DisplayMessage) {
        self.messages.push_back(message);
        if self.messages.len() > MAX_DISPLAY_MESSAGES {
            self.messages.pop_front();
        }
        // Auto-scroll to bottom
        self.scroll_to_bottom();
    }

    /// Process an incoming event
    pub fn process_event(&mut self, event: Event) {
        match event {
            Event::MessageEvent(msg) => {
                // Skip user messages - we already display them locally when sent
                if msg.base.source.as_deref() == Some("user") {
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
                let msg = DisplayMessage::action(&action);
                self.add_message(msg);

                // Check if confirmation is needed based on policy and security risk
                if self.needs_confirmation(&action) {
                    tracing::info!(
                        "Action requires confirmation: {} (risk: {:?})",
                        action.tool_name,
                        action.security_risk
                    );
                    self.pending_actions.push(PendingAction {
                        tool_call_id: action.tool_call_id.clone(),
                        tool_name: action.tool_name.clone(),
                        summary: action
                            .summary
                            .clone()
                            .unwrap_or_else(|| action.tool_name.clone()),
                        security_risk: action.security_risk.unwrap_or_default(),
                    });
                    self.input_mode = InputMode::Confirmation;
                    // Set status to waiting - the server should also send this
                    self.execution_status = ExecutionStatus::WaitingForConfirmation;
                }
            }
            Event::ObservationEvent(obs) => {
                tracing::debug!(
                    "Agent observation: tool={} result={}",
                    obs.tool_name,
                    obs.observation
                );
                // Don't add observations as separate messages - the action already shows
                // what the tool is doing. Observations just clutter the display.
                // Instead, mark the corresponding action as accepted (shows checkmark).
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
                self.add_message(DisplayMessage::error(&err.error));
            }
            Event::ConversationStateUpdateEvent(update) => {
                tracing::debug!("State update key='{}' value={}", update.key, update.value);
                match update.key.as_str() {
                    "execution_status" => {
                        if let Ok(status) = serde_json::from_value::<ExecutionStatus>(update.value)
                        {
                            let was_running = self.execution_status == ExecutionStatus::Running;
                            self.execution_status = status;
                            // Request stats refresh when execution finishes
                            if was_running && status == ExecutionStatus::Finished {
                                self.needs_stats_refresh = true;
                            }
                            // Show error message when execution fails
                            if status == ExecutionStatus::Error {
                                // Only add generic message if no error message was already shown
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
                        self.parse_metrics(&update.value);
                    }
                    "full_state" => {
                        // Full state: metrics are at stats.usage_to_metrics.{usage_id}
                        if let Some(stats) = update.value.get("stats") {
                            self.parse_metrics(stats);
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
            Event::TokenEvent(_) => {
                // Token events are for streaming updates, ignore for now
            }
            Event::Unknown => {}
        }
    }

    /// Parse metrics from a JSON value
    /// Supports multiple formats including:
    /// - {"usage_to_metrics": {"usage_id": {"accumulated_cost": 0.01, "accumulated_token_usage": {...}}}}
    /// - {"accumulated_cost": 0.01, "accumulated_token_usage": {"prompt_tokens": 100, "completion_tokens": 50, "context_window": 200000}}
    pub fn parse_metrics(&mut self, value: &serde_json::Value) {
        // Check if this is the stats.usage_to_metrics format
        if let Some(usage_map) = value.get("usage_to_metrics").and_then(|v| v.as_object()) {
            // Iterate over all usage entries and sum them
            let mut total_cost = 0.0;
            let mut total_prompt = 0u64;
            let mut total_completion = 0u64;

            for (_usage_id, metrics) in usage_map {
                if let Some(cost) = metrics.get("accumulated_cost").and_then(|v| v.as_f64()) {
                    total_cost += cost;
                }
                if let Some(usage) = metrics.get("accumulated_token_usage") {
                    if let Some(prompt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
                        total_prompt += prompt;
                    }
                    if let Some(completion) =
                        usage.get("completion_tokens").and_then(|v| v.as_u64())
                    {
                        total_completion += completion;
                    }
                    // Get context window from token usage
                    if let Some(ctx) = usage.get("context_window").and_then(|v| v.as_u64()) {
                        if ctx > 0 {
                            self.context_window = ctx;
                        }
                    }
                }
            }

            self.total_cost = total_cost;
            self.prompt_tokens = total_prompt;
            self.completion_tokens = total_completion;
            self.total_tokens = total_prompt + total_completion;

            if self.total_tokens > 0 || self.total_cost > 0.0 {
                tracing::info!(
                    "Updated metrics: tokens={} (prompt={}, completion={}), cost={}, context={}",
                    self.total_tokens,
                    self.prompt_tokens,
                    self.completion_tokens,
                    self.total_cost,
                    self.context_window
                );
            }
            return;
        }

        // Direct format: {"accumulated_cost": 0.01, "accumulated_token_usage": {...}}
        if let Some(cost) = value.get("accumulated_cost").and_then(|v| v.as_f64()) {
            self.total_cost = cost;
        }

        if let Some(usage) = value.get("accumulated_token_usage") {
            if let Some(prompt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
                self.prompt_tokens = prompt;
            }
            if let Some(completion) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
                self.completion_tokens = completion;
            }
            if let Some(ctx) = usage.get("context_window").and_then(|v| v.as_u64()) {
                if ctx > 0 {
                    self.context_window = ctx;
                }
            }
            self.total_tokens = self.prompt_tokens + self.completion_tokens;

            if self.total_tokens > 0 {
                tracing::info!(
                    "Updated metrics: tokens={} (prompt={}, completion={}), cost={}, context={}",
                    self.total_tokens,
                    self.prompt_tokens,
                    self.completion_tokens,
                    self.total_cost,
                    self.context_window
                );
            }
        }
    }

    /// Check if an action needs confirmation based on policy
    fn needs_confirmation(&self, action: &ActionEvent) -> bool {
        match self.confirmation_policy {
            ConfirmationPolicy::NeverConfirm => false,
            ConfirmationPolicy::AlwaysConfirm => true,
            ConfirmationPolicy::ConfirmRisky => {
                matches!(
                    action.security_risk,
                    Some(SecurityRisk::Medium) | Some(SecurityRisk::High)
                )
            }
        }
    }

    /// Handle user input character
    pub fn handle_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    /// Handle backspace
    pub fn handle_backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input_buffer.remove(self.cursor_position);
        }
    }

    /// Handle delete
    pub fn handle_delete(&mut self) {
        if self.cursor_position < self.input_buffer.len() {
            self.input_buffer.remove(self.cursor_position);
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor_position < self.input_buffer.len() {
            self.cursor_position += 1;
        }
    }

    /// Move cursor to start
    pub fn cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to end
    pub fn cursor_end(&mut self) {
        self.cursor_position = self.input_buffer.len();
    }

    /// Get and clear the input buffer
    pub fn take_input(&mut self) -> String {
        let input = std::mem::take(&mut self.input_buffer);
        self.cursor_position = 0;
        input
    }

    /// Scroll up (increase offset = show older content)
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    /// Scroll down (decrease offset = show newer content)
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll to bottom (offset 0 = at the latest messages)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Expand or collapse all actions
    pub fn toggle_all_actions(&mut self) {
        // Check if any action is collapsed
        let any_collapsed = self
            .messages
            .iter()
            .any(|msg| matches!(msg.role, MessageRole::Action) && msg.collapsed);

        // If any is collapsed, expand all; otherwise collapse all
        let new_state = !any_collapsed;
        for msg in &mut self.messages {
            if matches!(msg.role, MessageRole::Action) {
                msg.collapsed = new_state;
            }
        }
    }

    /// Add a notification
    pub fn notify(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    /// Remove expired notifications
    pub fn cleanup_notifications(&mut self, max_age: Duration) {
        self.notifications.retain(|n| !n.is_expired(max_age));
    }

    /// Clear pending confirmations and mark corresponding messages as accepted
    pub fn clear_pending_actions(&mut self) {
        // Mark the corresponding action messages as accepted
        for pending in &self.pending_actions {
            // Find matching action message by tool_call_id
            for msg in self.messages.iter_mut() {
                if msg.role == MessageRole::Action {
                    if let Some(ref msg_id) = msg.id {
                        if msg_id == &pending.tool_call_id {
                            msg.accepted = true;
                            break;
                        }
                    }
                }
            }
        }
        self.pending_actions.clear();
        if self.input_mode == InputMode::Confirmation {
            self.input_mode = InputMode::Normal;
        }
    }

    /// Update elapsed time
    pub fn update_elapsed(&mut self) {
        if let Some(start) = self.start_time {
            if self.execution_status == ExecutionStatus::Running {
                self.elapsed_seconds = start.elapsed().as_secs();
            }
        }
    }

    /// Start timing
    pub fn start_timer(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Check if agent is running
    pub fn is_running(&self) -> bool {
        self.execution_status == ExecutionStatus::Running
    }

    /// Advance the spinner animation
    pub fn tick_spinner(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);
    }

    /// Change to a new random fun fact
    pub fn next_fun_fact(&mut self) {
        use crate::config::theme::FUN_FACTS;
        // Simple rotation - could use random if desired
        self.fun_fact_index = (self.fun_fact_index + 1) % FUN_FACTS.len();
    }

    /// Cycle to the next spinner style (call when starting a new LLM request)
    pub fn randomize_spinner(&mut self) {
        self.spinner_style = self.spinner_style.next();
        self.spinner_tick = 0;
    }

    /// Get the current spinner frame
    pub fn spinner_frame(&self) -> &'static str {
        let frames = self.spinner_style.frames();
        frames[self.spinner_tick % frames.len()]
    }

    /// Get the current fun fact
    pub fn current_fun_fact(&self) -> &'static str {
        use crate::config::theme::FUN_FACTS;
        FUN_FACTS[self.fun_fact_index % FUN_FACTS.len()]
    }

    /// Set workspace path
    pub fn set_workspace(&mut self, path: String) {
        self.workspace_path = path;
    }
}
