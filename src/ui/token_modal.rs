//! Token usage modal widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::state::AppState;

/// Token usage modal showing detailed metrics
pub struct TokenUsageModal<'a> {
    state: &'a AppState,
}

impl<'a> TokenUsageModal<'a> {
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

        // Modal dimensions
        let modal_width = 50.min(area.width.saturating_sub(4));
        let modal_height = 14.min(area.height.saturating_sub(4));

        // Center the modal
        let modal_x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = area.y + (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

        // Clear the area behind the modal
        Clear.render(modal_area, buf);

        // Build content
        let mut lines: Vec<Line> = Vec::new();

        // Empty line for padding
        lines.push(Line::from(""));

        // Total tokens
        lines.push(Line::from(vec![
            Span::styled("  Total Tokens:     ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_tokens(self.state.total_tokens),
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));

        // Breakdown
        lines.push(Line::from(vec![
            Span::styled("  Prompt Tokens:    ", Style::default().fg(t.muted)),
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

        lines.push(Line::from(""));

        // Divider
        let divider_width = (modal_width as usize).saturating_sub(6);
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", "─".repeat(divider_width)),
            Style::default().fg(t.muted),
        )]));

        lines.push(Line::from(""));

        // Cost
        lines.push(Line::from(vec![
            Span::styled("  Total Cost:       ", Style::default().fg(t.muted)),
            Span::styled(
                Self::format_cost(self.state.total_cost),
                Style::default().fg(t.success).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(""));

        // Close hint
        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" or ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" to close", Style::default().fg(t.muted)),
        ]));

        // Create block with rounded corners
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.accent))
            .title(Span::styled(
                " Token Usage ",
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
