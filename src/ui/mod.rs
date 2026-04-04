//! User interface components built with Ratatui.

pub mod command_menu;
pub mod formatting;
mod input;
mod layout;
pub mod markdown;
mod messages;
pub mod modals;
mod spinner;
mod status;

pub use layout::render;
pub use modals::ConfirmOption;
