//! Token usage modal widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::AppState;

/// Token usage modal showing detailed metrics
pub struct TokenUsageModal<'a> {
    state: &'a AppState,
}

impl<'a> TokenUsageModal<'a> {
    const TITLE: &'static str = "Token Usage";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn format_tokens(tokens: u64) -> String {
        if tokens >= 1_000_000 {
            format!("{:.2}M", tokens as f64 / 1_000_000.0)
        } else if tokens >= 1_000 {
            format!("{:.1}k", tokens as f64 / 1_000.0)
        } else {
            format!("{}", tokens)
        }
    }

    fn format_cost(cost: f64) -> String {
        if cost < 0.001 {
            format!("${:.6}", cost)
        } else if cost < 0.01 {
            format!("${:.4}", cost)
        } else {
            format!("${:.2}", cost)
        }
    }
}

#[allow(clippy::vec_init_then_push)]
impl Widget for TokenUsageModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Total Tokens:     ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_tokens(self.state.total_tokens),
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Prompt:           ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_tokens(self.state.prompt_tokens),
                Style::default().fg(t.foreground),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Completion:       ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_tokens(self.state.completion_tokens),
                Style::default().fg(t.foreground),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Reasoning:        ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_tokens(self.state.reasoning_tokens),
                Style::default().fg(t.foreground),
            ),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Cache Read:       ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_tokens(self.state.cache_read_tokens),
                Style::default().fg(t.success),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Cache Write:      ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_tokens(self.state.cache_write_tokens),
                Style::default().fg(t.foreground),
            ),
        ]));

        // Cache hit rate
        let cache_rate = if self.state.prompt_tokens > 0 {
            format!(
                "{:.0}%",
                self.state.cache_read_tokens as f64 / self.state.prompt_tokens as f64 * 100.0
            )
        } else {
            "—".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("  Cache Hit Rate:   ", Style::default().fg(t.muted)),
            Span::styled(cache_rate, Style::default().fg(t.success)),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", "─".repeat(44)),
            Style::default().fg(t.muted),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Total Cost:       ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_cost(self.state.total_cost),
                Style::default().fg(t.success).add_modifier(Modifier::BOLD),
            ),
        ]));

        // Context utilization (current turn)
        let ctx_pct = if self.state.context_window > 0 {
            format!(
                "{:.0}% of {}",
                self.state.per_turn_tokens as f64 / self.state.context_window as f64 * 100.0,
                Self::format_tokens(self.state.context_window),
            )
        } else {
            "—".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("  Context Usage:    ", Style::default().fg(t.muted)),
            Span::styled(ctx_pct, Style::default().fg(t.foreground)),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" or ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" to close", Style::default().fg(t.muted)),
        ]));

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
