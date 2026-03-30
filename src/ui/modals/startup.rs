//! Startup modal displayed while the agent server is initializing.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::AppState;

const SPINNER_FRAMES: &[&str] = &["   ", ".  ", ".. ", "..."];

pub struct StartupModal<'a> {
    state: &'a AppState,
}

impl<'a> StartupModal<'a> {
    const TITLE: &'static str = "Rho";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for StartupModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
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

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
