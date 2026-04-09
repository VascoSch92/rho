//! Input handling and command processing.

mod commands;
mod input;
mod settings;
mod slash;

pub use commands::{process_command, resume_conversation};
pub use input::handle_key_event;

use uuid::Uuid;

use crate::state::ConfirmationPolicy;

/// Application commands from user input
#[derive(Debug)]
pub enum AppCommand {
    SendMessage(String),
    RunBashCommand(String),
    NewConversation,
    ResumeConversation(Uuid),
    Pause,
    ConfirmYes,
    ConfirmNo,
    ConfirmAll,
    ConfirmDefer,
    SetPolicy(ConfirmationPolicy),
    RenameConversation(String),
    LoadSkills,
    SyncSkills,
    ForceQuit,
    CancelQuit,
}
