//! Reusable tab bar and pane layout helpers for modal dialogs.
//!
//! Provides:
//! - [`tab_bar_line`] — renders a styled tab bar line (active tab underlined + bold in primary)
//! - [`build_tabbed_lines`] — composes tab bar + padded pane body so the modal stays a stable size
//!   when switching tabs.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::config::theme::Theme;

/// Render a tab bar as a styled `Line`.
///
/// The active tab is shown as a "pill": primary foreground on the border color
/// background, bold. Inactive tabs are muted. Tabs are separated by a `│`.
pub fn tab_bar_line(tabs: &[&str], active: usize, t: &Theme) -> Line<'static> {
    let mut spans = vec![Span::raw("  ")];
    for (i, name) in tabs.iter().enumerate() {
        let is_active = i == active;
        if is_active {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(t.primary)
                    .bg(t.border)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(t.muted),
            ));
        }
        if i < tabs.len() - 1 {
            spans.push(Span::styled(" │ ", Style::default().fg(t.border)));
        }
    }
    Line::from(spans)
}

/// Rotate a tab index by one step, wrapping around.
///
/// `forward = true` moves to the next tab, `false` to the previous one.
/// Returns the new index. Safe for any `num_tabs >= 1`.
pub fn rotate_tab(current: usize, num_tabs: usize, forward: bool) -> usize {
    if num_tabs == 0 {
        return 0;
    }
    if forward {
        (current + 1) % num_tabs
    } else {
        (current + num_tabs - 1) % num_tabs
    }
}

/// Compute the maximum visible character width of a set of lines.
pub fn max_line_width(lines: &[Line<'static>]) -> usize {
    lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0)
}

/// Build the final lines for a tabbed modal.
///
/// Takes all tab panes, the active index, and an optional footer (e.g. keybinding hints).
/// The result:
///   - Tab bar at the top.
///   - Active pane body, padded to the max width/height across all panes so the modal
///     has a stable size when switching tabs.
///   - Footer line(s) appended after the body.
pub fn build_tabbed_lines(
    tabs: &[&str],
    active: usize,
    panes: Vec<Vec<Line<'static>>>,
    footer: Vec<Line<'static>>,
    t: &Theme,
) -> Vec<Line<'static>> {
    assert_eq!(
        tabs.len(),
        panes.len(),
        "tabs and panes must have the same length"
    );
    let active = active.min(panes.len().saturating_sub(1));

    // Compute max dimensions across all panes
    let max_width = panes.iter().map(|p| max_line_width(p)).max().unwrap_or(0);
    let max_height = panes.iter().map(|p| p.len()).max().unwrap_or(0);

    // Also consider the footer width so we don't clip it
    let footer_width = max_line_width(&footer);
    let max_width = max_width.max(footer_width);

    // Pad the active pane to the max dimensions
    let mut body = panes.into_iter().nth(active).unwrap_or_default();
    for line in body.iter_mut() {
        let w: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        if w < max_width {
            line.spans.push(Span::raw(" ".repeat(max_width - w)));
        }
    }
    while body.len() < max_height {
        body.push(Line::from(Span::raw(" ".repeat(max_width))));
    }

    // Build the final list: spacer, tab bar, spacer, body, spacer, footer
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::raw(" ".repeat(max_width))));
    lines.push(tab_bar_line(tabs, active, t));
    lines.push(Line::from(Span::raw(" ".repeat(max_width))));
    lines.extend(body);
    lines.push(Line::from(Span::raw(" ".repeat(max_width))));
    lines.extend(footer);
    lines
}
