//! Help and policy modal widgets.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::{AppState, ConfirmationPolicy};

/// Help modal showing available commands
pub struct HelpModal<'a> {
    state: &'a AppState,
}

impl<'a> HelpModal<'a> {
    const TITLE: &'static str = "Help";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for HelpModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  /help      ", Style::default().fg(t.primary)),
            Span::styled("Show this help", Style::default().fg(t.foreground)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  /new       ", Style::default().fg(t.primary)),
            Span::styled(
                "Start a new conversation",
                Style::default().fg(t.foreground),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  /usage     ", Style::default().fg(t.primary)),
            Span::styled(
                "Show token usage details",
                Style::default().fg(t.foreground),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  /settings  ", Style::default().fg(t.primary)),
            Span::styled("Show current settings", Style::default().fg(t.foreground)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  /theme     ", Style::default().fg(t.primary)),
            Span::styled("Change color theme", Style::default().fg(t.foreground)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  /confirm   ", Style::default().fg(t.primary)),
            Span::styled(
                "Show/change confirmation policy",
                Style::default().fg(t.foreground),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  /exit      ", Style::default().fg(t.primary)),
            Span::styled("Exit the application", Style::default().fg(t.foreground)),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  !<cmd>     ", Style::default().fg(t.primary)),
            Span::styled(
                "Run bash command (e.g. !ls, !pwd)",
                Style::default().fg(t.foreground),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", "─".repeat(44)),
            Style::default().fg(t.muted),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Alt+Enter  ", Style::default().fg(t.accent)),
            Span::styled(
                "New line in input (or Shift+Enter)",
                Style::default().fg(t.muted),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Ctrl+Q     ", Style::default().fg(t.accent)),
            Span::styled("Quit", Style::default().fg(t.muted)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ↑↓ PgUp/Dn ", Style::default().fg(t.accent)),
            Span::styled("Scroll messages", Style::default().fg(t.muted)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Ctrl+E     ", Style::default().fg(t.accent)),
            Span::styled("Expand/collapse all actions", Style::default().fg(t.muted)),
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

/// Policy modal showing current confirmation policy and options
pub struct PolicyModal<'a> {
    state: &'a AppState,
}

impl<'a> PolicyModal<'a> {
    const TITLE: &'static str = "Confirmation Policy";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn policy_color(&self, policy: ConfirmationPolicy) -> ratatui::style::Color {
        let t = &self.state.theme;
        match policy {
            ConfirmationPolicy::AlwaysConfirm => t.success,
            ConfirmationPolicy::ConfirmRisky => t.primary,
            ConfirmationPolicy::NeverConfirm => t.error,
        }
    }
}

impl Widget for PolicyModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Current:  ", Style::default().fg(t.muted)),
            Span::styled(
                format!("{}", self.state.confirmation_policy),
                Style::default()
                    .fg(self.policy_color(self.state.confirmation_policy))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", "─".repeat(42)),
            Style::default().fg(t.muted),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  /confirm always  ", Style::default().fg(t.success)),
            Span::styled("Confirm all actions", Style::default().fg(t.muted)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  /confirm risky   ", Style::default().fg(t.primary)),
            Span::styled("Only risky actions", Style::default().fg(t.muted)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  /confirm never   ", Style::default().fg(t.error)),
            Span::styled("Auto-approve all", Style::default().fg(t.muted)),
        ]));

        lines.push(Line::from(""));
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
