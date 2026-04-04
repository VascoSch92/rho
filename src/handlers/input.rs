//! Key event handling — dispatches to the appropriate mode handler.

use crossterm::event::{self, KeyCode, KeyModifiers};

use super::settings::handle_settings_modal_input;
use super::slash::handle_slash_command;
use super::AppCommand;
use crate::cli::Args;
use crate::config::keybindings::Action;
use crate::state::{AppState, InputMode};
use crate::ui::ConfirmOption;

/// Handle key events and return an optional command
pub fn handle_key_event(
    state: &mut AppState,
    key: event::KeyEvent,
    args: &Args,
) -> Option<AppCommand> {
    // ── Global key bindings (work in any mode) ───────────────────────────
    if let Some(action) = state.keybindings.global.get(&key) {
        match action {
            Action::Quit => {
                if args.exit_without_confirmation {
                    return Some(AppCommand::ForceQuit);
                } else if state.exit_confirmation_pending {
                    return Some(AppCommand::CancelQuit);
                } else {
                    state.exit_confirmation_pending = true;
                    return None;
                }
            }
            Action::ToggleCollapseAll => {
                state.toggle_all_actions();
                return None;
            }
            _ => {}
        }
    }

    // ── Token modal ──────────────────────────────────────────────────────
    if state.show_token_modal {
        if let Some(action) = state.keybindings.modal.get(&key) {
            if matches!(action, Action::Dismiss | Action::Confirm) {
                state.show_token_modal = false;
            }
        }
        return None;
    }

    // ── Help modal ───────────────────────────────────────────────────────
    if state.show_help_modal {
        if let Some(action) = state.keybindings.modal.get(&key) {
            if matches!(action, Action::Dismiss | Action::Confirm) {
                state.show_help_modal = false;
            }
        }
        return None;
    }

    // ── Policy modal ─────────────────────────────────────────────────────
    if state.show_policy_modal {
        let policies = [
            crate::state::ConfirmationPolicy::AlwaysConfirm,
            crate::state::ConfirmationPolicy::ConfirmRisky,
            crate::state::ConfirmationPolicy::NeverConfirm,
        ];
        if let Some(action) = state.keybindings.modal.get(&key) {
            match action {
                Action::Dismiss => {
                    state.show_policy_modal = false;
                }
                Action::NavUp => {
                    state.policy_selected = state.policy_selected.saturating_sub(1);
                }
                Action::NavDown => {
                    state.policy_selected = (state.policy_selected + 1).min(policies.len() - 1);
                }
                Action::Confirm => {
                    state.confirmation_policy = policies[state.policy_selected];
                    state.show_policy_modal = false;
                }
                _ => {}
            }
        }
        return None;
    }

    // ── Theme modal ──────────────────────────────────────────────────────
    if state.show_theme_modal {
        let num_themes = state.available_themes.len();
        if let Some(action) = state.keybindings.modal.get(&key) {
            match action {
                Action::Dismiss => {
                    if let Some(original) = state.theme_before_preview.take() {
                        if let Some(&t) = state.themes.get(&original) {
                            state.theme = t;
                        }
                        state.theme_name = original;
                    }
                    state.show_theme_modal = false;
                }
                Action::NavUp => {
                    state.theme_selected = state.theme_selected.saturating_sub(1);
                    let name = &state.available_themes[state.theme_selected];
                    if let Some(&t) = state.themes.get(name) {
                        state.theme = t;
                    }
                    state.theme_name = name.clone();
                }
                Action::NavDown => {
                    state.theme_selected =
                        (state.theme_selected + 1).min(num_themes.saturating_sub(1));
                    let name = &state.available_themes[state.theme_selected];
                    if let Some(&t) = state.themes.get(name) {
                        state.theme = t;
                    }
                    state.theme_name = name.clone();
                }
                Action::Confirm => {
                    state.theme_before_preview = None;
                    state.show_theme_modal = false;
                }
                _ => {}
            }
        }
        return None;
    }

    // ── Settings modal ───────────────────────────────────────────────────
    if state.show_settings_modal {
        return handle_settings_modal_input(state, key);
    }

    // ── Notification — any key dismisses ─────────────────────────────────
    if !state.notifications.is_empty() {
        state.notifications.clear();
        return None;
    }

    // ── Exit confirmation ────────────────────────────────────────────────
    if state.exit_confirmation_pending {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => return Some(AppCommand::ForceQuit),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                return Some(AppCommand::CancelQuit)
            }
            _ => return None,
        }
    }

    // ── Confirmation mode ────────────────────────────────────────────────
    if state.input_mode == InputMode::Confirmation {
        let num_options = ConfirmOption::all().len();
        if let Some(action) = state.keybindings.confirmation.get(&key) {
            match action {
                Action::NavLeft => {
                    state.confirmation_selected = state.confirmation_selected.saturating_sub(1);
                    return None;
                }
                Action::NavRight => {
                    state.confirmation_selected =
                        (state.confirmation_selected + 1).min(num_options - 1);
                    return None;
                }
                Action::Confirm => {
                    let selected = ConfirmOption::all()[state.confirmation_selected];
                    state.confirmation_selected = 0;
                    return match selected {
                        ConfirmOption::Accept => Some(AppCommand::ConfirmYes),
                        ConfirmOption::AlwaysAccept => Some(AppCommand::ConfirmAll),
                        ConfirmOption::Reject => Some(AppCommand::ConfirmNo),
                    };
                }
                Action::ConfirmAll => return Some(AppCommand::ConfirmAll),
                Action::Reject => return Some(AppCommand::ConfirmNo),
                Action::Dismiss => return Some(AppCommand::ConfirmDefer),
                _ => return None,
            }
        }
        return None;
    }

    // ── Command menu navigation ──────────────────────────────────────────
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
                if let Some(cmd) = crate::ui::command_menu::selected_command(state) {
                    state.input_buffer = format!("/{}", cmd);
                    state.cursor_position = state.input_buffer.len();
                    state.show_command_menu = false;
                }
                return None;
            }
            KeyCode::Enter => {
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

    // ── Normal input mode (config-driven where possible) ─────────────────
    if let Some(action) = state.keybindings.normal.get(&key) {
        match action {
            Action::Submit => {
                state.show_command_menu = false;
                let input = state.take_input();
                if input.is_empty() {
                    return None;
                }
                if let Some(cmd) = input.strip_prefix('/') {
                    return handle_slash_command(cmd, state);
                }
                if let Some(cmd) = input.strip_prefix('!') {
                    if !cmd.is_empty() {
                        return Some(AppCommand::RunBashCommand(cmd.to_string()));
                    }
                }
                return Some(AppCommand::SendMessage(input));
            }
            Action::NewLine => {
                state.input_buffer.insert(state.cursor_position, '\n');
                state.cursor_position += 1;
                return None;
            }
            Action::ScrollUp => {
                state.scroll_up(state.scroll_lines);
                return None;
            }
            Action::ScrollDown => {
                state.scroll_down(state.scroll_lines);
                return None;
            }
            Action::ScrollUpLarge => {
                state.scroll_up(state.scroll_lines_large);
                return None;
            }
            Action::ScrollDownLarge => {
                state.scroll_down(state.scroll_lines_large);
                return None;
            }
            Action::CursorLeft => {
                state.cursor_left();
                return None;
            }
            Action::CursorRight => {
                state.cursor_right();
                return None;
            }
            Action::CursorHome => {
                state.cursor_home();
                return None;
            }
            Action::CursorEnd => {
                state.cursor_end();
                return None;
            }
            Action::Backspace => {
                state.handle_backspace();
                state.show_command_menu =
                    state.input_buffer.starts_with('/') && state.input_buffer.len() <= 10;
                return None;
            }
            Action::Delete => {
                state.handle_delete();
                return None;
            }
            Action::Pause => {
                if state.show_command_menu {
                    state.show_command_menu = false;
                } else if state.is_running() {
                    return Some(AppCommand::Pause);
                }
                return None;
            }
            _ => {}
        }
    }

    // ── Character input (not bound to any action) ────────────────────────
    if let KeyCode::Char(c) = key.code {
        // Don't capture modified chars (ctrl-x is handled by global bindings above)
        if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
            state.handle_char(c);
            if state.input_buffer.starts_with('/') && state.input_buffer.len() <= 10 {
                state.show_command_menu = true;
                state.command_menu_selected = 0;
            } else {
                state.show_command_menu = false;
            }
        }
    }

    None
}
