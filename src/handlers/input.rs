//! Key event handling — dispatches to the appropriate mode handler.

use crossterm::event::{self, KeyCode, KeyModifiers};

use super::settings::handle_settings_modal_input;
use super::slash::handle_slash_command;
use super::AppCommand;
use crate::cli::Args;
use crate::state::{AppState, InputMode};
use crate::ui::ConfirmOption;

/// Handle key events and return an optional command
pub fn handle_key_event(
    state: &mut AppState,
    key: event::KeyEvent,
    args: &Args,
) -> Option<AppCommand> {
    // Global key bindings
    match (key.code, key.modifiers) {
        // Quit shortcuts
        (KeyCode::Char('q'), KeyModifiers::CONTROL)
        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            if args.exit_without_confirmation {
                return Some(AppCommand::ForceQuit);
            } else if state.exit_confirmation_pending {
                return Some(AppCommand::CancelQuit);
            } else {
                state.exit_confirmation_pending = true;
                return None;
            }
        }
        // Expand/collapse all actions
        (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
            state.toggle_all_actions();
            return None;
        }
        _ => {}
    }

    // Handle token modal
    if state.show_token_modal {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                state.show_token_modal = false;
                return None;
            }
            _ => return None,
        }
    }

    // Handle help modal
    if state.show_help_modal {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                state.show_help_modal = false;
                return None;
            }
            _ => return None,
        }
    }

    // Handle policy modal
    if state.show_policy_modal {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                state.show_policy_modal = false;
                return None;
            }
            _ => return None,
        }
    }

    // Handle theme modal — live preview on navigate, confirm/revert on Enter/Esc
    if state.show_theme_modal {
        let themes = crate::config::theme::ThemeName::all();
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Revert to original theme
                if let Some(original) = state.theme_before_preview.take() {
                    state.theme = original.to_theme();
                    state.theme_name = original;
                }
                state.show_theme_modal = false;
                return None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.theme_selected = state.theme_selected.saturating_sub(1);
                // Live preview
                let selected = themes[state.theme_selected];
                state.theme = selected.to_theme();
                state.theme_name = selected;
                return None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.theme_selected = (state.theme_selected + 1).min(themes.len() - 1);
                // Live preview
                let selected = themes[state.theme_selected];
                state.theme = selected.to_theme();
                state.theme_name = selected;
                return None;
            }
            KeyCode::Enter => {
                // Confirm — keep the previewed theme
                state.theme_before_preview = None;
                state.show_theme_modal = false;
                return None;
            }
            _ => return None,
        }
    }

    // Handle settings modal
    if state.show_settings_modal {
        return handle_settings_modal_input(state, key);
    }

    // Handle notification modal - any key dismisses
    if !state.notifications.is_empty() {
        state.notifications.clear();
        return None;
    }

    // Handle exit confirmation mode
    if state.exit_confirmation_pending {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => return Some(AppCommand::ForceQuit),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                return Some(AppCommand::CancelQuit)
            }
            _ => return None,
        }
    }

    // Handle confirmation mode with arrow key navigation
    if state.input_mode == InputMode::Confirmation {
        let num_options = ConfirmOption::all().len();
        match key.code {
            // Arrow key navigation
            KeyCode::Left => {
                state.confirmation_selected = state.confirmation_selected.saturating_sub(1);
                return None;
            }
            KeyCode::Right => {
                state.confirmation_selected =
                    (state.confirmation_selected + 1).min(num_options - 1);
                return None;
            }
            // Enter confirms the selected option
            KeyCode::Enter => {
                let selected = ConfirmOption::all()[state.confirmation_selected];
                state.confirmation_selected = 0; // Reset for next time
                return match selected {
                    ConfirmOption::Accept => Some(AppCommand::ConfirmYes),
                    ConfirmOption::AlwaysAccept => Some(AppCommand::ConfirmAll),
                    ConfirmOption::Reject => Some(AppCommand::ConfirmNo),
                };
            }
            // Legacy single-key shortcuts still work
            KeyCode::Char('y') | KeyCode::Char('Y') => return Some(AppCommand::ConfirmYes),
            KeyCode::Char('n') | KeyCode::Char('N') => return Some(AppCommand::ConfirmNo),
            KeyCode::Char('a') | KeyCode::Char('A') => return Some(AppCommand::ConfirmAll),
            KeyCode::Esc => return Some(AppCommand::ConfirmDefer),
            _ => return None,
        }
    }

    // Handle command menu navigation
    if state.show_command_menu {
        match key.code {
            KeyCode::Up => {
                let count = crate::ui::command_menu::command_count(state);
                if count > 0 {
                    state.command_menu_selected = state.command_menu_selected.saturating_sub(1);
                }
                return None;
            }
            KeyCode::Down => {
                let count = crate::ui::command_menu::command_count(state);
                if count > 0 {
                    state.command_menu_selected = (state.command_menu_selected + 1) % count;
                }
                return None;
            }
            KeyCode::Tab => {
                // Autocomplete the selected command
                if let Some(cmd) = crate::ui::command_menu::selected_command(state) {
                    state.input_buffer = format!("/{}", cmd);
                    state.cursor_position = state.input_buffer.len();
                    state.show_command_menu = false;
                }
                return None;
            }
            KeyCode::Enter => {
                // Execute the selected command
                if let Some(cmd) = crate::ui::command_menu::selected_command(state) {
                    state.input_buffer = format!("/{}", cmd);
                    state.cursor_position = state.input_buffer.len();
                    state.show_command_menu = false;
                    let input = state.take_input();
                    return handle_slash_command(&input[1..], state);
                }
                return None;
            }
            KeyCode::Esc => {
                state.show_command_menu = false;
                return None;
            }
            _ => {}
        }
    }

    // Normal input mode
    match key.code {
        KeyCode::Enter => {
            // Alt+Enter or Shift+Enter: add newline
            if key.modifiers.contains(KeyModifiers::ALT)
                || key.modifiers.contains(KeyModifiers::SHIFT)
            {
                state.input_buffer.insert(state.cursor_position, '\n');
                state.cursor_position += 1;
                return None;
            }

            // Regular Enter: submit
            state.show_command_menu = false;

            let input = state.take_input();
            if input.is_empty() {
                return None;
            }

            // Check for slash commands
            if let Some(cmd) = input.strip_prefix('/') {
                return handle_slash_command(cmd, state);
            }

            // Check for bash commands (starts with !)
            if let Some(cmd) = input.strip_prefix('!') {
                if !cmd.is_empty() {
                    return Some(AppCommand::RunBashCommand(cmd.to_string()));
                }
            }

            return Some(AppCommand::SendMessage(input));
        }
        KeyCode::Char(c) => {
            state.handle_char(c);
            // Show command menu when typing /
            if state.input_buffer.starts_with('/') && state.input_buffer.len() <= 10 {
                state.show_command_menu = true;
                state.command_menu_selected = 0;
            } else {
                state.show_command_menu = false;
            }
        }
        KeyCode::Backspace => {
            state.handle_backspace();
            // Update command menu visibility
            state.show_command_menu =
                state.input_buffer.starts_with('/') && state.input_buffer.len() <= 10;
        }
        KeyCode::Delete => {
            state.handle_delete();
        }
        KeyCode::Left => {
            state.cursor_left();
        }
        KeyCode::Right => {
            state.cursor_right();
        }
        KeyCode::Home => {
            state.cursor_home();
        }
        KeyCode::End => {
            state.cursor_end();
        }
        KeyCode::Up => {
            state.scroll_up(3);
        }
        KeyCode::Down => {
            state.scroll_down(3);
        }
        KeyCode::PageUp => {
            state.scroll_up(10);
        }
        KeyCode::PageDown => {
            state.scroll_down(10);
        }
        KeyCode::Esc => {
            if state.show_command_menu {
                state.show_command_menu = false;
            } else if state.is_running() {
                return Some(AppCommand::Pause);
            }
        }
        _ => {}
    }

    None
}
