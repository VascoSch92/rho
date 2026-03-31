//! Confirmation panel for pending agent actions.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::config::theme::Theme;
use crate::events::SecurityRisk;
use crate::state::{AppState, PendingAction};

/// Confirmation options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmOption {
    Accept,
    AlwaysAccept,
    Reject,
}

impl ConfirmOption {
    pub fn label(&self) -> &'static str {
        match self {
            ConfirmOption::Accept => "Accept",
            ConfirmOption::AlwaysAccept => "Always Accept",
            ConfirmOption::Reject => "Reject",
        }
    }

    pub fn all() -> &'static [ConfirmOption] {
        &[
            ConfirmOption::Accept,
            ConfirmOption::AlwaysAccept,
            ConfirmOption::Reject,
        ]
    }
}

/// Confirmation panel for pending actions
pub struct ConfirmationPanel<'a> {
    state: &'a AppState,
}

impl<'a> ConfirmationPanel<'a> {
    const TITLE: &'static str = "Confirm Action";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn risk_style(risk: SecurityRisk, t: &Theme) -> Style {
        match risk {
            SecurityRisk::Unknown => Style::default().fg(t.muted),
            SecurityRisk::Low => Style::default().fg(t.success),
            SecurityRisk::Medium => Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            SecurityRisk::High => Style::default().fg(t.error).add_modifier(Modifier::BOLD),
        }
    }

    fn format_action_line(action: &PendingAction, t: &Theme) -> Line<'static> {
        let risk_text = format!("[{}]", action.security_risk);
        Line::from(vec![
            Span::styled("  ▶ ", Style::default().fg(t.muted)),
            Span::styled(
                action.tool_name.clone(),
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(risk_text, Self::risk_style(action.security_risk, t)),
        ])
    }
}

impl Widget for ConfirmationPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.state.pending_actions.is_empty() {
            return;
        }

        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        lines.push(Line::from(vec![Span::styled(
            "  Action requiring confirmation:",
            Style::default().fg(t.muted),
        )]));

        lines.push(Line::from(""));

        for action in self.state.pending_actions.iter().take(3) {
            lines.push(Self::format_action_line(action, t));
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    action.summary.chars().take(45).collect::<String>(),
                    Style::default().fg(t.foreground),
                ),
            ]));
        }

        if self.state.pending_actions.len() > 3 {
            lines.push(Line::from(vec![Span::styled(
                format!("  ... and {} more", self.state.pending_actions.len() - 3),
                Style::default().fg(t.muted),
            )]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", "─".repeat(49)),
            Style::default().fg(t.muted),
        )]));
        lines.push(Line::from(""));

        // Options with selection indicator
        let options = ConfirmOption::all();
        let selected = self.state.confirmation_selected;

        let mut option_spans: Vec<Span> = vec![Span::raw("  ")];
        for (i, opt) in options.iter().enumerate() {
            if i > 0 {
                option_spans.push(Span::styled("  │  ", Style::default().fg(t.muted)));
            }

            let is_selected = i == selected;
            let style = if is_selected {
                match opt {
                    ConfirmOption::Accept => {
                        Style::default().fg(t.success).add_modifier(Modifier::BOLD)
                    }
                    ConfirmOption::AlwaysAccept => {
                        Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
                    }
                    ConfirmOption::Reject => {
                        Style::default().fg(t.error).add_modifier(Modifier::BOLD)
                    }
                }
            } else {
                Style::default().fg(t.muted)
            };

            let prefix = if is_selected { "► " } else { "  " };
            option_spans.push(Span::styled(format!("{}{}", prefix, opt.label()), style));
        }

        lines.push(Line::from(option_spans));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(t.muted)),
            Span::styled("← →", Style::default().fg(t.primary)),
            Span::styled(" to select, ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" to confirm", Style::default().fg(t.muted)),
        ]));

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
