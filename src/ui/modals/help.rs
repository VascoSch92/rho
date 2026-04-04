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
            Span::styled("  /resume    ", Style::default().fg(t.primary)),
            Span::styled(
                "Resume a previous conversation",
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

/// Policy modal — navigate with ↑/↓, Enter to apply, Esc to cancel
pub struct PolicyModal<'a> {
    state: &'a AppState,
}

impl<'a> PolicyModal<'a> {
    const TITLE: &'static str = "Confirmation Policy";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

const POLICIES: &[ConfirmationPolicy] = &[
    ConfirmationPolicy::AlwaysConfirm,
    ConfirmationPolicy::ConfirmRisky,
    ConfirmationPolicy::NeverConfirm,
];

fn policy_color(
    policy: ConfirmationPolicy,
    t: &crate::config::theme::Theme,
) -> ratatui::style::Color {
    match policy {
        ConfirmationPolicy::AlwaysConfirm => t.success,
        ConfirmationPolicy::ConfirmRisky => t.primary,
        ConfirmationPolicy::NeverConfirm => t.error,
    }
}

fn policy_description(policy: ConfirmationPolicy) -> &'static str {
    match policy {
        ConfirmationPolicy::AlwaysConfirm => "   Confirm all actions",
        ConfirmationPolicy::ConfirmRisky => "   Only risky actions",
        ConfirmationPolicy::NeverConfirm => "   Auto-approve all",
    }
}

impl Widget for PolicyModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        for (i, policy) in POLICIES.iter().enumerate() {
            let is_selected = i == self.state.policy_selected;
            let is_current = *policy == self.state.confirmation_policy;
            let color = policy_color(*policy, t);

            let indicator = if is_selected { " ▶ " } else { "   " };
            let name_style = if is_selected {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.foreground)
            };

            let mut spans = vec![
                Span::styled(indicator, name_style),
                Span::styled(format!("{:<16}", policy), name_style),
                Span::styled(policy_description(*policy), Style::default().fg(t.muted)),
            ];

            if is_current {
                spans.push(Span::styled("  (current)", Style::default().fg(t.muted)));
            }

            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  ↑/↓", Style::default().fg(t.primary)),
            Span::styled(" navigate  ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" apply  ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" cancel", Style::default().fg(t.muted)),
        ]));

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
