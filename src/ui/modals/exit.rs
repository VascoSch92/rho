//! Exit confirmation modal widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::AppState;

pub struct ExitConfirmationModal<'a> {
    pub show: bool,
    pub state: &'a AppState,
}

impl ExitConfirmationModal<'_> {
    const TITLE: &'static str = "Exit";
}

impl Widget for ExitConfirmationModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.show {
            return;
        }

        let t = &self.state.theme;

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Are you sure you want to exit?",
                Style::default().fg(t.foreground),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Press ", Style::default().fg(t.muted)),
                Span::styled(
                    "Y",
                    Style::default().fg(t.error).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to exit, ", Style::default().fg(t.muted)),
                Span::styled(
                    "N",
                    Style::default().fg(t.success).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to stay", Style::default().fg(t.muted)),
            ]),
        ];

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
