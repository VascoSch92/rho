//! File path autocomplete menu for the @-trigger in the input field.
//!
//! When the user types `@` in the input field, a popup shows files and
//! directories relative to the current workspace. As the user types after
//! the `@`, the list filters by the current path segment:
//!
//! - `@`          → files in the workspace root
//! - `@src`       → files in the workspace root matching "src"
//! - `@src/`      → files inside `src/`
//! - `@src/ma`    → files inside `src/` matching "ma"
//!
//! On Tab/Enter, the `@...` token at the cursor is replaced with the full
//! relative path of the selected entry.

use std::fs;
use std::path::{Path, PathBuf};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::state::AppState;

/// Maximum entries returned by `scan_entries` (to keep rendering snappy).
const MAX_ENTRIES: usize = 50;
/// Maximum rows shown in the popup widget.
const MAX_VISIBLE: usize = 8;

/// A single entry in the autocomplete list.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Display name (e.g. `main.rs` or `src`).
    pub name: String,
    /// Full relative path from the workspace root (e.g. `src/main.rs`).
    pub relative_path: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
}

/// Parse the `@...` token the cursor is currently in, if any.
///
/// Returns `(token_start, directory, filter)` where:
/// - `token_start` is the index of the `@` in the input buffer
/// - `directory` is the directory to list (relative to the workspace)
/// - `filter` is the substring used to narrow the results
///
/// Returns `None` if the cursor is not inside an `@...` token.
pub fn parse_token(input: &str, cursor: usize) -> Option<(usize, String, String)> {
    let cursor = cursor.min(input.len());
    let before_cursor = &input[..cursor];

    // Find the `@` that starts the current token (last one before whitespace)
    let at_pos = before_cursor.rfind('@')?;

    // Reject if there's whitespace between the `@` and the cursor
    // (the user has moved past the token)
    let segment = &before_cursor[at_pos + 1..];
    if segment.contains(char::is_whitespace) {
        return None;
    }
    // Also reject if the char right before `@` is not a whitespace or the start of input
    // (avoid matching mid-word `foo@bar`)
    if at_pos > 0 {
        let prev = before_cursor[..at_pos].chars().last();
        if let Some(c) = prev {
            if !c.is_whitespace() {
                return None;
            }
        }
    }

    // Split the segment into directory + filter at the last `/`
    let (dir, filter) = match segment.rfind('/') {
        Some(idx) => (segment[..idx].to_string(), segment[idx + 1..].to_string()),
        None => (String::new(), segment.to_string()),
    };
    Some((at_pos, dir, filter))
}

/// Scan a directory relative to `workspace_root`, returning entries whose
/// names start with `filter` (case-insensitive). Directories sort first.
pub fn scan_entries(workspace_root: &Path, dir: &str, filter: &str) -> Vec<FileEntry> {
    let full_dir = if dir.is_empty() {
        workspace_root.to_path_buf()
    } else {
        workspace_root.join(dir)
    };

    // Safety: reject absolute paths or any `..` traversal up beyond the workspace
    if let Ok(canonical) = full_dir.canonicalize() {
        if let Ok(root_canonical) = workspace_root.canonicalize() {
            if !canonical.starts_with(&root_canonical) {
                return Vec::new();
            }
        }
    }

    let Ok(read_dir) = fs::read_dir(&full_dir) else {
        return Vec::new();
    };

    let filter_lower = filter.to_lowercase();
    let mut entries: Vec<FileEntry> = read_dir
        .filter_map(|r| r.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden files unless the filter explicitly starts with `.`
            if name.starts_with('.') && !filter.starts_with('.') {
                return None;
            }
            if !name.to_lowercase().starts_with(&filter_lower) {
                return None;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let relative_path = if dir.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", dir, name)
            };
            Some(FileEntry {
                name,
                relative_path,
                is_dir,
            })
        })
        .collect();

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries.truncate(MAX_ENTRIES);
    entries
}

