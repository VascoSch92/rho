//! Application state management.
//!
//! Split into focused sub-modules:
//! - `types` — core types (DisplayMessage, Notification, etc.)
//! - `metrics` — token/cost tracking
//! - `llm` — LLM provider/model configuration
//! - `settings` — settings modal state
//! - `events` — event processing (process_event)
//! - `helpers` — input, scroll, timer, spinner helpers
//! - `conversations` — stored conversation scanning

pub mod conversations;
mod events;
mod helpers;
pub mod llm;
pub mod metrics;
pub mod settings;
pub mod types;

use std::collections::VecDeque;

use uuid::Uuid;

use crate::client::ExecutionStatus;
use crate::config::keybindings::KeyBindingsConfig;
use crate::config::theme::Theme;
use crate::config::RhoConfig;

// Re-export commonly used types at the state:: level
pub use llm::{LlmProvider, LlmState};
pub use metrics::MetricsState;
pub use settings::SettingsState;
pub use types::{
    ConfirmationPolicy, DisplayMessage, InputMode, MessageRole, Notification, NotificationSeverity,
    PendingAction,
};

/// CLI version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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

    // Sub-states
    pub metrics: MetricsState,
    pub llm: LlmState,
    pub settings: SettingsState,

    // Replay flag — true while replaying stored events on resume
    pub replaying: bool,
    // Exit flag
    pub should_exit: bool,
    pub exit_confirmation_pending: bool,

    // Modals
    pub show_token_modal: bool,
    pub show_help_modal: bool,
    pub show_policy_modal: bool,

    // Command menu state
    pub show_command_menu: bool,
    pub command_menu_selected: usize,

    // Confirmation dialog state (arrow key navigation)
    pub confirmation_selected: usize,

    // Animation state
    pub spinner_tick: usize,
    pub spinner_style: String,
    pub spinner_frames: Vec<String>,
    pub spinners: std::collections::HashMap<String, Vec<String>>,
    pub spinner_names: Vec<String>,
    pub fun_fact_index: usize,
    pub fun_facts: Vec<String>,

    // Config
    pub keybindings: KeyBindingsConfig,
    pub scroll_lines: usize,
    pub scroll_lines_large: usize,

    // Workspace info
    pub workspace_path: String,

    // Flag to request stats refresh (set when execution finishes)
    pub needs_stats_refresh: bool,

    // Server startup state
    pub server_starting: bool,
    pub server_starting_tick: usize,

    // Policy modal state
    pub policy_selected: usize,

    // Resume modal
    pub show_resume_modal: bool,
    pub resume_conversations: Vec<conversations::ConversationEntry>,
    pub resume_selected: usize,
    pub resume_confirm_delete: bool,

    // Theme
    pub theme: Theme,
    pub theme_name: String,
    pub available_themes: Vec<String>,
    pub themes: std::collections::HashMap<String, Theme>,
    pub show_theme_modal: bool,
    pub theme_selected: usize,
    pub theme_before_preview: Option<String>,
}

impl AppState {
    /// Create AppState with config applied.
    pub fn with_config(config: RhoConfig) -> Self {
        let provider = LlmProvider::Anthropic;
        let default_model = provider
            .models()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let theme = config.resolve_theme(&config.theme_name);
        let spinner_frames = config
            .spinners
            .get(&config.spinner_style)
            .cloned()
            .unwrap_or_default();
        Self {
            spinner_style: config.spinner_style,
            spinner_frames,
            spinners: config.spinners,
            spinner_names: config.spinner_names,
            fun_facts: config.fun_facts,
            keybindings: config.keybindings,
            scroll_lines: config.scroll_lines,
            scroll_lines_large: config.scroll_lines_large,
            theme,
            theme_name: config.theme_name,
            available_themes: config.theme_names,
            themes: config.themes,
            ..Self::new_default(provider, default_model)
        }
    }

    fn new_default(provider: LlmProvider, default_model: String) -> Self {
        let defaults = RhoConfig::default();
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
            metrics: MetricsState {
                context_window: 200000,
                ..MetricsState::default()
            },
            llm: LlmState {
                provider,
                model: default_model,
                ..LlmState::default()
            },
            settings: SettingsState::default(),
            replaying: false,
            should_exit: false,
            exit_confirmation_pending: false,
            show_token_modal: false,
            show_help_modal: false,
            show_policy_modal: false,
            show_command_menu: false,
            command_menu_selected: 0,
            confirmation_selected: 0,
            spinner_tick: 0,
            spinner_style: defaults.spinner_style.clone(),
            spinner_frames: defaults
                .spinners
                .get(&defaults.spinner_style)
                .cloned()
                .unwrap_or_default(),
            spinners: defaults.spinners,
            spinner_names: defaults.spinner_names,
            fun_fact_index: 0,
            fun_facts: defaults.fun_facts,
            keybindings: defaults.keybindings,
            scroll_lines: defaults.scroll_lines,
            scroll_lines_large: defaults.scroll_lines_large,
            workspace_path: ".".to_string(),
            needs_stats_refresh: false,
            server_starting: false,
            server_starting_tick: 0,
            theme: Theme::default(),
            theme_name: "rho".into(),
            show_resume_modal: false,
            resume_conversations: Vec::new(),
            resume_selected: 0,
            resume_confirm_delete: false,
            available_themes: defaults.theme_names,
            themes: defaults.themes,
            policy_selected: 0,
            show_theme_modal: false,
            theme_selected: 0,
            theme_before_preview: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        let provider = LlmProvider::Anthropic;
        let default_model = provider
            .models()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self::new_default(provider, default_model)
    }
}
