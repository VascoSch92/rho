//! Shared modal frame — consistent styling for all modal dialogs.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

use crate::config::theme::Theme;

/// Render a modal with consistent styling: rounded border, themed colors, centered.
/// Width and height are computed from the content (+ 2 for borders).
pub fn render_modal(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    lines: Vec<Line<'_>>,
    theme: &Theme,
) {
    render_modal_inner(area, buf, title, lines, theme, Alignment::Left);
}

/// Like `render_modal` but centers every line horizontally inside the modal.
pub fn render_modal_centered(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    lines: Vec<Line<'_>>,
    theme: &Theme,
) {
    render_modal_inner(area, buf, title, lines, theme, Alignment::Center);
}

fn render_modal_inner(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    lines: Vec<Line<'_>>,
    theme: &Theme,
    alignment: Alignment,
) {
    // Compute width from the longest line (+ 2 for border + 2 for padding)
    let title_width = title.chars().count() + 4; // " Title " + border
    let content_width = lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0);
    let width = (content_width.max(title_width) + 4) as u16;

    // Height = number of lines + 2 for borders
    let height = (lines.len() as u16) + 2;

    let modal_width = width.min(area.width.saturating_sub(4));
    let modal_height = height.min(area.height.saturating_sub(4));
    let modal_x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

    Clear.render(modal_area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(modal_area);
    block.render(modal_area, buf);

    let paragraph = Paragraph::new(lines).alignment(alignment);
    paragraph.render(inner, buf);
}
