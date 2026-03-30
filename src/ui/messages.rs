//! Message display widget with Rho theme styling.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget},
};
use textwrap::wrap;

use crate::events::SecurityRisk;
use crate::state::{AppState, DisplayMessage, MessageRole, VERSION};
use crate::ui::markdown::render_markdown;
use crate::ui::theme::{Theme, RHO_BANNER};

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

    /// Extract a preview from content (for collapsed view)
    fn extract_preview(content: &str, max_len: usize) -> String {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(cmd) = json.get("command").and_then(|v| v.as_str()) {
                let preview: String = cmd.chars().take(max_len).collect();
                let ellipsis = if cmd.len() > max_len { "..." } else { "" };
                return format!("{}{}", preview.replace('\n', " "), ellipsis);
            }
            if let Some(path) = json.get("path").and_then(|v| v.as_str()) {
                return format!("path: {}", path);
            }
            if let Some(text) = json.get("text").and_then(|v| v.as_str()) {
                let preview: String = text.chars().take(max_len - 6).collect();
                let ellipsis = if text.len() > max_len - 6 { "..." } else { "" };
                return format!("text: {}{}", preview.replace('\n', " "), ellipsis);
            }
        }

        let preview: String = content.chars().take(max_len).collect();
        let ellipsis = if content.len() > max_len { "..." } else { "" };
        format!("{}{}", preview.replace('\n', " "), ellipsis)
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
                let mut header_spans = vec![];

                header_spans.push(Span::styled("│ ", Style::default().fg(t.muted)));

                if msg.accepted {
                    header_spans.push(Span::styled("✓ ", Style::default().fg(t.success)));
                }

                if let Some(ref tool) = msg.tool_name {
                    header_spans.push(Span::styled(
                        tool.clone(),
                        Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                    ));
                } else {
                    let label = if msg.role == MessageRole::Action {
                        "Action"
                    } else {
                        "Result"
                    };
                    header_spans.push(Span::styled(label.to_string(), style));
                }

                if let Some(risk) = msg.security_risk {
                    header_spans.push(Span::raw(" "));
                    header_spans.push(Span::styled(
                        format!("[{}]", risk),
                        Self::security_risk_style(risk, t),
                    ));
                }

                lines.push(Line::from(header_spans));

                if msg.collapsed {
                    let preview = Self::extract_preview(&msg.content, 70);
                    lines.push(Line::from(vec![
                        Span::styled("│   ", Style::default().fg(t.muted)),
                        Span::styled(preview, Style::default().fg(t.muted)),
                    ]));
                } else {
                    let formatted_lines =
                        Self::format_tool_content(&msg.content, width.saturating_sub(4), t);
                    for formatted_line in formatted_lines {
                        let mut new_spans =
                            vec![Span::styled("│   ", Style::default().fg(t.muted))];
                        new_spans.extend(formatted_line.spans);
                        lines.push(Line::from(new_spans));
                    }
                }
                lines.push(Line::from(vec![Span::styled(
                    "│",
                    Style::default().fg(t.muted),
                )]));
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

        if self.state.messages.is_empty() {
            render_splash(inner_area, buf, self.state);
            return;
        }

        let t = &self.state.theme;
        let mut all_lines: Vec<Line> = Vec::new();
        let content_width = inner_area.width.saturating_sub(2) as usize;
        let is_running = self.state.is_running();

        all_lines.push(Line::from(""));

        for msg in self.state.messages.iter() {
            let msg_lines = Self::format_message(msg, content_width, t);
            all_lines.extend(msg_lines);
        }

        if is_running {
            let spinner = self.state.spinner_frame();
            let fun_fact = self.state.current_fun_fact();

            all_lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}  ", spinner),
                    Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Thinking...", Style::default().fg(t.primary)),
            ]));
            all_lines.push(Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled(
                    fun_fact.to_string(),
                    Style::default().fg(t.muted).add_modifier(Modifier::ITALIC),
                ),
            ]));
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

/// Render the Rho splash screen
fn render_splash(area: Rect, buf: &mut Buffer, state: &AppState) {
    let t = &state.theme;
    let mut lines: Vec<Line> = Vec::new();

    let banner_width = RHO_BANNER.iter().map(|s| s.len()).max().unwrap_or(60);
    let box_width = banner_width + 4;

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        format!("    ╭{}╮", "─".repeat(box_width)),
        Style::default().fg(t.primary),
    )]));

    lines.push(Line::from(vec![
        Span::styled("    │", Style::default().fg(t.primary)),
        Span::raw(" ".repeat(box_width)),
        Span::styled("│", Style::default().fg(t.primary)),
    ]));

    for banner_line in RHO_BANNER.iter() {
        let padding = box_width.saturating_sub(banner_line.len());
        lines.push(Line::from(vec![
            Span::styled("    │", Style::default().fg(t.primary)),
            Span::styled(format!(" {}", banner_line), Style::default().fg(t.primary)),
            Span::raw(" ".repeat(padding.saturating_sub(1))),
            Span::styled("│", Style::default().fg(t.primary)),
        ]));
    }

    let version_text = format!("Rho v{}", VERSION);
    let version_padding = box_width.saturating_sub(version_text.len() + 2);
    lines.push(Line::from(vec![
        Span::styled("    │", Style::default().fg(t.primary)),
        Span::styled(format!(" {}", version_text), Style::default().fg(t.muted)),
        Span::raw(" ".repeat(version_padding + 1)),
        Span::styled("│", Style::default().fg(t.primary)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("    │", Style::default().fg(t.primary)),
        Span::raw(" ".repeat(box_width)),
        Span::styled("│", Style::default().fg(t.primary)),
    ]));

    lines.push(Line::from(vec![Span::styled(
        format!("    ╰{}╯", "─".repeat(box_width)),
        Style::default().fg(t.primary),
    )]));

    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled("Workspace: ", Style::default().fg(t.muted)),
        Span::styled(
            truncate_path(&state.workspace_path, 50),
            Style::default().fg(t.foreground),
        ),
    ]));

    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "    Tips:",
        Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![
        Span::styled("      • ", Style::default().fg(t.muted)),
        Span::styled(
            "Ask questions, edit files, or run commands",
            Style::default().fg(t.foreground),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("      • ", Style::default().fg(t.muted)),
        Span::styled("Use ", Style::default().fg(t.foreground)),
        Span::styled("@", Style::default().fg(t.primary)),
        Span::styled(" to reference files", Style::default().fg(t.foreground)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("      • ", Style::default().fg(t.muted)),
        Span::styled("Type ", Style::default().fg(t.foreground)),
        Span::styled("/help", Style::default().fg(t.primary)),
        Span::styled(" for available commands", Style::default().fg(t.foreground)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("      • ", Style::default().fg(t.muted)),
        Span::styled("Press ", Style::default().fg(t.foreground)),
        Span::styled("Ctrl+Q", Style::default().fg(t.primary)),
        Span::styled(" to quit", Style::default().fg(t.foreground)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "    What do you want to build?",
        Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
    )]));

    let paragraph = Paragraph::new(lines);
    paragraph.render(area, buf);
}

/// Truncate path keeping the end visible
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else if max_len < 4 {
        "...".to_string()
    } else {
        format!("...{}", &path[path.len().saturating_sub(max_len - 3)..])
    }
}
