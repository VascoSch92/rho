//! Grouped modal state — one sub-struct per modal/popup.
//!
//! These exist to keep the top-level `AppState` from sprawling. Each struct
//! holds only the state specific to its widget; shared data (like `theme` or
//! `keybindings`) stays on `AppState` because many widgets consume it.

use crate::client::SkillInfo;

use super::conversations::ConversationEntry;

/// Skills modal — tabbed list with an inline detail view.
#[derive(Debug, Default)]
pub struct SkillsModalState {
    pub show: bool,
    /// Active tab index (All / User / Project / Public).
    pub tab: usize,
    /// Selected skill within the active tab.
    pub selected: usize,
    /// Whether the inline detail view is displayed instead of the list.
    pub detail_open: bool,
    /// Loaded skills returned by the server.
    pub skills: Vec<SkillInfo>,
    /// `true` while an async load or sync is in progress.
    pub loading: bool,
    /// Last error message from a load/sync call, if any.
    pub error: Option<String>,
}

/// Resume conversation modal.
#[derive(Debug, Default)]
pub struct ResumeModalState {
    pub show: bool,
    pub conversations: Vec<ConversationEntry>,
    pub selected: usize,
    /// Second-step confirmation for the `d` delete action.
    pub confirm_delete: bool,
}

/// Theme picker modal.
#[derive(Debug, Default)]
pub struct ThemeModalState {
    pub show: bool,
    pub selected: usize,
    /// Theme name at the moment the picker opened — used to revert on Esc.
    pub before_preview: Option<String>,
}

/// File path autocomplete menu (triggered by typing `@`).
#[derive(Debug, Default)]
pub struct FileMenuState {
    pub show: bool,
    pub selected: usize,
}

/// Slash command menu (triggered by typing `/`).
#[derive(Debug, Default)]
pub struct CommandMenuState {
    pub show: bool,
    pub selected: usize,
}
