//! Slash command handling (/help, /new, /settings, etc.)

use super::AppCommand;
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
        Some("tools") => {
            state.show_tools_modal = true;
            Some(AppCommand::LoadTools)
        }
        Some("skills") => {
            state.skills_modal.show = true;
            state.skills_modal.tab = 0;
            state.skills_modal.selected = 0;
            state.skills_modal.detail_open = false;
            // Trigger initial load
            Some(AppCommand::LoadSkills)
        }
        Some("settings") => {
            state.settings.show = true;
            None
        }
        Some("pause") => Some(AppCommand::Pause),
        Some("resume") => {
            state.resume_modal.conversations = crate::state::conversations::scan_conversations();
            state.resume_modal.selected = 0;
            state.resume_modal.confirm_delete = false;
            state.resume_modal.show = true;
            None
        }
        Some("theme") => {
            if let Some(name) = parts.get(1) {
                let name = name.to_lowercase();
                if let Some(&theme) = state.themes.get(&name) {
                    state.theme = theme;
                    state.theme_name = name.clone();
                    if let Err(e) = crate::config::save_theme(&name) {
                        tracing::warn!("Failed to save theme: {}", e);
                    }
                    state.notify(Notification::info(
                        "Theme Changed",
                        format!("Switched to {} theme", name),
                    ));
                } else {
                    let available = state.available_themes.join(", ");
                    state.add_message(DisplayMessage::error(format!(
                        "Unknown theme: {}. Available: {}",
                        name, available,
                    )));
                }
            } else {
                // Open theme picker modal, save current for revert on Esc
                state.theme_modal.selected = state
                    .available_themes
                    .iter()
                    .position(|t| *t == state.theme_name)
                    .unwrap_or(0);
                state.theme_modal.before_preview = Some(state.theme_name.clone());
                state.theme_modal.show = true;
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
        Some("rename") => {
            let new_name = parts[1..].join(" ");
            if new_name.is_empty() {
                state.add_message(DisplayMessage::error("Usage: /rename <new name>"));
                None
            } else if state.conversation_id.is_none() {
                state.add_message(DisplayMessage::error("No active conversation to rename."));
                None
            } else {
                Some(AppCommand::RenameConversation(new_name))
            }
        }
        Some("exit") | Some("quit") => {
            state.exit_confirmation_pending = true;
            state.exit_confirmation_selected = 0;
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
