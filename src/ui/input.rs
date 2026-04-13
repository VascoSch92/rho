//! Input field widget with Rho theme styling.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::state::AppState;

/// Input field widget - simple two yellow lines
pub struct InputWidget<'a> {
    state: &'a AppState,
}

impl<'a> InputWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for InputWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }

        let t = &self.state.theme;
        let is_terminal_mode = self.state.input_buffer.starts_with('!');
        let border_color = if is_terminal_mode {
            t.accent
        } else {
            t.primary
        };

        // Draw top line
        let top_y = area.y;
        for x in area.x..area.x + area.width {
            buf[(x, top_y)].set_char('─');
            buf[(x, top_y)].set_fg(border_color);
        }

        // Show "Bash Mode" label on the left of the top line
        if is_terminal_mode {
            let label = " Bash Mode ";
            for (i, ch) in label.chars().enumerate() {
                let x = area.x + i as u16;
                if x < area.x + area.width {
                    buf[(x, top_y)].set_char(ch);
                    buf[(x, top_y)].set_style(
                        ratatui::style::Style::default()
                            .fg(t.accent)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    );
                }
            }
        }

        // Draw bottom line
        let bottom_y = area.y + area.height - 1;
        for x in area.x..area.x + area.width {
            buf[(x, bottom_y)].set_char('─');
            buf[(x, bottom_y)].set_fg(border_color);
        }

        // Input area is between the lines
        let input_area = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );

        if input_area.height == 0 || input_area.width == 0 {
            return;
        }

        let input = &self.state.input_buffer;
        let cursor_pos = self.state.cursor_position;

        // Check if input has newlines (multiline content)
        if input.contains('\n') {
            // Multiline rendering
            let lines: Vec<&str> = input.split('\n').collect();
            let mut rendered_lines: Vec<Line> = Vec::new();
            let mut char_count = 0usize;

            for (line_idx, line_text) in lines.iter().enumerate() {
                let line_start = char_count;
                let line_end = char_count + line_text.len();

                if cursor_pos >= line_start && cursor_pos <= line_end {
                    // Cursor is on this line
                    let pos_in_line = cursor_pos - line_start;
                    let before = &line_text[..pos_in_line];
                    let cursor_char = line_text.chars().nth(pos_in_line).unwrap_or(' ');
                    let after = if pos_in_line < line_text.len() {
                        &line_text[pos_in_line + cursor_char.len_utf8()..]
                    } else {
                        ""
                    };

                    rendered_lines.push(Line::from(vec![
                        Span::styled(before, Style::default().fg(t.foreground)),
                        Span::styled(
                            cursor_char.to_string(),
                            Style::default()
                                .fg(t.primary)
                                .add_modifier(Modifier::UNDERLINED)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(after, Style::default().fg(t.foreground)),
                    ]));
                } else {
                    rendered_lines.push(Line::from(Span::styled(
                        *line_text,
                        Style::default().fg(t.foreground),
                    )));
                }

                char_count = line_end + 1; // +1 for the newline

                // Only render lines that fit
                if line_idx as u16 >= input_area.height {
                    break;
                }
            }

            let paragraph = Paragraph::new(rendered_lines);
            paragraph.render(input_area, buf);
        } else {
            // Single line rendering
            let before_cursor = &input[..cursor_pos];
            let cursor_char = input.chars().nth(cursor_pos).unwrap_or(' ');
            let after_cursor = if cursor_pos < input.len() {
                &input[cursor_pos + cursor_char.len_utf8()..]
            } else {
                ""
            };

            let mut spans = vec![
                Span::styled(before_cursor, Style::default().fg(t.foreground)),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(t.primary)
                        .add_modifier(Modifier::UNDERLINED)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(after_cursor, Style::default().fg(t.foreground)),
            ];

            if let Some(hint) = command_param_hint(input) {
                spans.push(Span::styled(
                    format!(" {}", hint),
                    Style::default().fg(t.muted).add_modifier(Modifier::ITALIC),
                ));
            }

            let paragraph = Paragraph::new(Line::from(spans));
            paragraph.render(input_area, buf);
        }
    }
}

/// Returns a muted hint describing the parameter a slash command expects,
/// shown after the typed command while no argument has been entered yet.
fn command_param_hint(input: &str) -> Option<&'static str> {
    let rest = input.strip_prefix('/')?;
    // Split into command and whatever follows. We only want to show the hint
    // when the user has the command fully typed but hasn't started the arg.
    let (cmd, after) = match rest.find(' ') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };
    if !after.is_empty() {
        return None;
    }
    match cmd.to_lowercase().as_str() {
        "confirm" => Some("[always|never|risky]"),
        "theme" => Some("[theme name]"),
        "btw" => Some("<question>"),
        "rename" => Some("<new name>"),
        _ => None,
    }
}

/// Calculate the number of lines needed for the input widget
pub fn input_height(state: &AppState) -> u16 {
    // Count newlines in input buffer
    let line_count = state.input_buffer.matches('\n').count() + 1;
    // Minimum 3 (top border + 1 line + bottom border), max 12
    (line_count as u16 + 2).clamp(3, 12)
}
