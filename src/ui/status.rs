//! Status line widgets with Rho theme styling.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::state::{AppState, VERSION};

/// Top status bar showing compact header
pub struct TopStatusBar<'a> {
    state: &'a AppState,
}

impl<'a> TopStatusBar<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    /// Get the last directory name from a path
    fn last_dir(path: &str) -> String {
        let path = path.trim_end_matches('/');
        if let Some(pos) = path.rfind('/') {
            format!("/{}", &path[pos + 1..])
        } else if path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path)
        }
    }
}

impl Widget for TopStatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < 10 {
            return;
        }

        let t = &self.state.theme;
        let mut spans = vec![];
        let yellow = Style::default().fg(t.primary);

        // Opening: ──┤
        spans.push(Span::styled("──┤ ", yellow));

        // "OH" in primary bold
        spans.push(Span::styled(
            "OH",
            Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(" │ ", yellow));

        // Workspace: "wrk /dirname"
        spans.push(Span::styled("wrk ", Style::default().fg(t.muted)));
        let dir_name = Self::last_dir(&self.state.workspace_path);
        spans.push(Span::styled(
            dir_name,
            Style::default().fg(t.foreground),
        ));

        spans.push(Span::styled(" │ ", yellow));

        // Session ID: "id xxxxxxxx"
        spans.push(Span::styled("id ", Style::default().fg(t.muted)));
        if let Some(id) = self.state.conversation_id {
            let id_str = id.to_string();
            let short_id = if id_str.len() >= 8 { &id_str[..8] } else { &id_str };
            spans.push(Span::styled(
                short_id.to_string(),
                Style::default().fg(t.accent),
            ));
        } else {
            spans.push(Span::styled(
                "---",
                Style::default().fg(t.muted),
            ));
        }

        // Closing: ├── and continue with line
        spans.push(Span::styled(" ├", yellow));

        // Calculate remaining space and fill with line
        let content_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let remaining = (area.width as usize).saturating_sub(content_len);

        if remaining > 0 {
            spans.push(Span::styled(
                "─".repeat(remaining),
                yellow,
            ));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }
}

/// Bottom status bar showing metrics and help
pub struct BottomStatusBar<'a> {
    state: &'a AppState,
}

impl<'a> BottomStatusBar<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn format_duration(seconds: u64) -> String {
        let mins = seconds / 60;
        let secs = seconds % 60;
        if mins > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}s", secs)
        }
    }

    fn format_cost(cost: f64) -> String {
        if cost < 0.01 {
            format!("${:.4}", cost)
        } else {
            format!("${:.2}", cost)
        }
    }

    fn format_tokens(tokens: u64) -> String {
        if tokens >= 1_000_000 {
            format!("{:.1}M", tokens as f64 / 1_000_000.0)
        } else if tokens >= 1_000 {
            format!("{:.1}k", tokens as f64 / 1_000.0)
        } else {
            tokens.to_string()
        }
    }
}

