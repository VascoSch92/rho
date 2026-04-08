//! Command menu popup for slash commands.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::state::AppState;

/// Available slash commands
pub const COMMANDS: &[(&str, &str)] = &[
    ("help", "Show available commands"),
    ("new", "Start a new conversation"),
    ("resume", "Resume a previous conversation"),
    ("usage", "Show token usage"),
    ("settings", "Show current settings"),
    ("theme", "Change color theme"),
    ("rename", "Rename the conversation"),
    ("confirm", "Change confirmation policy"),
    ("exit", "Exit the application"),
];

/// Command menu widget
pub struct CommandMenuWidget<'a> {
    state: &'a AppState,
}

impl<'a> CommandMenuWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    /// Get filtered commands based on current input
    fn filtered_commands(&self) -> Vec<(&'static str, &'static str)> {
        let input = &self.state.input_buffer;

        // Get the command part after /
        let filter = input.strip_prefix('/').unwrap_or("");

        if filter.is_empty() {
            // Show all commands
            COMMANDS.to_vec()
        } else {
            // Filter commands that start with the input
            COMMANDS
                .iter()
                .filter(|(cmd, _)| cmd.starts_with(&filter.to_lowercase()))
                .copied()
                .collect()
        }
    }
}

impl Widget for CommandMenuWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let commands = self.filtered_commands();

        if commands.is_empty() {
            return;
        }

        let t = &self.state.theme;

        // Calculate menu size
        let menu_height = (commands.len() + 2).min(10) as u16; // +2 for border
        let menu_width = 55.min(area.width.saturating_sub(4));

        // Position above the input area
        let menu_x = area.x + 1;
        let menu_y = area.height.saturating_sub(menu_height + 4); // 4 for input area

        let menu_area = Rect::new(menu_x, menu_y, menu_width, menu_height);

        // Clear the area behind the menu
        Clear.render(menu_area, buf);

        // Build menu content
        let mut lines: Vec<Line> = Vec::new();

        for (i, (cmd, desc)) in commands.iter().enumerate() {
            let is_selected = i == self.state.command_menu_selected % commands.len();

            let style = if is_selected {
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.foreground)
            };

            let prefix = super::formatting::selector_prefix(is_selected, &self.state.selector_indicator);

            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("/{:<10}", cmd), style),
                Span::styled(desc.to_string(), Style::default().fg(t.muted)),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border))
            .title(Span::styled(" Commands ", Style::default().fg(t.accent)));

        let paragraph = Paragraph::new(lines).block(block);
        paragraph.render(menu_area, buf);
    }
}

/// Get the number of available commands (filtered)
pub fn command_count(state: &AppState) -> usize {
    let input = &state.input_buffer;
    let filter = input.strip_prefix('/').unwrap_or("");

    if filter.is_empty() {
        COMMANDS.len()
    } else {
        COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(&filter.to_lowercase()))
            .count()
    }
}

/// Get the selected command name
pub fn selected_command(state: &AppState) -> Option<&'static str> {
    let input = &state.input_buffer;
    let filter = input.strip_prefix('/').unwrap_or("");

    let commands: Vec<_> = if filter.is_empty() {
        COMMANDS.to_vec()
    } else {
        COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(&filter.to_lowercase()))
            .copied()
            .collect()
    };

    if commands.is_empty() {
        None
    } else {
        Some(commands[state.command_menu_selected % commands.len()].0)
    }
}
