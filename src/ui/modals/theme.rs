//! Theme picker modal widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::AppState;

pub struct ThemeModal<'a> {
    state: &'a AppState,
}

impl<'a> ThemeModal<'a> {
    const TITLE: &'static str = "Theme";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for ThemeModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let themes = &self.state.available_themes;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        for (i, theme_name) in themes.iter().enumerate() {
            let is_selected = i == self.state.theme_modal.selected;

            let indicator = format!(
                " {}",
                crate::ui::formatting::selector_prefix(is_selected, &self.state.selector_indicator)
            );
            let name_style = if is_selected {
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.foreground)
            };

            lines.push(Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(theme_name.clone(), name_style),
            ]));
        }

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  ↑/↓", Style::default().fg(t.primary)),
            Span::styled(" preview  ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" confirm  ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" cancel", Style::default().fg(t.muted)),
        ]));

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
