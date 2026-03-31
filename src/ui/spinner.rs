//! Spinner widget displayed above the input bar while the agent is thinking.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::state::AppState;

pub struct SpinnerWidget<'a> {
    state: &'a AppState,
}

impl<'a> SpinnerWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for SpinnerWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.state.is_running() || area.height == 0 {
            return;
        }

        let t = &self.state.theme;
        let spinner = self.state.spinner_frame();
        let fun_fact = self.state.current_fun_fact();
        let tick = self.state.spinner_tick;

        let mut spans = vec![Span::styled(
            format!(" {}  ", spinner),
            Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
        )];
        spans.extend(crate::config::theme::animated_thinking_spans(
            fun_fact, tick, t,
        ));

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }
}

/// Returns the height needed for the spinner (1 if running, 0 otherwise)
pub fn spinner_height(state: &AppState) -> u16 {
    if state.is_running() {
        1
    } else {
        0
    }
}
