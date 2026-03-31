//! Message display widget with Rho theme styling.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget},
};
use textwrap::wrap;

use crate::config::theme::{rho_banner, Theme};
use crate::events::SecurityRisk;
use crate::state::{AppState, DisplayMessage, MessageRole, VERSION};
use crate::ui::markdown::render_markdown;

/// Message list widget showing conversation history
pub struct MessageListWidget<'a> {
    state: &'a AppState,
}

impl<'a> MessageListWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn role_style(role: MessageRole, t: &Theme) -> Style {
        match role {
            MessageRole::User => Style::default().fg(t.primary),
            MessageRole::Assistant => Style::default().fg(t.foreground),
            MessageRole::System => Style::default().fg(t.muted),
            MessageRole::Action => Style::default().fg(t.primary),
            MessageRole::Error => Style::default().fg(t.error).add_modifier(Modifier::BOLD),
            MessageRole::Terminal => Style::default().fg(t.accent),
        }
    }

    fn security_risk_style(risk: SecurityRisk, t: &Theme) -> Style {
        match risk {
            SecurityRisk::Unknown => Style::default().fg(t.muted),
            SecurityRisk::Low => Style::default().fg(t.success),
            SecurityRisk::Medium => Style::default().fg(t.primary),
            SecurityRisk::High => Style::default().fg(t.error).add_modifier(Modifier::BOLD),
        }
    }

    /// Format tool content nicely for expanded view
    fn format_tool_content(content: &str, width: usize, t: &Theme) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(obj) = json.as_object() {
                for (key, value) in obj {
                    let key_line = Line::from(vec![
                        Span::raw("   "),
                        Span::styled(
                            format!("{}:", key),
                            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                        ),
                    ]);
                    lines.push(key_line);

                    let value_lines = Self::format_json_value(value, width.saturating_sub(6), 6, t);
                    lines.extend(value_lines);
                }
            } else {
                let value_lines = Self::format_json_value(&json, width.saturating_sub(4), 4, t);
                lines.extend(value_lines);
            }
        } else {
            let wrapped = wrap(content, width);
            for line in wrapped {
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::raw(line.to_string()),
                ]));
            }
        }

        lines
    }

    /// Format a JSON value with proper indentation
    fn format_json_value(
        value: &serde_json::Value,
        width: usize,
        indent: usize,
        t: &Theme,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let indent_str = " ".repeat(indent);

        match value {
            serde_json::Value::String(s) => {
                if s.len() > width {
                    let wrapped = wrap(s, width);
                    for line in wrapped {
                        lines.push(Line::from(vec![
                            Span::raw(indent_str.clone()),
                            Span::styled(line.to_string(), Style::default().fg(t.foreground)),
                        ]));
                    }
                } else {
                    lines.push(Line::from(vec![
                        Span::raw(indent_str),
                        Span::styled(s.clone(), Style::default().fg(t.foreground)),
                    ]));
                }
            }
            serde_json::Value::Number(n) => {
                lines.push(Line::from(vec![
                    Span::raw(indent_str),
                    Span::styled(n.to_string(), Style::default().fg(t.primary)),
                ]));
            }
            serde_json::Value::Bool(b) => {
                lines.push(Line::from(vec![
                    Span::raw(indent_str),
                    Span::styled(b.to_string(), Style::default().fg(t.primary)),
                ]));
            }
            serde_json::Value::Null => {
                lines.push(Line::from(vec![
                    Span::raw(indent_str),
                    Span::styled("null", Style::default().fg(t.muted)),
                ]));
            }
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    lines.push(Line::from(vec![
                        Span::raw(indent_str.clone()),
                        Span::styled(format!("[{}]", i), Style::default().fg(t.muted)),
                    ]));
                    let item_lines =
                        Self::format_json_value(item, width.saturating_sub(2), indent + 2, t);
                    lines.extend(item_lines);
                }
            }
            serde_json::Value::Object(obj) => {
                for (key, val) in obj {
                    lines.push(Line::from(vec![
                        Span::raw(indent_str.clone()),
                        Span::styled(format!("{}: ", key), Style::default().fg(t.accent)),
                    ]));

                    match val {
                        serde_json::Value::String(s) if s.len() < width / 2 => {
                            if let Some(last) = lines.last_mut() {
                                last.spans.push(Span::styled(
                                    s.clone(),
                                    Style::default().fg(t.foreground),
                                ));
                            }
                        }
                        serde_json::Value::Number(n) => {
                            if let Some(last) = lines.last_mut() {
                                last.spans.push(Span::styled(
                                    n.to_string(),
                                    Style::default().fg(t.primary),
                                ));
                            }
                        }
                        serde_json::Value::Bool(b) => {
                            if let Some(last) = lines.last_mut() {
                                last.spans.push(Span::styled(
                                    b.to_string(),
                                    Style::default().fg(t.primary),
                                ));
                            }
                        }
                        _ => {
                            let val_lines = Self::format_json_value(
                                val,
                                width.saturating_sub(2),
                                indent + 2,
                                t,
                            );
                            lines.extend(val_lines);
                        }
                    }
                }
            }
        }

        lines
    }

    fn format_message(msg: &DisplayMessage, width: usize, t: &Theme) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let style = Self::role_style(msg.role, t);

        match msg.role {
            MessageRole::User => {
                let content_width = width.saturating_sub(2);
                let wrapped = wrap(&msg.content, content_width);
                for (i, line) in wrapped.iter().enumerate() {
                    if i == 0 {
                        lines.push(Line::from(vec![
                            Span::styled("> ", style),
                            Span::styled(line.to_string(), style),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(line.to_string(), style),
                        ]));
                    }
                }
            }
            MessageRole::Assistant => {
                let content_width = width.saturating_sub(2);
                let md_lines = render_markdown(&msg.content, content_width, t);
                for md_line in md_lines {
                    let mut indented_spans = vec![Span::raw(" ")];
                    indented_spans.extend(md_line.spans);
                    lines.push(Line::from(indented_spans));
                }
            }
            MessageRole::Action => {
                // Content format: "args_display\nsummary"
                // tool_name and security_risk are in separate fields
                let (args_line, summary_line) =
                    msg.content.split_once('\n').unwrap_or((&msg.content, ""));

                // First line: tool_name(args) RISK
                let mut header_spans = vec![];

                if msg.accepted {
                    header_spans.push(Span::styled("✓ ", Style::default().fg(t.success)));
                }

                // Tool name in accent/blue + bold
                let tool_name = msg.tool_name.as_deref().unwrap_or("Action");
                header_spans.push(Span::styled(
                    tool_name.to_string(),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                ));

                // Arguments in grey
                if !args_line.is_empty() {
                    header_spans.push(Span::styled(
                        format!("({})", args_line),
                        Style::default().fg(t.muted),
                    ));
                }

                // Security risk in its own color
                if let Some(risk) = msg.security_risk {
                    header_spans.push(Span::raw(" "));
                    header_spans.push(Span::styled(
                        format!("{}", risk),
                        Self::security_risk_style(risk, t),
                    ));
                }

                lines.push(Line::from(header_spans));

                // Second line: ⎿  summary
                if msg.collapsed {
                    if !summary_line.is_empty() {
                        lines.push(Line::from(vec![
                            Span::styled("  ⎿  ", Style::default().fg(t.muted)),
                            Span::styled(summary_line.to_string(), Style::default().fg(t.muted)),
                        ]));
                    }
                } else {
                    let formatted_lines =
                        Self::format_tool_content(&msg.content, width.saturating_sub(6), t);
                    for formatted_line in formatted_lines {
                        let mut new_spans =
                            vec![Span::styled("  ⎿  ", Style::default().fg(t.muted))];
                        new_spans.extend(formatted_line.spans);
                        lines.push(Line::from(new_spans));
                    }
                }
            }
            MessageRole::System => {
                for line in msg.content.lines() {
                    lines.push(Line::from(vec![Span::styled(line.to_string(), style)]));
                }
            }
            MessageRole::Error => {
                let content_width = width.saturating_sub(2);
                let wrapped = wrap(&msg.content, content_width);
                for (i, line) in wrapped.iter().enumerate() {
                    if i == 0 {
                        lines.push(Line::from(vec![
                            Span::styled("✗ ", style),
                            Span::styled(line.to_string(), style),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(line.to_string(), style),
                        ]));
                    }
                }
            }
            MessageRole::Terminal => {
                // Content format: "$ command\noutput"
                let (cmd_line, output) = msg.content.split_once('\n').unwrap_or((&msg.content, ""));

                // Header: command line
                lines.push(Line::from(vec![
                    Span::styled("┌─ ", Style::default().fg(t.accent)),
                    Span::styled(
                        cmd_line.to_string(),
                        Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                    ),
                ]));

                // Output lines — show all
                for line in output.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(t.accent)),
                        Span::styled(line.to_string(), Style::default().fg(t.muted)),
                    ]));
                }

                // Footer
                lines.push(Line::from(vec![Span::styled(
                    "└─",
                    Style::default().fg(t.accent),
                )]));
            }
        }

        lines.push(Line::from(""));
        lines
    }
}