/// Get the entries to display based on the current input/cursor.
pub fn current_entries(state: &AppState) -> Vec<FileEntry> {
    let Some((_, dir, filter)) = parse_token(&state.input_buffer, state.cursor_position) else {
        return Vec::new();
    };
    let root = PathBuf::from(&state.workspace_path);
    scan_entries(&root, &dir, &filter)
}

/// Insert the selected entry into the input buffer, replacing the current
/// `@...` token. For directories a trailing `/` is appended so the user can
/// keep drilling down without retyping `@`.
pub fn apply_selection(state: &mut AppState, entry: &FileEntry) {
    let Some((at_pos, _dir, _filter)) = parse_token(&state.input_buffer, state.cursor_position)
    else {
        return;
    };
    let replacement = if entry.is_dir {
        format!("@{}/", entry.relative_path)
    } else {
        format!("@{}", entry.relative_path)
    };
    let mut new_buffer = String::with_capacity(
        state.input_buffer.len() - (state.cursor_position - at_pos) + replacement.len(),
    );
    new_buffer.push_str(&state.input_buffer[..at_pos]);
    new_buffer.push_str(&replacement);
    new_buffer.push_str(&state.input_buffer[state.cursor_position..]);
    let new_cursor = at_pos + replacement.len();
    state.input_buffer = new_buffer;
    state.cursor_position = new_cursor;
}

/// Popup widget rendering the entries above the input.
pub struct FileMenuWidget<'a> {
    state: &'a AppState,
}

impl<'a> FileMenuWidget<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl Widget for FileMenuWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let entries = current_entries(self.state);
        if entries.is_empty() {
            return;
        }

        let t = &self.state.theme;
        let selected = self
            .state
            .file_menu
            .selected
            .min(entries.len().saturating_sub(1));

        // Scrolling window around the selection
        let visible = entries.len().min(MAX_VISIBLE);
        let start = if entries.len() <= visible || selected < visible / 2 {
            0
        } else if selected + visible / 2 >= entries.len() {
            entries.len().saturating_sub(visible)
        } else {
            selected.saturating_sub(visible / 2)
        };
        let end = (start + visible).min(entries.len());

        // Menu dimensions
        let menu_height = (visible + 2) as u16; // +2 for border
        let menu_width = 55.min(area.width.saturating_sub(4));
        let menu_x = area.x + 1;
        let menu_y = area.height.saturating_sub(menu_height + 4); // 4 for input area
        let menu_area = Rect::new(menu_x, menu_y, menu_width, menu_height);

        Clear.render(menu_area, buf);

        let mut lines: Vec<Line> = Vec::new();
        for (i, entry) in entries.iter().enumerate().take(end).skip(start) {
            let is_selected = i == selected;
            let style = if is_selected {
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.foreground)
            };
            let prefix =
                crate::ui::formatting::selector_prefix(is_selected, &self.state.selector_indicator);
            let suffix = if entry.is_dir { "/" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("{}{}", entry.name, suffix), style),
            ]));
        }

        let title = format!(" Files ({}) ", entries.len());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border))
            .title(Span::styled(title, Style::default().fg(t.accent)));

        let paragraph = Paragraph::new(lines).block(block);
        paragraph.render(menu_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token_simple() {
        assert_eq!(
            parse_token("@", 1),
            Some((0, "".to_string(), "".to_string()))
        );
        assert_eq!(
            parse_token("@src", 4),
            Some((0, "".to_string(), "src".to_string()))
        );
        assert_eq!(
            parse_token("@src/", 5),
            Some((0, "src".to_string(), "".to_string()))
        );
        assert_eq!(
            parse_token("@src/main", 9),
            Some((0, "src".to_string(), "main".to_string()))
        );
    }

    #[test]
    fn test_parse_token_inline() {
        assert_eq!(
            parse_token("look at @src/main.rs", 19),
            Some((8, "src".to_string(), "main.r".to_string()))
        );
    }

    #[test]
    fn test_parse_token_none() {
        // Cursor past whitespace
        assert_eq!(parse_token("@src foo", 8), None);
        // Mid-word @ (e.g., email)
        assert_eq!(parse_token("foo@bar", 7), None);
        // No @
        assert_eq!(parse_token("hello", 5), None);
    }
}
