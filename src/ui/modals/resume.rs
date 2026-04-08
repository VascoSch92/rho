//! Resume conversation modal — list, select, resume, or delete conversations.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use crate::state::AppState;

pub struct ResumeModal<'a> {
    state: &'a AppState,
}

impl<'a> ResumeModal<'a> {
    const TITLE: &'static str = "Resume Conversation";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    /// Format a datetime string to a short display form.
    fn format_date(iso: &str) -> String {
        // "2026-03-31T02:44:15.261005Z" → "2026-03-31 02:44"
        if iso.len() >= 16 {
            iso[..16].replace('T', " ")
        } else {
            iso.to_string()
        }
    }

    /// Truncate a string to max chars, adding "..." if truncated.
    fn truncate(s: &str, max: usize) -> String {
        if s.chars().count() <= max {
            s.to_string()
        } else {
            let end = s
                .char_indices()
                .nth(max.saturating_sub(3))
                .map(|(i, _)| i)
                .unwrap_or(s.len());
            format!("{}...", &s[..end])
        }
    }
}

impl Widget for ResumeModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let convos = &self.state.resume_conversations;
        let mut lines: Vec<Line> = Vec::new();

        if convos.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  No conversations found.",
                Style::default().fg(t.muted),
            )]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Press ", Style::default().fg(t.muted)),
                Span::styled("Esc", Style::default().fg(t.primary)),
                Span::styled(" to close", Style::default().fg(t.muted)),
            ]));
        } else {
            lines.push(Line::from(""));

            // Show up to 10 conversations, scrolling around the selection
            let max_visible = 10;
            let total = convos.len();
            let selected = self.state.resume_selected;

            let start = if total <= max_visible || selected < max_visible / 2 {
                0
            } else if selected + max_visible / 2 >= total {
                total.saturating_sub(max_visible)
            } else {
                selected.saturating_sub(max_visible / 2)
            };
            let end = (start + max_visible).min(total);

            for (i, conv) in convos.iter().enumerate().take(end).skip(start) {
                let is_selected = i == selected;

                let indicator = format!(" {}", crate::ui::formatting::selector_prefix(is_selected, &self.state.selector_indicator));

                let title = Self::truncate(&conv.title, 35);
                let first_msg = if conv.first_message.is_empty() {
                    String::new()
                } else {
                    format!(" \"{}\"", Self::truncate(&conv.first_message, 10))
                };

                let short_id = if conv.id.len() >= 8 {
                    &conv.id[..8]
                } else {
                    &conv.id
                };

                let name_style = if is_selected {
                    Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.foreground)
                };

                // Line 1: indicator + title + first message preview
                let mut line1_spans = vec![
                    Span::styled(indicator, name_style),
                    Span::styled(title, name_style),
                ];
                if !first_msg.is_empty() {
                    line1_spans.push(Span::styled(
                        first_msg,
                        Style::default().fg(t.muted).add_modifier(Modifier::ITALIC),
                    ));
                }
                lines.push(Line::from(line1_spans));

                // Line 2: id (always accent, matching banner) + date
                let id_style = Style::default().fg(t.accent);
                lines.push(Line::from(vec![
                    Span::raw("     "),
                    Span::styled(short_id.to_string(), id_style),
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        Self::format_date(&conv.updated_at),
                        Style::default().fg(t.muted),
                    ),
                ]));
            }

            // Scroll indicator
            if total > max_visible {
                lines.push(Line::from(vec![Span::styled(
                    format!("   ({}/{})", selected + 1, total),
                    Style::default().fg(t.muted),
                )]));
            }

            lines.push(Line::from(""));

            // Delete confirmation or help line
            if self.state.resume_confirm_delete {
                lines.push(Line::from(vec![
                    Span::styled("  Delete this conversation? ", Style::default().fg(t.error)),
                    Span::styled(
                        "y",
                        Style::default().fg(t.error).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("/", Style::default().fg(t.muted)),
                    Span::styled(
                        "n",
                        Style::default().fg(t.success).add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ↑/↓", Style::default().fg(t.primary)),
                    Span::styled(" select  ", Style::default().fg(t.muted)),
                    Span::styled("Enter", Style::default().fg(t.primary)),
                    Span::styled(" resume  ", Style::default().fg(t.muted)),
                    Span::styled("d", Style::default().fg(t.error)),
                    Span::styled(" delete  ", Style::default().fg(t.muted)),
                    Span::styled("Esc", Style::default().fg(t.primary)),
                    Span::styled(" cancel", Style::default().fg(t.muted)),
                ]));
            }
        }

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
