//! Key event handling — dispatches to the appropriate mode handler.
//!
//! The main `handle_key_event` function checks global bindings first, then
//! delegates to the active modal or normal input mode.

use crossterm::event::{self, KeyCode, KeyModifiers};

use super::settings::handle_settings_modal_input;
use super::slash::handle_slash_command;
use super::AppCommand;
use crate::cli::Args;
use crate::config::keybindings::Action;
use crate::state::{AppState, InputMode};
use crate::ui::ConfirmOption;

/// Handle key events and return an optional command.
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
                    state.exit_confirmation_selected = 0;
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

    // ── Modal dispatch ───────────────────────────────────────────────────
    if state.show_token_modal {
        return handle_token_modal(state, key);
    }
    if state.show_skills_modal {
        return handle_skills_modal(state, key);
    }
    if state.show_help_modal {
        return handle_help_modal(state, key);
    }
    if state.show_policy_modal {
        return handle_policy_modal(state, key);
    }
    if state.show_theme_modal {
        return handle_theme_modal(state, key);
    }
    if state.show_resume_modal {
        return handle_resume_modal(state, key);
    }
    if state.settings.show {
        return handle_settings_modal_input(state, key);
    }
    if !state.notifications.is_empty() {
        state.notifications.clear();
        return None;
    }
    if state.exit_confirmation_pending {
        return handle_exit_confirmation(state, key);
    }
    if state.input_mode == InputMode::Confirmation {
        return handle_confirmation_mode(state, key);
    }

    // ── Command menu ────────────────────────────────────────────────────
    if state.show_command_menu {
        if let Some(cmd) = handle_command_menu(state, key) {
            return cmd;
        }
    }

    // ── File menu ───────────────────────────────────────────────────────
    if state.show_file_menu {
        if let Some(cmd) = handle_file_menu(state, key) {
            return cmd;
        }
    }

    // ── Normal input ────────────────────────────────────────────────────
    let cmd = handle_normal_input(state, key);
    // Refresh the file menu visibility based on the current buffer/cursor
    refresh_file_menu(state);
    cmd
}

/// Update `show_file_menu` based on whether the cursor is inside an `@...` token.
fn refresh_file_menu(state: &mut AppState) {
    let has_token =
        crate::ui::file_menu::parse_token(&state.input_buffer, state.cursor_position).is_some();
    if has_token {
        let entries = crate::ui::file_menu::current_entries(state);
        if !entries.is_empty() {
            if !state.show_file_menu {
                state.file_menu_selected = 0;
            }
            // Clamp selection to the new list size
            state.file_menu_selected = state.file_menu_selected.min(entries.len() - 1);
            state.show_file_menu = true;
            return;
        }
    }
    state.show_file_menu = false;
    state.file_menu_selected = 0;
}

/// File menu navigation (Up/Down/Tab/Enter/Esc). Returns `Some(None)` to
/// consume the key, `Some(Some(cmd))` to return a command, or `None` to fall
/// through to normal input.
fn handle_file_menu(state: &mut AppState, key: event::KeyEvent) -> Option<Option<AppCommand>> {
    let entries = crate::ui::file_menu::current_entries(state);
    if entries.is_empty() {
        return None;
    }
    match key.code {
        KeyCode::Up => {
            state.file_menu_selected = state.file_menu_selected.saturating_sub(1);
            Some(None)
        }
        KeyCode::Down => {
            state.file_menu_selected = (state.file_menu_selected + 1).min(entries.len() - 1);
            Some(None)
        }
        KeyCode::Tab | KeyCode::Enter => {
            if let Some(entry) = entries.get(state.file_menu_selected) {
                crate::ui::file_menu::apply_selection(state, entry);
                // After inserting a file (not a directory), close the menu.
                // For a directory, keep it open so the user can drill down.
                if !entry.is_dir {
                    state.show_file_menu = false;
                    state.file_menu_selected = 0;
                } else {
                    // Refresh entries for the new directory
                    refresh_file_menu(state);
                }
            }
            Some(None)
        }
        KeyCode::Esc => {
            state.show_file_menu = false;
            state.file_menu_selected = 0;
            Some(None)
        }
        _ => None, // Fall through to normal input
    }
}

// ── Modal handlers ──────────────────────────────────────────────────────────

