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
    let providers = LlmProvider::all();
    let models = state.llm_provider.models();

    if state.settings_editing {
        // In editing mode for text fields (API key, base URL)
        match key.code {
            KeyCode::Esc => {
                // Cancel editing
                state.settings_editing = false;
                state.settings_edit_buffer.clear();
            }
            KeyCode::Enter => {
                // Save the edited value
                match state.settings_field {
                    2 => {
                        // API Key
                        state.llm_api_key = state.settings_edit_buffer.clone();
                    }
                    3 => {
                        // Base URL
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
        return None;
    }

    // Normal navigation mode
    match key.code {
        KeyCode::Esc => {
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
        KeyCode::Left | KeyCode::Char('h') => {
            match state.settings_field {
                0 => {
                    // Provider - cycle backward
                    if let Some(idx) = providers.iter().position(|p| *p == state.llm_provider) {
                        let new_idx = if idx == 0 {
                            providers.len() - 1
                        } else {
                            idx - 1
                        };
                        state.llm_provider = providers[new_idx].clone();
                        // Reset model to first available for new provider
                        let new_models = state.llm_provider.models();
                        state.llm_model = new_models
                            .first()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                    }
                }
                1 => {
                    // Model - cycle backward
                    if let Some(idx) = models.iter().position(|m| *m == state.llm_model) {
                        let new_idx = if idx == 0 { models.len() - 1 } else { idx - 1 };
                        state.llm_model = models[new_idx].to_string();
                    }
                }
                _ => {}
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            match state.settings_field {
                0 => {
                    // Provider - cycle forward
                    if let Some(idx) = providers.iter().position(|p| *p == state.llm_provider) {
                        let new_idx = (idx + 1) % providers.len();
                        state.llm_provider = providers[new_idx].clone();
                        // Reset model to first available for new provider
                        let new_models = state.llm_provider.models();
                        state.llm_model = new_models
                            .first()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                    }
                }
                1 => {
                    // Model - cycle forward
                    if !models.is_empty() {
                        if let Some(idx) = models.iter().position(|m| *m == state.llm_model) {
                            let new_idx = (idx + 1) % models.len();
                            state.llm_model = models[new_idx].to_string();
                        } else {
                            // Current model not in list, select first
                            state.llm_model = models[0].to_string();
                        }
                    }
                }
                _ => {}
            }
        }
        KeyCode::Enter => {
            match state.settings_field {
                0 | 1 => {
                    // Provider/Model fields cycle on Enter too
                    match state.settings_field {
                        0 => {
                            if let Some(idx) =
                                providers.iter().position(|p| *p == state.llm_provider)
                            {
                                let new_idx = (idx + 1) % providers.len();
                                state.llm_provider = providers[new_idx].clone();
                                let new_models = state.llm_provider.models();
                                state.llm_model = new_models
                                    .first()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                            }
                        }
                        1 => {
                            if !models.is_empty() {
                                if let Some(idx) = models.iter().position(|m| *m == state.llm_model)
                                {
                                    let new_idx = (idx + 1) % models.len();
                                    state.llm_model = models[new_idx].to_string();
                                }
                            }
                        }
                        _ => {}
                    }
                }
                2 => {
                    // API Key - enter edit mode
                    state.settings_editing = true;
                    state.settings_edit_buffer = state.llm_api_key.clone();
                }
                3 => {
                    // Base URL - enter edit mode
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
