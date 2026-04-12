//! User interface components built with Ratatui.

pub mod command_menu;
pub mod file_menu;
pub mod formatting;
pub mod input;
mod layout;
pub mod markdown;
pub mod messages;
pub mod modals;
pub mod spinner;
pub mod status;
pub mod tasks;

pub use layout::render;
pub use modals::ConfirmOption;