impl Widget for MessageListWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = Rect::new(
            area.x + 1,
            area.y,
            area.width.saturating_sub(2),
            area.height,
        );

        let t = &self.state.theme;
        let mut all_lines: Vec<Line> = Vec::new();
        let content_width = inner_area.width.saturating_sub(2) as usize;
        let is_running = self.state.is_running();

        // Always show the banner at the top
        all_lines.extend(build_banner_lines(self.state, t));

        for msg in self.state.messages.iter() {
            let msg_lines = Self::format_message(msg, content_width, t);
            all_lines.extend(msg_lines);
        }

        if is_running {
            let spinner = self.state.spinner_frame();
            let fun_fact = self.state.current_fun_fact();
            let tick = self.state.spinner_tick;

            let mut spans = vec![Span::styled(
                format!("  {}  ", spinner),
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            )];
            spans.extend(crate::config::theme::animated_thinking_spans(
                fun_fact, tick, t,
            ));

            all_lines.push(Line::from(spans));
            all_lines.push(Line::from(""));
        }

        let visible_height = inner_area.height as usize;
        let total_lines = all_lines.len();
        let max_scroll = total_lines.saturating_sub(visible_height);

        let scroll_from_top = max_scroll.saturating_sub(self.state.scroll_offset);
        let scroll_offset = scroll_from_top.min(max_scroll);

        let visible_lines: Vec<Line> = all_lines
            .into_iter()
            .skip(scroll_offset)
            .take(visible_height)
            .collect();

        let paragraph = Paragraph::new(Text::from(visible_lines));
        paragraph.render(inner_area, buf);
    }
}

