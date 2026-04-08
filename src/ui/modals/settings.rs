//! Settings modal widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::{AppState, LlmProvider};

/// Number of settings fields
pub const SETTINGS_FIELD_COUNT: usize = 4;

/// Settings modal showing configuration options
pub struct SettingsModal<'a> {
    state: &'a AppState,
}

impl<'a> SettingsModal<'a> {
    const TITLE: &'static str = "Settings";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn mask_api_key(key: &str) -> String {
        if key.is_empty() {
            "(not set)".to_string()
        } else if key.len() <= 4 {
            "*".repeat(key.len())
        } else {
            format!("{}{}", &key[..4], "*".repeat(key.len().min(12) - 4))
        }
    }
}

impl Widget for SettingsModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();
        let selected = self.state.settings.field;
        let editing = self.state.settings.editing;
        let dropdown = self.state.settings.dropdown;

        lines.push(Line::from(""));

        // ── Provider field (0) ───────────────────────────────────────────
        let is_selected = selected == 0;
        let label_style = field_label_style(is_selected, t);
        let value_style = Style::default().fg(t.foreground);
        lines.push(Line::from(vec![
            Span::styled(indicator(is_selected, &self.state.selector_indicator), label_style),
            Span::styled("Provider:  ", label_style),
            Span::styled(self.state.llm.provider.display_name(), value_style),
            if is_selected && !dropdown {
                Span::styled("  Enter to select", Style::default().fg(t.muted))
            } else {
                Span::raw("")
            },
        ]));

        // Provider dropdown
        if is_selected && dropdown {
            let providers = LlmProvider::all();
            let dd_indicator = format!("    {}", crate::ui::formatting::selector_prefix(true, &self.state.selector_indicator));
            for (i, p) in providers.iter().enumerate() {
                let is_dd_selected = i == self.state.settings.dropdown_selected;
                let dd_style = if is_dd_selected {
                    Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.foreground)
                };
                let prefix = if is_dd_selected { &dd_indicator } else { "       " };
                lines.push(Line::from(vec![
                    Span::raw(prefix.to_string()),
                    Span::styled(p.display_name().to_string(), dd_style),
                ]));
            }
        }

        lines.push(Line::from(""));

        // ── Model field (1) ──────────────────────────────────────────────
        let is_selected = selected == 1;
        let label_style = field_label_style(is_selected, t);
        let model_display = if self.state.llm.model.is_empty() {
            "(select a model)"
        } else {
            &self.state.llm.model
        };
        lines.push(Line::from(vec![
            Span::styled(indicator(is_selected, &self.state.selector_indicator), label_style),
            Span::styled("Model:     ", label_style),
            Span::styled(model_display, Style::default().fg(t.foreground)),
            if is_selected && !dropdown {
                Span::styled("  Enter to select", Style::default().fg(t.muted))
            } else {
                Span::raw("")
            },
        ]));

        // Model dropdown
        if is_selected && dropdown {
            let models = self.state.llm.provider.models();
            let dd_indicator = format!("    {}", crate::ui::formatting::selector_prefix(true, &self.state.selector_indicator));
            for (i, m) in models.iter().enumerate() {
                let is_dd_selected = i == self.state.settings.dropdown_selected;
                let dd_style = if is_dd_selected {
                    Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.foreground)
                };
                let prefix = if is_dd_selected { &dd_indicator } else { "       " };
                lines.push(Line::from(vec![
                    Span::raw(prefix.to_string()),
                    Span::styled(*m, dd_style),
                ]));
            }
        }

        lines.push(Line::from(""));

        // ── API Key field (2) ────────────────────────────────────────────
        let is_selected = selected == 2;
        let label_style = field_label_style(is_selected, t);
        let key_display = if is_selected && editing {
            format!("{}_", &self.state.settings.edit_buffer)
        } else {
            Self::mask_api_key(&self.state.llm.api_key)
        };
        let value_style = if is_selected && editing {
            Style::default().fg(t.success)
        } else {
            Style::default().fg(t.foreground)
        };
        lines.push(Line::from(vec![
            Span::styled(indicator(is_selected, &self.state.selector_indicator), label_style),
            Span::styled("API Key:   ", label_style),
            Span::styled(key_display, value_style),
            if is_selected && !editing {
                Span::styled("  Enter to edit", Style::default().fg(t.muted))
            } else {
                Span::raw("")
            },
        ]));

        lines.push(Line::from(""));

        // ── Base URL field (3) ───────────────────────────────────────────
        let is_selected = selected == 3;
        let label_style = field_label_style(is_selected, t);
        let url_display = if is_selected && editing {
            format!("{}_", &self.state.settings.edit_buffer)
        } else {
            self.state
                .llm
                .base_url
                .clone()
                .unwrap_or_else(|| "(default)".to_string())
        };
        let value_style = if is_selected && editing {
            Style::default().fg(t.success)
        } else {
            Style::default().fg(t.foreground)
        };
        lines.push(Line::from(vec![
            Span::styled(indicator(is_selected, &self.state.selector_indicator), label_style),
            Span::styled("Base URL:  ", label_style),
            Span::styled(url_display, value_style),
            if is_selected && !editing {
                Span::styled("  Enter to edit", Style::default().fg(t.muted))
            } else {
                Span::raw("")
            },
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("   {}", "─".repeat(56)),
            Style::default().fg(t.muted),
        )]));
        lines.push(Line::from(""));

        // Help line
        if dropdown {
            lines.push(Line::from(vec![
                Span::styled("   ↑/↓", Style::default().fg(t.primary)),
                Span::styled(" navigate  ", Style::default().fg(t.muted)),
                Span::styled("Enter", Style::default().fg(t.primary)),
                Span::styled(" select  ", Style::default().fg(t.muted)),
                Span::styled("Esc", Style::default().fg(t.primary)),
                Span::styled(" cancel", Style::default().fg(t.muted)),
            ]));
        } else if editing {
            lines.push(Line::from(vec![
                Span::styled("   Type to edit  ", Style::default().fg(t.muted)),
                Span::styled("Enter", Style::default().fg(t.primary)),
                Span::styled(" save  ", Style::default().fg(t.muted)),
                Span::styled("Esc", Style::default().fg(t.primary)),
                Span::styled(" cancel", Style::default().fg(t.muted)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("   ↑/↓", Style::default().fg(t.primary)),
                Span::styled(" navigate  ", Style::default().fg(t.muted)),
                Span::styled("Enter", Style::default().fg(t.primary)),
                Span::styled(" select/edit  ", Style::default().fg(t.muted)),
                Span::styled("Esc", Style::default().fg(t.primary)),
                Span::styled(" close & save", Style::default().fg(t.muted)),
            ]));
        }

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}

fn indicator(is_selected: bool, selector: &str) -> String {
    format!(" {}", crate::ui::formatting::selector_prefix(is_selected, selector))
}

fn field_label_style(is_selected: bool, t: &crate::config::theme::Theme) -> Style {
    if is_selected {
        Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.muted)
    }
}
