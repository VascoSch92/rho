//! Slash command handling (/help, /new, /settings, etc.)

use super::AppCommand;
use crate::config::theme::Theme;
use crate::state::{AppState, ConfirmationPolicy, DisplayMessage, Notification};

/// Handle slash commands
pub fn handle_slash_command(command: &str, state: &mut AppState) -> Option<AppCommand> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let cmd = parts.first().map(|s| s.to_lowercase());

    match cmd.as_deref() {
        Some("help") => {
            state.show_help_modal = true;
            None
        }
        Some("new") => Some(AppCommand::NewConversation),
        Some("usage") => {
            state.show_token_modal = true;
            None
        }
        Some("settings") => {
            state.show_settings_modal = true;
            None
        }
        Some("pause") => Some(AppCommand::Pause),
        Some("theme") => {
            if let Some(name) = parts.get(1) {
                state.theme = Theme::by_name(name);
                state.theme_name = name.to_lowercase();
                state.notify(Notification::info(
                    "Theme Changed",
                    format!("Switched to {} theme", name),
                ));
            } else {
                let available = Theme::available().join(", ");
                state.add_message(DisplayMessage::system(format!(
                    "Current theme: {}. Available: {}",
                    state.theme_name, available,
                )));
            }
            None
        }
        Some("confirm") => {
            if let Some(policy) = parts.get(1) {
                match policy.to_lowercase().as_str() {
                    "always" => Some(AppCommand::SetPolicy(ConfirmationPolicy::AlwaysConfirm)),
                    "never" => Some(AppCommand::SetPolicy(ConfirmationPolicy::NeverConfirm)),
                    "risky" => Some(AppCommand::SetPolicy(ConfirmationPolicy::ConfirmRisky)),
                    _ => {
                        // Invalid policy - show modal with options
                        state.show_policy_modal = true;
                        None
                    }
                }
            } else {
                // No argument - show policy modal
                state.show_policy_modal = true;
                None
            }
        }
        Some("exit") | Some("quit") => {
            state.exit_confirmation_pending = true;
            None
        }
        _ => {
            state.add_message(DisplayMessage::error(format!(
                "Unknown command: /{}. Type /help for available commands.",
                command
            )));
            None
        }
    }
}
