//! Modal dialog widgets with shared styling.

mod confirmation;
mod exit;
pub mod frame;
mod help;
mod resume;
pub mod settings;
mod skills;
pub mod tabs;
mod theme;
mod token;
mod tools;

pub use confirmation::{ConfirmOption, ConfirmationPanel};
pub use exit::ExitConfirmationModal;
pub use help::{HelpModal, PolicyModal};
pub use resume::ResumeModal;
pub use settings::SettingsModal;
pub use skills::SkillsModal;
pub use theme::ThemeModal;
pub use token::TokenUsageModal;
pub use tools::ToolsModal;