/// Skills modal — tabs + compact list + inline detail view.
fn handle_skills_modal(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    const NUM_TABS: usize = 4;

    // Detail view: only Esc (return to list) is handled
    if state.skill_detail_open {
        if let KeyCode::Esc = key.code {
            state.skill_detail_open = false;
        }
        return None;
    }

    // Count skills in the active tab for clamping navigation
    let tab_count = {
        let tab = state.skills_modal_tab;
        state
            .skills
            .iter()
            .filter(|s| match tab {
                0 => true,
                1 => s
                    .source
                    .as_deref()
                    .map(|src| src.to_lowercase().contains("user"))
                    .unwrap_or(false),
                2 => s
                    .source
                    .as_deref()
                    .map(|src| src.to_lowercase().contains("project"))
                    .unwrap_or(false),
                3 => s
                    .source
                    .as_deref()
                    .map(|src| {
                        let l = src.to_lowercase();
                        l.contains("public") || l.contains("marketplace")
                    })
                    .unwrap_or(false),
                _ => true,
            })
            .count()
    };

    match key.code {
        KeyCode::Tab | KeyCode::Right => {
            state.skills_modal_tab = (state.skills_modal_tab + 1) % NUM_TABS;
            state.skills_modal_selected = 0;
            return None;
        }
        KeyCode::BackTab | KeyCode::Left => {
            state.skills_modal_tab = (state.skills_modal_tab + NUM_TABS - 1) % NUM_TABS;
            state.skills_modal_selected = 0;
            return None;
        }
        KeyCode::Up => {
            state.skills_modal_selected = state.skills_modal_selected.saturating_sub(1);
            return None;
        }
        KeyCode::Down => {
            if tab_count > 0 {
                state.skills_modal_selected = (state.skills_modal_selected + 1).min(tab_count - 1);
            }
            return None;
        }
        KeyCode::Enter => {
            if tab_count > 0 {
                state.skill_detail_open = true;
            }
            return None;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            return Some(AppCommand::SyncSkills);
        }
        KeyCode::Esc => {
            state.show_skills_modal = false;
            state.skills_modal_tab = 0;
            state.skills_modal_selected = 0;
            return None;
        }
        _ => {}
    }
    None
}

/// Help modal with tabs (Commands / Shortcuts).
fn handle_help_modal(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    const NUM_TABS: usize = 2;
    match key.code {
        KeyCode::Tab | KeyCode::Right => {
            state.help_modal_tab = (state.help_modal_tab + 1) % NUM_TABS;
            return None;
        }
        KeyCode::BackTab | KeyCode::Left => {
            state.help_modal_tab = (state.help_modal_tab + NUM_TABS - 1) % NUM_TABS;
            return None;
        }
        _ => {}
    }
    if let Some(action) = state.keybindings.modal.get(&key) {
        if matches!(action, Action::Dismiss | Action::Confirm) {
            state.show_help_modal = false;
            state.help_modal_tab = 0;
        }
    }
    None
}

/// Token usage modal with tabs (Stats / Chart).
fn handle_token_modal(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    const NUM_TABS: usize = 2;
    // Tab switching: Tab / Left / Right
    match key.code {
        KeyCode::Tab | KeyCode::Right => {
            state.token_modal_tab = (state.token_modal_tab + 1) % NUM_TABS;
            return None;
        }
        KeyCode::BackTab | KeyCode::Left => {
            state.token_modal_tab = (state.token_modal_tab + NUM_TABS - 1) % NUM_TABS;
            return None;
        }
        _ => {}
    }
    if let Some(action) = state.keybindings.modal.get(&key) {
        if matches!(action, Action::Dismiss | Action::Confirm) {
            state.show_token_modal = false;
            state.token_modal_tab = 0;
        }
    }
    None
}

/// Policy picker modal.
fn handle_policy_modal(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    let policies = [
        crate::state::ConfirmationPolicy::AlwaysConfirm,
        crate::state::ConfirmationPolicy::ConfirmRisky,
        crate::state::ConfirmationPolicy::NeverConfirm,
    ];
    if let Some(action) = state.keybindings.modal.get(&key) {
        match action {
            Action::Dismiss => state.show_policy_modal = false,
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
    None
}

/// Theme picker modal with live preview.
fn handle_theme_modal(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
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
                apply_theme_preview(state);
            }
            Action::NavDown => {
                state.theme_selected = (state.theme_selected + 1).min(num_themes.saturating_sub(1));
                apply_theme_preview(state);
            }
            Action::Confirm => {
                state.theme_before_preview = None;
                state.show_theme_modal = false;
                if let Err(e) = crate::config::save_theme(&state.theme_name) {
                    tracing::warn!("Failed to save theme: {}", e);
                }
            }
            _ => {}
        }
    }
    None
}

fn apply_theme_preview(state: &mut AppState) {
    let name = &state.available_themes[state.theme_selected];
    if let Some(&t) = state.themes.get(name) {
        state.theme = t;
    }
    state.theme_name = name.clone();
}

