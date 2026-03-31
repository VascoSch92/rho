//! Slash command handling (/help, /new, /settings, etc.)

use super::AppCommand;
use crate::config::theme::ThemeName;
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
                match name.parse::<ThemeName>() {
                    Ok(theme_name) => {
                        state.theme = theme_name.to_theme();
                        state.theme_name = theme_name;
                        state.notify(Notification::info(
                            "Theme Changed",
                            format!("Switched to {} theme", theme_name),
                        ));
                    }
                    Err(_) => {
                        let available: Vec<String> =
                            ThemeName::all().iter().map(|t| t.to_string()).collect();
                        state.add_message(DisplayMessage::error(format!(
                            "Unknown theme: {}. Available: {}",
                            name,
                            available.join(", "),
                        )));
                    }
                }
            } else {
                // Open theme picker modal, save current for revert on Esc
                let themes = ThemeName::all();
                state.theme_selected = themes
                    .iter()
                    .position(|t| *t == state.theme_name)
                    .unwrap_or(0);
                state.theme_before_preview = Some(state.theme_name);
                state.show_theme_modal = true;
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
                        open_policy_modal(state);
                        None
                    }
                }
            } else {
                open_policy_modal(state);
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

fn open_policy_modal(state: &mut AppState) {
    let policies = [
        ConfirmationPolicy::AlwaysConfirm,
        ConfirmationPolicy::ConfirmRisky,
        ConfirmationPolicy::NeverConfirm,
    ];
    state.policy_selected = policies
        .iter()
        .position(|p| *p == state.confirmation_policy)
        .unwrap_or(0);
    state.show_policy_modal = true;
}
