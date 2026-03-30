//! Settings modal widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::AppState;

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
        } else if key.len() <= 8 {
            "*".repeat(key.len())
        } else {
            format!("{}...{}", &key[..4], &key[key.len() - 4..])
        }
    }
}

impl Widget for SettingsModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();
        let selected = self.state.settings_field;
        let editing = self.state.settings_editing;

        lines.push(Line::from(""));

        // Provider field (0)
        let is_selected = selected == 0;
        let label_style = if is_selected {
            Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.muted)
        };
        let value_style = if is_selected && editing {
            Style::default().fg(t.success).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.foreground)
        };
        lines.push(Line::from(vec![
            Span::styled(if is_selected { " ▶ " } else { "   " }, label_style),
            Span::styled("Provider:  ", label_style),
            Span::styled(self.state.llm_provider.display_name(), value_style),
            if is_selected && !editing {
                Span::styled("  ←/→ to change", Style::default().fg(t.muted))
            } else {
                Span::raw("")
            },
        ]));

        lines.push(Line::from(""));

        // Model field (1)
        let is_selected = selected == 1;
        let label_style = if is_selected {
            Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.muted)
        };
        let value_style = if is_selected && editing {
            Style::default().fg(t.success).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.foreground)
        };
        let model_display = if self.state.llm_model.is_empty() {
            "(select a model)"
        } else {
            &self.state.llm_model
        };
        lines.push(Line::from(vec![
            Span::styled(if is_selected { " ▶ " } else { "   " }, label_style),
            Span::styled("Model:     ", label_style),
            Span::styled(model_display, value_style),
            if is_selected && !editing {
                Span::styled("  ←/→ to change", Style::default().fg(t.muted))
            } else {
                Span::raw("")
            },
        ]));

        lines.push(Line::from(""));

        // API Key field (2)
        let is_selected = selected == 2;
        let label_style = if is_selected {
            Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.muted)
        };
        let key_display = if is_selected && editing {
            format!("{}_", &self.state.settings_edit_buffer)
        } else {
            Self::mask_api_key(&self.state.llm_api_key)
        };
        let value_style = if is_selected && editing {
            Style::default().fg(t.success)
        } else {
            Style::default().fg(t.foreground)
        };
        lines.push(Line::from(vec![
            Span::styled(if is_selected { " ▶ " } else { "   " }, label_style),
            Span::styled("API Key:   ", label_style),
            Span::styled(key_display, value_style),
            if is_selected && !editing {
                Span::styled("  Enter to edit", Style::default().fg(t.muted))
            } else {
                Span::raw("")
            },
        ]));

        lines.push(Line::from(""));

        // Base URL field (3)
        let is_selected = selected == 3;
        let label_style = if is_selected {
            Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.muted)
        };
        let url_display = if is_selected && editing {
            format!("{}_", &self.state.settings_edit_buffer)
        } else {
            self.state
                .llm_base_url
                .clone()
                .unwrap_or_else(|| "(default)".to_string())
        };
        let value_style = if is_selected && editing {
            Style::default().fg(t.success)
        } else {
            Style::default().fg(t.foreground)
        };
        lines.push(Line::from(vec![
            Span::styled(if is_selected { " ▶ " } else { "   " }, label_style),
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

        lines.push(Line::from(vec![
            Span::styled("   Models for ", Style::default().fg(t.muted)),
            Span::styled(
                self.state.llm_provider.display_name(),
                Style::default().fg(t.accent),
            ),
            Span::styled(":", Style::default().fg(t.muted)),
        ]));

        let models = self.state.llm_provider.models();
        let model_list: String = models
            .iter()
            .take(4)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if models.len() > 4 {
            format!(" (+{})", models.len() - 4)
        } else {
            String::new()
        };
        lines.push(Line::from(vec![Span::styled(
            format!("   {}{}", model_list, suffix),
            Style::default().fg(t.muted),
        )]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("   ↑/↓", Style::default().fg(t.primary)),
            Span::styled(" navigate  ", Style::default().fg(t.muted)),
            Span::styled("←/→", Style::default().fg(t.primary)),
            Span::styled(" change  ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" edit  ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" close", Style::default().fg(t.muted)),
        ]));

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