impl Widget for BottomStatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;

        // Fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_bg(t.background);
            }
        }

        let mut spans = vec![];

        // Policy indicator
        spans.push(Span::styled(
            "Policy: ",
            Style::default().fg(t.muted),
        ));

        let policy_color = match self.state.confirmation_policy {
            crate::state::ConfirmationPolicy::AlwaysConfirm => t.success,
            crate::state::ConfirmationPolicy::ConfirmRisky => t.primary,
            crate::state::ConfirmationPolicy::NeverConfirm => t.error,
        };
        spans.push(Span::styled(
            format!("{}", self.state.confirmation_policy),
            Style::default().fg(policy_color),
        ));
        spans.push(Span::styled(" | ", Style::default().fg(t.muted)));

        // Elapsed time (if running)
        if self.state.is_running() || self.state.elapsed_seconds > 0 {
            spans.push(Span::styled(
                format!("⏱ {}", Self::format_duration(self.state.elapsed_seconds)),
                Style::default().fg(t.primary),
            ));
            spans.push(Span::styled(" | ", Style::default().fg(t.muted)));
        }

        // Context usage bar
        let context_used = self.state.prompt_tokens;
        let context_max = self.state.context_window;
        let percentage = if context_max > 0 {
            (context_used as f64 / context_max as f64 * 100.0) as u8
        } else {
            0
        };

        let bar_color = if percentage < 65 {
            t.success
        } else if percentage < 75 {
            t.primary
        } else if percentage < 90 {
            Color::Rgb(255, 165, 0)  // Orange
        } else {
            t.error
        };

        let context_size = if context_max >= 1_000_000 {
            format!("{}M", context_max / 1_000_000)
        } else if context_max >= 1_000 {
            format!("{}k", context_max / 1_000)
        } else {
            format!("{}", context_max)
        };

        let bar_width = 10;
        let filled = (percentage as usize * bar_width / 100).min(bar_width);
        let empty = bar_width - filled;

        spans.push(Span::styled("Context: [", Style::default().fg(t.muted)));
        spans.push(Span::styled("█".repeat(filled), Style::default().fg(bar_color)));
        spans.push(Span::styled("░".repeat(empty), Style::default().fg(t.muted)));
        spans.push(Span::styled("] ", Style::default().fg(t.muted)));
        spans.push(Span::styled(
            format!("{}%", percentage),
            Style::default().fg(bar_color),
        ));
        spans.push(Span::styled(
            format!(" ({})", context_size),
            Style::default().fg(t.muted),
        ));
        spans.push(Span::styled(" | ", Style::default().fg(t.muted)));

        // Cost
        if self.state.total_cost > 0.0 {
            spans.push(Span::styled(
                Self::format_cost(self.state.total_cost),
                Style::default().fg(t.success),
            ));
            spans.push(Span::styled(" | ", Style::default().fg(t.muted)));
        }

        // Token metrics
        spans.push(Span::styled("↑ ", Style::default().fg(t.muted)));
        spans.push(Span::styled(
            Self::format_tokens(self.state.prompt_tokens),
            Style::default().fg(t.foreground),
        ));
        spans.push(Span::styled(" ↓ ", Style::default().fg(t.muted)));
        spans.push(Span::styled(
            Self::format_tokens(self.state.completion_tokens),
            Style::default().fg(t.foreground),
        ));

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }
}

/// Notification modal widget (centered, same style as other modals)
pub struct NotificationWidget<'a> {
    state: &'a AppState,
}

impl<'a> NotificationWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for NotificationWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::{Block, Borders, BorderType, Clear};

        if let Some(notif) = self.state.notifications.last() {
            use crate::state::NotificationSeverity;

            let t = &self.state.theme;

            let border_color = match notif.severity {
                NotificationSeverity::Info => t.accent,
                NotificationSeverity::Warning => t.primary,
                NotificationSeverity::Error => t.error,
            };

            let modal_width = 44.min(area.width.saturating_sub(4));
            let modal_height = 7.min(area.height.saturating_sub(4));

            let modal_x = area.x + (area.width.saturating_sub(modal_width)) / 2;
            let modal_y = area.y + (area.height.saturating_sub(modal_height)) / 2;

            let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

            Clear.render(modal_area, buf);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&notif.message, Style::default().fg(t.foreground)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Press ", Style::default().fg(t.muted)),
                Span::styled("any key", Style::default().fg(t.primary)),
                Span::styled(" to dismiss", Style::default().fg(t.muted)),
            ]));

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(
                    format!(" {} ", notif.title),
                    Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
                ));

            let inner = block.inner(modal_area);
            block.render(modal_area, buf);

            let paragraph = Paragraph::new(lines);
            paragraph.render(inner, buf);
        }
    }
}

/// Right border - draws a vertical line on the right edge
pub struct RightBorder;

impl Widget for RightBorder {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let x = area.x + area.width - 1;
        for y in area.y..area.y + area.height {
            buf[(x, y)].set_char('│').set_style(Style::default().fg(Color::Rgb(114, 121, 135)));
        }
    }
}