/// Resume conversation modal with delete support.
fn handle_resume_modal(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    if state.resume_confirm_delete {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(conv) = state.resume_conversations.get(state.resume_selected) {
                    let id = conv.id.clone();
                    if let Err(e) = crate::state::conversations::delete_conversation(&id) {
                        tracing::warn!("Failed to delete conversation: {}", e);
                    }
                    state.resume_conversations.remove(state.resume_selected);
                    if state.resume_selected > 0
                        && state.resume_selected >= state.resume_conversations.len()
                    {
                        state.resume_selected -= 1;
                    }
                }
                state.resume_confirm_delete = false;
            }
            _ => state.resume_confirm_delete = false,
        }
        return None;
    }

    if let Some(action) = state.keybindings.modal.get(&key) {
        match action {
            Action::Dismiss => state.show_resume_modal = false,
            Action::NavUp => {
                state.resume_selected = state.resume_selected.saturating_sub(1);
            }
            Action::NavDown => {
                let max = state.resume_conversations.len().saturating_sub(1);
                state.resume_selected = (state.resume_selected + 1).min(max);
            }
            Action::Confirm => {
                if let Some(conv) = state.resume_conversations.get(state.resume_selected) {
                    if let Ok(uuid) = uuid::Uuid::parse_str(&conv.id) {
                        state.show_resume_modal = false;
                        return Some(AppCommand::ResumeConversation(uuid));
                    }
                }
            }
            _ => {}
        }
    } else if matches!(key.code, KeyCode::Char('d') | KeyCode::Char('D'))
        && !state.resume_conversations.is_empty()
    {
        state.resume_confirm_delete = true;
    }
    None
}

/// Exit confirmation — arrow navigation + Y/N shortcuts.
fn handle_exit_confirmation(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    match key.code {
        // Y/N shortcuts still work directly
        KeyCode::Char('y') | KeyCode::Char('Y') => Some(AppCommand::ForceQuit),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(AppCommand::CancelQuit),
        // Arrow navigation between Yes (1) and No (0)
        KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
            state.exit_confirmation_selected = 1 - state.exit_confirmation_selected;
            None
        }
        // Enter applies the current selection
        KeyCode::Enter => Some(if state.exit_confirmation_selected == 1 {
            AppCommand::ForceQuit
        } else {
            AppCommand::CancelQuit
        }),
        _ => None,
    }
}

/// Action confirmation dialog (accept/reject/always).
fn handle_confirmation_mode(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    let num_options = ConfirmOption::all().len();
    if let Some(action) = state.keybindings.confirmation.get(&key) {
        match action {
            Action::NavLeft => {
                state.confirmation_selected = state.confirmation_selected.saturating_sub(1);
            }
            Action::NavRight => {
                state.confirmation_selected =
                    (state.confirmation_selected + 1).min(num_options - 1);
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
            _ => {}
        }
    }
    None
}

// ── Command menu & normal input ─────────────────────────────────────────────

/// Handle command menu navigation. Returns Some(Some(cmd)) to return a command,
/// Some(None) to consume the event, or None to fall through to normal input.
fn handle_command_menu(state: &mut AppState, key: event::KeyEvent) -> Option<Option<AppCommand>> {
    match key.code {
        KeyCode::Up => {
            let count = crate::ui::command_menu::command_count(state);
            if count > 0 {
                state.command_menu_selected = state.command_menu_selected.saturating_sub(1);
            }
            Some(None)
        }
        KeyCode::Down => {
            let count = crate::ui::command_menu::command_count(state);
            if count > 0 {
                state.command_menu_selected = (state.command_menu_selected + 1) % count;
            }
            Some(None)
        }
        KeyCode::Tab => {
            if let Some(cmd) = crate::ui::command_menu::selected_command(state) {
                state.input_buffer = format!("/{}", cmd);
                state.cursor_position = state.input_buffer.len();
                state.show_command_menu = false;
            }
            Some(None)
        }
        KeyCode::Enter => {
            if let Some(cmd) = crate::ui::command_menu::selected_command(state) {
                state.input_buffer = format!("/{}", cmd);
                state.cursor_position = state.input_buffer.len();
                state.show_command_menu = false;
                let input = state.take_input();
                return Some(handle_slash_command(&input[1..], state));
            }
            Some(None)
        }
        KeyCode::Esc => {
            state.show_command_menu = false;
            Some(None)
        }
        _ => None, // Fall through to normal input
    }
}

/// Handle normal input mode (typing, scrolling, cursor movement).
fn handle_normal_input(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
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

    // Character input (not bound to any action)
    if let KeyCode::Char(c) = key.code {
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
