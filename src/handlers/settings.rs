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
    if state.settings_dropdown {
        return handle_dropdown_input(state, key);
    }

    // ── Text editing mode (API key or base URL) ─────────────────────────
    if state.settings_editing {
        return handle_text_edit_input(state, key);
    }

    // ── Normal navigation mode ──────────────────────────────────────────
    match key.code {
        KeyCode::Esc => {
            // Persist LLM settings to config file on close
            let model = format!(
                "{}/{}",
                state.llm_provider.provider_prefix(),
                state.llm_model
            );
            if let Err(e) =
                crate::config::save_llm(&model, &state.llm_api_key, state.llm_base_url.as_deref())
            {
                tracing::warn!("Failed to save LLM settings: {}", e);
            }
            state.show_settings_modal = false;
            state.settings_field = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.settings_field > 0 {
                state.settings_field -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.settings_field < SETTINGS_FIELD_COUNT - 1 {
                state.settings_field += 1;
            }
        }
        KeyCode::Enter => {
            match state.settings_field {
                0 => {
                    // Open provider dropdown
                    let providers = LlmProvider::all();
                    state.settings_dropdown = true;
                    state.settings_dropdown_selected = providers
                        .iter()
                        .position(|p| *p == state.llm_provider)
                        .unwrap_or(0);
                }
                1 => {
                    // Open model dropdown
                    let models = state.llm_provider.models();
                    state.settings_dropdown = true;
                    state.settings_dropdown_selected = models
                        .iter()
                        .position(|m| *m == state.llm_model)
                        .unwrap_or(0);
                }
                2 => {
                    // API Key - enter text edit mode
                    state.settings_editing = true;
                    state.settings_edit_buffer = state.llm_api_key.clone();
                }
                3 => {
                    // Base URL - enter text edit mode
                    state.settings_editing = true;
                    state.settings_edit_buffer = state.llm_base_url.clone().unwrap_or_default();
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
    let item_count = match state.settings_field {
        0 => LlmProvider::all().len(),
        1 => state.llm_provider.models().len(),
        _ => 0,
    };

    match key.code {
        KeyCode::Esc => {
            state.settings_dropdown = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.settings_dropdown_selected = state.settings_dropdown_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.settings_dropdown_selected =
                (state.settings_dropdown_selected + 1).min(item_count.saturating_sub(1));
        }
        KeyCode::Enter => {
            match state.settings_field {
                0 => {
                    // Select provider
                    let providers = LlmProvider::all();
                    if let Some(p) = providers.get(state.settings_dropdown_selected) {
                        state.llm_provider = p.clone();
                        // Reset model to first for new provider
                        let models = state.llm_provider.models();
                        state.llm_model = models.first().map(|s| s.to_string()).unwrap_or_default();
                    }
                }
                1 => {
                    // Select model
                    let models = state.llm_provider.models();
                    if let Some(m) = models.get(state.settings_dropdown_selected) {
                        state.llm_model = m.to_string();
                    }
                }
                _ => {}
            }
            state.settings_dropdown = false;
        }
        _ => {}
    }
    None
}

/// Handle input when editing a text field (API key or base URL).
fn handle_text_edit_input(state: &mut AppState, key: event::KeyEvent) -> Option<AppCommand> {
    match key.code {
        KeyCode::Esc => {
            state.settings_editing = false;
            state.settings_edit_buffer.clear();
        }
        KeyCode::Enter => {
            match state.settings_field {
                2 => {
                    state.llm_api_key = state.settings_edit_buffer.clone();
                }
                3 => {
                    if state.settings_edit_buffer.is_empty() {
                        state.llm_base_url = None;
                    } else {
                        state.llm_base_url = Some(state.settings_edit_buffer.clone());
                    }
                }
                _ => {}
            }
            state.settings_editing = false;
            state.settings_edit_buffer.clear();
        }
        KeyCode::Backspace => {
            state.settings_edit_buffer.pop();
        }
        KeyCode::Char(c) => {
            state.settings_edit_buffer.push(c);
        }
        _ => {}
    }
    None
}
