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

    /// Format a group of consecutive Action messages into bordered boxes.
    /// Actions are split into sub-groups (batches) whenever an action carries
    /// a `thought` — the thought text is displayed between batches.
    /// Each batch gets a `┌─  Tool calls: N` header.
    fn format_action_group(
        actions: &[&DisplayMessage],
        width: usize,
        t: &Theme,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Split into sub-groups: a new sub-group starts when an action has a thought.
        let mut sub_groups: Vec<(Option<&str>, Vec<&DisplayMessage>)> = Vec::new();
        for action in actions {
            if action.thought.is_some() || sub_groups.is_empty() {
                sub_groups.push((action.thought.as_deref(), vec![action]));
            } else {
                sub_groups.last_mut().unwrap().1.push(action);
            }
        }

        for (thought, group) in &sub_groups {
            // Show thought/reasoning before this batch
            if let Some(thought_text) = thought {
                lines.push(Line::from(""));
                let content_width = width.saturating_sub(4);
                let wrapped = wrap(thought_text, content_width);
                for line in wrapped.iter() {
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(
                            line.to_string(),
                            Style::default()
                                .fg(t.foreground)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
                lines.push(Line::from(""));
            }

            // Check if entire group is collapsed (all actions have collapsed=true)
            let group_collapsed = group.iter().all(|m| m.collapsed);
            let badge_style = Style::default().fg(t.foreground).bg(t.border);

            if group_collapsed {
                // Collapsed: badge + hint
                lines.push(Line::from(vec![
                    Span::styled("┌─ ", Style::default().fg(t.accent)),
                    Span::styled("[ ", badge_style),
                    Span::styled(format!("Tool calls: {}", group.len()), badge_style),
                    Span::styled(" ]", badge_style),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("│ ", Style::default().fg(t.accent)),
                    Span::styled("...", Style::default().fg(t.muted)),
                ]));
                lines.push(Line::from(vec![Span::styled(
                    "└─",
                    Style::default().fg(t.accent),
                )]));
            } else {
                // Expanded: header + all tool entries
                lines.push(Line::from(vec![
                    Span::styled("┌─ ", Style::default().fg(t.accent)),
                    Span::styled("[ ", badge_style),
                    Span::styled(format!("Tool calls: {}", group.len()), badge_style),
                    Span::styled(" ]", badge_style),
                ]));

                for msg in group {
                    let mut header_spans = vec![Span::styled("├─ ", Style::default().fg(t.accent))];

                    if msg.accepted {
                        header_spans.push(Span::styled("✓ ", Style::default().fg(t.success)));
                    }

                    let tool_name = msg.tool_name.as_deref().unwrap_or("Action");
                    header_spans.push(Span::styled(
                        tool_name.to_string(),
                        Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                    ));

                    if let Some(risk) = msg.security_risk {
                        if risk != SecurityRisk::Unknown {
                            header_spans.push(Span::raw(" "));
                            header_spans.push(Span::styled(
                                format!("{}", risk),
                                Self::security_risk_style(risk, t),
                            ));
                        }
                    }

                    lines.push(Line::from(header_spans));

                    // Content format: "args_display\nsummary"
                    let (args_line, summary_line) =
                        msg.content.split_once('\n').unwrap_or((&msg.content, ""));

                    if !args_line.is_empty() {
                        let args_width = width.saturating_sub(4);
                        let wrapped = wrap(args_line, args_width);
                        for wl in wrapped.iter() {
                            lines.push(Line::from(vec![
                                Span::styled("│ ", Style::default().fg(t.accent)),
                                Span::styled(wl.to_string(), Style::default().fg(t.foreground)),
                            ]));
                        }
                    }
                    if !summary_line.is_empty() {
                        let summary_width = width.saturating_sub(4);
                        let wrapped = wrap(summary_line, summary_width);
                        for wl in wrapped.iter() {
                            lines.push(Line::from(vec![
                                Span::styled("│ ", Style::default().fg(t.accent)),
                                Span::styled(
                                    wl.to_string(),
                                    Style::default().fg(t.muted).add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                        }
                    }
                }

                // Batch footer
                lines.push(Line::from(vec![Span::styled(
                    "└─",
                    Style::default().fg(t.accent),
                )]));
            }
        }

        lines.push(Line::from(""));
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
                // Consecutive actions are merged by format_action_group in render().
                // This fallback handles any Action that might be called individually.
                let group = [msg];
                let mut action_lines = Self::format_action_group(&group, width, t);
                // Remove the trailing empty line — format_message adds its own
                if action_lines.last().is_some_and(|l| l.spans.is_empty()) {
                    action_lines.pop();
                }
                lines.extend(action_lines);
            }
            MessageRole::System => {
                for line in msg.content.lines() {
                    lines.push(Line::from(vec![Span::styled(line.to_string(), style)]));
                }
            }
            MessageRole::Error => {
                let content_width = width.saturating_sub(2);
                // Split error code from detail (separated by newline)
                let (error_code, detail) =
                    msg.content.split_once('\n').unwrap_or((&msg.content, ""));

                // Error code line: bold red with ✗ prefix
                let wrapped_code = wrap(error_code, content_width);
                for (i, line) in wrapped_code.iter().enumerate() {
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

                // Detail lines: muted style, indented
                if !detail.is_empty() {
                    let detail_style = Style::default().fg(t.foreground);
                    let wrapped_detail = wrap(detail, content_width);
                    for line in wrapped_detail.iter() {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(line.to_string(), detail_style),
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

                // Output lines — show all, wrapped to width
                let output_width = width.saturating_sub(4);
                for line in output.lines() {
                    let wrapped = wrap(line, output_width);
                    for wl in wrapped.iter() {
                        lines.push(Line::from(vec![
                            Span::styled("│ ", Style::default().fg(t.accent)),
                            Span::styled(wl.to_string(), Style::default().fg(t.muted)),
                        ]));
                    }
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

        // Always show the banner at the top
        all_lines.extend(build_banner_lines(self.state, t, content_width));

        // Group consecutive Action messages together
        let messages = &self.state.messages;
        let mut i = 0;
        while i < messages.len() {
            if messages[i].role == MessageRole::Action {
                // Collect consecutive Action messages
                let mut action_group: Vec<&DisplayMessage> = Vec::new();
                while i < messages.len() && messages[i].role == MessageRole::Action {
                    action_group.push(&messages[i]);
                    i += 1;
                }
                let group_lines = Self::format_action_group(&action_group, content_width, t);
                all_lines.extend(group_lines);
            } else {
                let msg_lines = Self::format_message(&messages[i], content_width, t);
                all_lines.extend(msg_lines);
                i += 1;
            }
        }

        // Spinner is rendered separately in the layout, not in the message list

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

/// Build the banner lines (logo + info on the right, or stacked when narrow)
fn build_banner_lines<'a>(state: &AppState, t: &Theme, content_width: usize) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    let banner = rho_banner(VERSION);
    let banner_width = banner.iter().map(|s| s.chars().count()).max().unwrap_or(20);

    // Right-side info lines — each is a Vec<Span> to preserve styling
    let bullet = Span::styled("• ", Style::default().fg(t.muted));

    let id_display = if let Some(id) = state.conversation_id {
        let id_str = id.to_string();
        let short_id = if id_str.len() >= 8 {
            id_str[..8].to_string()
        } else {
            id_str
        };
        Span::styled(short_id, Style::default().fg(t.accent))
    } else {
        Span::styled("---", Style::default().fg(t.muted))
    };

    let info_lines: Vec<Vec<Span>> = vec![
        vec![
            bullet.clone(),
            Span::styled("wkr: ", Style::default().fg(t.muted)),
            Span::styled(
                super::formatting::truncate_path(&state.workspace_path),
                Style::default().fg(t.foreground),
            ),
        ],
        vec![
            bullet.clone(),
            Span::styled("id:  ", Style::default().fg(t.muted)),
            id_display,
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

    // Minimum width needed for side-by-side layout:
    //   4 (left pad) + banner_width + 5 ("  │  ") + ~20 (info) = ~50+
    let min_side_by_side = banner_width + 4 + 5 + 20;
    let side_by_side = content_width >= min_side_by_side;

    if side_by_side {
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
    } else {
        // Narrow terminal: stack logo above info
        for banner_line in &banner {
            let mut spans = vec![Span::styled("  ", Style::default())];

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

            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));

        for info_spans in &info_lines {
            if info_spans.is_empty() {
                continue;
            }
            let mut spans = vec![Span::styled("  ", Style::default())];
            spans.extend(info_spans.iter().cloned());
            lines.push(Line::from(spans));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines
}
