//! Input handling and command processing.

mod commands;
mod input;
mod settings;
mod slash;

pub use commands::process_command;
pub use input::handle_key_event;

use crate::state::ConfirmationPolicy;

/// Application commands from user input
#[derive(Debug)]
pub enum AppCommand {
    SendMessage(String),
    RunBashCommand(String),
    NewConversation,
    Pause,
    ConfirmYes,
    ConfirmNo,
    ConfirmAll,
    ConfirmDefer,
    SetPolicy(ConfirmationPolicy),
    ForceQuit,
    CancelQuit,
}