/// Build the banner lines (logo + info on the right)
fn build_banner_lines<'a>(state: &AppState, t: &Theme) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    let banner = rho_banner(VERSION);
    let banner_width = banner.iter().map(|s| s.chars().count()).max().unwrap_or(20);

    // Right-side info lines — each is a Vec<Span> to preserve styling
    let bullet = Span::styled("• ", Style::default().fg(t.muted));
    let info_lines: Vec<Vec<Span>> = vec![
        vec![
            bullet.clone(),
            Span::styled("wkr: ", Style::default().fg(t.muted)),
            Span::styled(
                super::path_utils::truncate_path(&state.workspace_path),
                Style::default().fg(t.foreground),
            ),
        ],
        vec![
            bullet,
            Span::styled("Type ", Style::default().fg(t.muted)),
            Span::styled("/help", Style::default().fg(t.primary)),
            Span::styled(" for commands", Style::default().fg(t.muted)),
        ],
        vec![],
        vec![Span::styled(
            "Time to build something awesome",
            Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
        )],
    ];

    lines.push(Line::from(""));

    let version_suffix = format!("v{}", VERSION);

    for (i, banner_line) in banner.iter().enumerate() {
        let padding = banner_width.saturating_sub(banner_line.chars().count());
        let mut spans = vec![Span::styled("    ", Style::default())];

        // Split off the version suffix so it can be styled differently
        if let Some(pos) = banner_line.find(&version_suffix) {
            let logo_part = &banner_line[..pos];
            spans.push(Span::styled(
                logo_part.to_string(),
                Style::default().fg(t.primary),
            ));
            spans.push(Span::styled(
                version_suffix.clone(),
                Style::default().fg(t.muted),
            ));
        } else {
            spans.push(Span::styled(
                banner_line.clone(),
                Style::default().fg(t.primary),
            ));
        }

        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled("  │  ", Style::default().fg(t.muted)));

        if let Some(info_spans) = info_lines.get(i) {
            spans.extend(info_spans.iter().cloned());
        }

        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines
}
