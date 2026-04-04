//! Settings modal input handling.

use crossterm::event::{self, KeyCode};

use super::AppCommand;
use crate::state::{AppState, LlmProvider};
use crate::ui::modals::SETTINGS_FIELD_COUNT;

/// Handle settings modal input
pub fn handle_settings_modal_input(
    state: &mut AppState,
    key: event::KeyEvent,
) -> Option<AppCommand> {
    // ── Dropdown mode (provider or model list open) ──────────────────────
    if state.settings.dropdown {
        return handle_dropdown_input(state, key);
    }

    // ── Text editing mode (API key or base URL) ─────────────────────────
    if state.settings.editing {
        return handle_text_edit_input(state, key);
    }

    // ── Normal navigation mode ──────────────────────────────────────────
    match key.code {
        KeyCode::Esc => {
            // Persist LLM settings to config file on close
            let model = format!(
                "{}/{}",
                state.llm.provider.provider_prefix(),
                state.llm.model
            );
            if let Err(e) =
                crate::config::save_llm(&model, &state.llm.api_key, state.llm.base_url.as_deref())
            {
                tracing::warn!("Failed to save LLM settings: {}", e);
            }
            state.settings.show = false;
            state.settings.field = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.settings.field > 0 {
                state.settings.field -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.settings.field < SETTINGS_FIELD_COUNT - 1 {
                state.settings.field += 1;
            }
        }
        KeyCode::Enter => {
            match state.settings.field {
                0 => {
                    // Open provider dropdown
                    let providers = LlmProvider::all();
                    state.settings.dropdown = true;
                    state.settings.dropdown_selected = providers
                        .iter()
                        .position(|p| *p == state.llm.provider)
                        .unwrap_or(0);
                }
                1 => {
                    // Open model dropdown
                    let models = state.llm.provider.models();
                    state.settings.dropdown = true;
                    state.settings.dropdown_selected = models
                        .iter()
                        .position(|m| *m == state.llm.model)
                        .unwrap_or(0);
                }
                2 => {
                    // API Key - enter text edit mode
                    state.settings.editing = true;
                    state.settings.edit_buffer = state.llm.api_key.clone();
                }
                3 => {
                    // Base URL - enter text edit mode
                    state.settings.editing = true;
                    state.settings.edit_buffer = state.llm.base_url.clone().unwrap_or_default();
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

/// Handle input when a dropdown list is open (provider or model).
fn handle_dropdown_input(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    let item_count = match state.settings.field {
        0 => LlmProvider::all().len(),
        1 => state.llm.provider.models().len(),
        _ => 0,
    };

    match key.code {
        KeyCode::Esc => {
            state.settings.dropdown = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.settings.dropdown_selected = state.settings.dropdown_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.settings.dropdown_selected =
                (state.settings.dropdown_selected + 1).min(item_count.saturating_sub(1));
        }
        KeyCode::Enter => {
            match state.settings.field {
                0 => {
                    // Select provider
                    let providers = LlmProvider::all();
                    if let Some(p) = providers.get(state.settings.dropdown_selected) {
                        state.llm.provider = p.clone();
                        // Reset model to first for new provider
                        let models = state.llm.provider.models();
                        state.llm.model = models.first().map(|s| s.to_string()).unwrap_or_default();
                    }
                }
                1 => {
                    // Select model
                    let models = state.llm.provider.models();
                    if let Some(m) = models.get(state.settings.dropdown_selected) {
                        state.llm.model = m.to_string();
                    }
                }
                _ => {}
            }
            state.settings.dropdown = false;
        }
        _ => {}
    }
    None
}

/// Handle input when editing a text field (API key or base URL).
fn handle_text_edit_input(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    match key.code {
        KeyCode::Esc => {
            state.settings.editing = false;
            state.settings.edit_buffer.clear();
        }
        KeyCode::Enter => {
            match state.settings.field {
                2 => {
                    state.llm.api_key = state.settings.edit_buffer.clone();
                }
                3 => {
                    if state.settings.edit_buffer.is_empty() {
                        state.llm.base_url = None;
                    } else {
                        state.llm.base_url = Some(state.settings.edit_buffer.clone());
                    }
                }
                _ => {}
            }
            state.settings.editing = false;
            state.settings.edit_buffer.clear();
        }
        KeyCode::Backspace => {
            state.settings.edit_buffer.pop();
        }
        KeyCode::Char(c) => {
            state.settings.edit_buffer.push(c);
        }
        _ => {}
    }
    None
}
