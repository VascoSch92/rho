//! Startup modal displayed while the agent server is initializing.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::state::AppState;

const SPINNER_FRAMES: &[&str] = &["   ", ".  ", ".. ", "..."];

pub struct StartupModal<'a> {
    state: &'a AppState,
}

impl<'a> StartupModal<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for StartupModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;

        let modal_width = 44.min(area.width.saturating_sub(4));
        let modal_height = 7.min(area.height.saturating_sub(4));

        let modal_x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = area.y + (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

        Clear.render(modal_area, buf);

        let dots = SPINNER_FRAMES[self.state.server_starting_tick % SPINNER_FRAMES.len()];

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Preparing everything",
                    Style::default()
                        .fg(t.foreground)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(dots, Style::default().fg(t.accent)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Starting Agent Server",
                Style::default().fg(t.muted),
            )]),
            Line::from(""),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.primary))
            .title(" Rho ");

        let paragraph = Paragraph::new(lines).block(block);
        paragraph.render(modal_area, buf);
    }
}
