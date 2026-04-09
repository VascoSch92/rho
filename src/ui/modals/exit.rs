//! Exit confirmation modal widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal_centered;
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
        let selected = self.state.exit_confirmation_selected;

        // 0 = No, 1 = Yes
        let yes_selected = selected == 1;
        let no_selected = selected == 0;

        // Pill style (like Stats/Chart tabs): selected has the border bg + bold.
        // Inactive is muted. Selected Yes keeps its red fg, selected No keeps green.
        let yes_style = if yes_selected {
            Style::default()
                .fg(t.error)
                .bg(t.border)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.muted)
        };
        let no_style = if no_selected {
            Style::default()
                .fg(t.success)
                .bg(t.border)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.muted)
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Are you sure you want to exit?",
                Style::default().fg(t.foreground),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Yes ", yes_style),
                Span::styled("   ", Style::default()),
                Span::styled(" No ", no_style),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("←→", Style::default().fg(t.primary)),
                Span::styled(" select  ", Style::default().fg(t.muted)),
                Span::styled("Enter", Style::default().fg(t.primary)),
                Span::styled(" confirm  ", Style::default().fg(t.muted)),
                Span::styled("Y", Style::default().fg(t.primary)),
                Span::styled("/", Style::default().fg(t.muted)),
                Span::styled("N", Style::default().fg(t.primary)),
                Span::styled(" shortcut", Style::default().fg(t.muted)),
            ]),
        ];

        render_modal_centered(area, buf, Self::TITLE, lines, t);
    }
}
