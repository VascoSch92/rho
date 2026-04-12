//! Task list widget — a persistent panel showing tasks from the task_tracker tool.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::state::AppState;

/// Height of the task list panel (including border lines).
/// Returns 0 when there are no tasks.
pub fn task_list_height(state: &AppState) -> u16 {
    if state.tasks.is_empty() || !state.tasks_visible {
        0
    } else {
        1 + state.tasks.len() as u16 // top rule + tasks
    }
}

/// Task list panel widget.
pub struct TaskListWidget<'a> {
    state: &'a AppState,
}

impl<'a> TaskListWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for TaskListWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.state.tasks.is_empty() || !self.state.tasks_visible || area.height < 2 {
            return;
        }

        let t = &self.state.theme;
        let inner = Rect::new(
            area.x + 1,
            area.y,
            area.width.saturating_sub(2),
            area.height,
        );

        let mut lines: Vec<Line> = Vec::new();

        // Top rule with title and close hint
        let hint = "(ctrl+t to close)";
        let title = "Tasks";
        let rule_width = (inner.width as usize).saturating_sub(title.len() + 3 + hint.len() + 4);
        lines.push(Line::from(vec![
            Span::styled("── ", Style::default().fg(t.border)),
            Span::styled(
                title,
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(hint, Style::default().fg(t.muted)),
            Span::styled(" ", Style::default()),
            Span::styled("─".repeat(rule_width), Style::default().fg(t.border)),
        ]));

        // Task rows
        for task in &self.state.tasks {
            let (checkbox, checkbox_style) = match task.status.as_str() {
                "done" => ("[x]", Style::default().fg(t.success)),
                "in_progress" => ("[>]", Style::default().fg(t.primary)),
                _ => ("[ ]", Style::default().fg(t.muted)),
            };

            let title_style = if task.status == "done" {
                Style::default()
                    .fg(t.muted)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else if task.status == "in_progress" {
                Style::default()
                    .fg(t.foreground)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.foreground)
            };

            let mut spans = vec![
                Span::styled(format!(" {} ", checkbox), checkbox_style),
                Span::styled(task.title.clone(), title_style),
            ];

            if !task.notes.is_empty() && task.status != "done" {
                spans.push(Span::styled(
                    format!("  {}", task.notes),
                    Style::default().fg(t.muted),
                ));
            }

            lines.push(Line::from(spans));
        }

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
