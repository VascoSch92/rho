//! Confirmation panel widget with Rho theme.
//! Design matches the Token Usage modal.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::events::SecurityRisk;
use crate::state::{AppState, PendingAction};
use crate::ui::theme::Theme;

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

        // Modal dimensions (same as token modal)
        let modal_width = 55.min(area.width.saturating_sub(4));
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

        // Header text
        lines.push(Line::from(vec![Span::styled(
            "  Action requiring confirmation:",
            Style::default().fg(t.muted),
        )]));

        lines.push(Line::from(""));

        // Actions (show first one, or multiple)
        for action in self.state.pending_actions.iter().take(3) {
            lines.push(Self::format_action_line(action, t));
            // Summary on next line
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

        // Divider
        let divider_width = (modal_width as usize).saturating_sub(6);
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", "─".repeat(divider_width)),
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

        // Close hint (same style as token modal)
        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(t.muted)),
            Span::styled("← →", Style::default().fg(t.primary)),
            Span::styled(" to select, ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" to confirm", Style::default().fg(t.muted)),
        ]));

        // Create block with rounded corners (same as token modal)
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.accent))
            .title(Span::styled(
                " Confirm Action ",
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

/// Exit confirmation modal (same design as token modal)
pub struct ExitConfirmationModal<'a> {
    pub show: bool,
    pub state: &'a AppState,
}

impl Widget for ExitConfirmationModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.show {
            return;
        }

        let t = &self.state.theme;

        let modal_width = 45;
        let modal_height = 9;

        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

        Clear.render(modal_area, buf);

        // Build content
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Are you sure you want to exit?",
                Style::default().fg(t.foreground),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Press ", Style::default().fg(t.muted)),
                Span::styled(
                    "Y",
                    Style::default().fg(t.error).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to exit, ", Style::default().fg(t.muted)),
                Span::styled(
                    "N",
                    Style::default().fg(t.success).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to stay", Style::default().fg(t.muted)),
            ]),
        ];

        // Create block with rounded corners (same as token modal)
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.accent))
            .title(Span::styled(
                " Exit ",
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        let paragraph = Paragraph::new(text);
        paragraph.render(inner, buf);
    }
}
