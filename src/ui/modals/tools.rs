//! Tools modal — lists available tools from the agent server.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::AppState;

pub struct ToolsModal<'a> {
    state: &'a AppState,
}

impl<'a> ToolsModal<'a> {
    const TITLE: &'static str = "Available Tools";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for ToolsModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        if self.state.tools_list.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  No tools available.",
                Style::default().fg(t.muted),
            )]));
        } else {
            for tool in &self.state.tools_list {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        self.state.selector_indicator.clone(),
                        Style::default().fg(t.accent),
                    ),
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        tool.clone(),
                        Style::default()
                            .fg(t.foreground)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" close", Style::default().fg(t.muted)),
        ]));

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
