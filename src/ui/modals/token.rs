//! Token usage modal widget with tabs (Stats / Chart).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use super::tabs::build_tabbed_lines;
use crate::config::theme::Theme;
use crate::state::{AppState, MetricsState};

const TABS: &[&str] = &["Stats", "Chart"];

/// Token usage modal showing detailed metrics with two tabs: Stats and Chart.
pub struct TokenUsageModal<'a> {
    state: &'a AppState,
}

impl<'a> TokenUsageModal<'a> {
    const TITLE: &'static str = "Token Usage";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    /// Lines for the Stats tab (the original view).
    #[allow(clippy::vec_init_then_push)]
    fn stats_lines(metrics: &MetricsState, t: &Theme) -> Vec<Line<'static>> {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("  Total Tokens:     ", Style::default().fg(t.muted)),
            Span::styled(
                crate::ui::formatting::format_tokens_detailed(metrics.total_tokens),
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Prompt:           ", Style::default().fg(t.muted)),
            Span::styled(
                crate::ui::formatting::format_tokens_detailed(metrics.prompt_tokens),
                Style::default().fg(t.foreground),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Completion:       ", Style::default().fg(t.muted)),
            Span::styled(
                crate::ui::formatting::format_tokens_detailed(metrics.completion_tokens),
                Style::default().fg(t.foreground),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Reasoning:        ", Style::default().fg(t.muted)),
            Span::styled(
                crate::ui::formatting::format_tokens_detailed(metrics.reasoning_tokens),
                Style::default().fg(t.foreground),
            ),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Cache Read:       ", Style::default().fg(t.muted)),
            Span::styled(
                crate::ui::formatting::format_tokens_detailed(metrics.cache_read_tokens),
                Style::default().fg(t.success),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Cache Write:      ", Style::default().fg(t.muted)),
            Span::styled(
                crate::ui::formatting::format_tokens_detailed(metrics.cache_write_tokens),
                Style::default().fg(t.foreground),
            ),
        ]));

        let cache_rate = if metrics.prompt_tokens > 0 {
            format!(
                "{:.0}%",
                metrics.cache_read_tokens as f64 / metrics.prompt_tokens as f64 * 100.0
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
                crate::ui::formatting::format_cost(metrics.total_cost),
                Style::default().fg(t.success).add_modifier(Modifier::BOLD),
            ),
        ]));

        let ctx_pct = if metrics.context_window > 0 {
            format!(
                "{:.0}% of {}",
                metrics.per_turn_tokens as f64 / metrics.context_window as f64 * 100.0,
                crate::ui::formatting::format_tokens_detailed(metrics.context_window),
            )
        } else {
            "—".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("  Context Usage:    ", Style::default().fg(t.muted)),
            Span::styled(ctx_pct, Style::default().fg(t.foreground)),
        ]));

        lines
    }

    /// Lines for the Chart tab — horizontal bar chart showing token breakdown.
    fn chart_lines(metrics: &MetricsState, t: &Theme) -> Vec<Line<'static>> {
        let mut lines: Vec<Line> = Vec::new();
        const BAR_WIDTH: usize = 40;

        // Categories to display in the chart
        let rows: Vec<(&str, u64, Style)> = vec![
            (
                "Prompt     ",
                metrics.prompt_tokens,
                Style::default().fg(t.primary),
            ),
            (
                "Completion ",
                metrics.completion_tokens,
                Style::default().fg(t.accent),
            ),
            (
                "Reasoning  ",
                metrics.reasoning_tokens,
                Style::default().fg(t.foreground),
            ),
            (
                "Cache Read ",
                metrics.cache_read_tokens,
                Style::default().fg(t.success),
            ),
            (
                "Cache Write",
                metrics.cache_write_tokens,
                Style::default().fg(t.muted),
            ),
        ];

        let max = rows.iter().map(|(_, v, _)| *v).max().unwrap_or(0).max(1);

        lines.push(Line::from(""));

        for (label, value, style) in &rows {
            let filled = ((*value as f64 / max as f64) * BAR_WIDTH as f64).round() as usize;
            let filled = filled.min(BAR_WIDTH);
            let empty = BAR_WIDTH - filled;

            let value_str = crate::ui::formatting::format_tokens_detailed(*value);

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", label), Style::default().fg(t.muted)),
                Span::styled("█".repeat(filled), *style),
                Span::styled("░".repeat(empty), Style::default().fg(t.border)),
                Span::styled(
                    format!("  {}", value_str),
                    Style::default().fg(t.foreground),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", "─".repeat(BAR_WIDTH + 20)),
            Style::default().fg(t.muted),
        )]));
        lines.push(Line::from(""));

        // Total at the bottom
        lines.push(Line::from(vec![
            Span::styled("  Total:       ", Style::default().fg(t.muted)),
            Span::styled(
                crate::ui::formatting::format_tokens_detailed(metrics.total_tokens),
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Total Cost:  ", Style::default().fg(t.muted)),
            Span::styled(
                crate::ui::formatting::format_cost(metrics.total_cost),
                Style::default().fg(t.success).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines
    }
}

impl Widget for TokenUsageModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let metrics = &self.state.metrics;

        let panes = vec![Self::stats_lines(metrics, t), Self::chart_lines(metrics, t)];

        let footer = vec![Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Tab", Style::default().fg(t.primary)),
            Span::styled("/", Style::default().fg(t.muted)),
            Span::styled("←→", Style::default().fg(t.primary)),
            Span::styled(" switch tab  ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" close", Style::default().fg(t.muted)),
        ])];

        let lines = build_tabbed_lines(TABS, self.state.token_modal_tab, panes, footer, t);
        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
