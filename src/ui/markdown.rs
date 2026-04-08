//! Markdown rendering for the TUI.
//!
//! Converts markdown text to styled ratatui Lines.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::config::theme::Theme;

/// Render markdown text to styled ratatui `Line`s.
///
/// Supports headings (H1–H6 with block prefixes), bold, italic, strikethrough,
/// inline code, code blocks, lists (with nesting), links, blockquotes,
/// horizontal rules, and tables (with box-drawing borders).
pub fn render_markdown(text: &str, width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(text, options);
    let mut renderer = MarkdownRenderer::new(width, theme);

    for event in parser {
        renderer.process_event(event);
    }

    renderer.finish()
}

/// Inline style modifiers (bold, italic, strikethrough).
#[derive(Default)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    strikethrough: bool,
}

/// Link state (accumulates text while inside a link).
#[derive(Default)]
struct LinkState {
    active: bool,
    url: String,
    text: String,
}

/// Table accumulator — collects rows and column widths until end of table.
#[derive(Default)]
struct TableState {
    active: bool,
    in_head: bool,
    row: Vec<String>,
    rows: Vec<Vec<String>>,
    col_widths: Vec<usize>,
    is_header: Vec<bool>,
}

struct MarkdownRenderer {
    lines: Vec<Line<'static>>,
    current_line: Vec<Span<'static>>,
    width: usize,
    t: Theme,

    // Inline style
    style: InlineStyle,

    // Block context
    in_code_block: bool,
    code_block_content: String,
    in_list: bool,
    in_item: bool,
    list_indent: usize,
    in_heading: Option<HeadingLevel>,
    in_blockquote: bool,

    // Link state
    link: LinkState,

    // Table state
    table: TableState,
}

impl MarkdownRenderer {
    fn new(width: usize, theme: &Theme) -> Self {
        Self {
            lines: Vec::new(),
            current_line: Vec::new(),
            width,
            t: *theme,
            style: InlineStyle::default(),
            in_code_block: false,
            code_block_content: String::new(),
            in_list: false,
            in_item: false,
            list_indent: 0,
            in_heading: None,
            in_blockquote: false,
            link: LinkState::default(),
            table: TableState::default(),
        }
    }

    fn current_style(&self) -> Style {
        let mut style = Style::default().fg(self.t.foreground);

        if self.style.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.style.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.style.strikethrough {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        if self.in_heading.is_some() {
            style = style.fg(self.t.primary).add_modifier(Modifier::BOLD);
        }
        if self.in_blockquote {
            style = style.fg(self.t.muted).add_modifier(Modifier::ITALIC);
        }

        style
    }

    fn push_text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_block_content.push_str(text);
            return;
        }

        if self.link.active {
            self.link.text.push_str(text);
            return;
        }

        if self.table.active {
            // Accumulate text into the current cell
            if let Some(cell) = self.table.row.last_mut() {
                cell.push_str(text);
            }
            return;
        }

        let style = self.current_style();
        self.current_line
            .push(Span::styled(text.to_string(), style));
    }

    fn flush_line(&mut self) {
        if self.current_line.is_empty() {
            self.current_line = Vec::new();
            return;
        }

        let prefix: Option<Span<'static>> = if self.in_blockquote {
            Some(Span::styled("│ ", Style::default().fg(self.t.muted)))
        } else {
            None
        };

        let prefix_width = prefix.as_ref().map_or(0, |p| p.content.chars().count());
        let wrap_width = self.width.saturating_sub(prefix_width);

        let spans = std::mem::take(&mut self.current_line);
        let wrapped = Self::wrap_spans(&spans, wrap_width);

        for line_spans in wrapped {
            let mut out = Vec::new();
            if let Some(ref pfx) = prefix {
                out.push(pfx.clone());
            }
            out.extend(line_spans);
            self.lines.push(Line::from(out));
        }
    }

    /// Wrap a sequence of styled spans into multiple lines that fit within `max_width`.
    fn wrap_spans(spans: &[Span<'static>], max_width: usize) -> Vec<Vec<Span<'static>>> {
        if max_width == 0 {
            return vec![spans.to_vec()];
        }

        // Build a flat list of (char, style_index) so we can find break points
        let mut chars: Vec<(char, usize)> = Vec::new();
        let mut styles: Vec<Style> = Vec::new();
        for (si, span) in spans.iter().enumerate() {
            styles.push(span.style);
            for ch in span.content.chars() {
                chars.push((ch, si));
            }
        }

        if chars.is_empty() {
            return vec![spans.to_vec()];
        }

        // Check total width — if it fits, return as-is
        if chars.len() <= max_width {
            return vec![spans.to_vec()];
        }

        // Split into lines, preferring word boundaries
        let mut result: Vec<Vec<Span<'static>>> = Vec::new();
        let mut pos = 0;

        while pos < chars.len() {
            let remaining = chars.len() - pos;
            if remaining <= max_width {
                // Last chunk fits
                result.push(Self::chars_to_spans(&chars[pos..], &styles));
                break;
            }

            let end = pos + max_width;
            // Look backwards for a space to break at
            let break_at = (pos..end)
                .rev()
                .find(|&i| chars[i].0 == ' ')
                .map(|i| i + 1) // break after the space
                .unwrap_or(end); // no space found, hard break

            result.push(Self::chars_to_spans(&chars[pos..break_at], &styles));
            pos = break_at;
            // Skip leading spaces on the new line
            while pos < chars.len() && chars[pos].0 == ' ' {
                pos += 1;
            }
        }

        if result.is_empty() {
            result.push(vec![]);
        }
        result
    }

    /// Convert a slice of (char, style_index) back into coalesced spans.
    fn chars_to_spans(chars: &[(char, usize)], styles: &[Style]) -> Vec<Span<'static>> {
        if chars.is_empty() {
            return vec![];
        }
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut current_text = String::new();
        let mut current_style_idx = chars[0].1;

        for &(ch, si) in chars {
            if si != current_style_idx {
                if !current_text.is_empty() {
                    spans.push(Span::styled(
                        current_text.clone(),
                        styles[current_style_idx],
                    ));
                    current_text.clear();
                }
                current_style_idx = si;
            }
            current_text.push(ch);
        }
        if !current_text.is_empty() {
            spans.push(Span::styled(current_text, styles[current_style_idx]));
        }
        spans
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.push_text(&text),
            Event::Code(code) => {
                if self.table.active {
                    if let Some(cell) = self.table.row.last_mut() {
                        cell.push_str(&code);
                    }
                } else {
                    self.current_line.push(Span::styled(
                        code.to_string(),
                        Style::default().fg(self.t.primary),
                    ));
                }
            }
            Event::SoftBreak => {
                if self.table.active {
                    if let Some(cell) = self.table.row.last_mut() {
                        cell.push(' ');
                    }
                } else {
                    self.current_line.push(Span::raw(" "));
                }
            }
            Event::HardBreak => {
                self.flush_line();
            }
            Event::Rule => {
                self.flush_line();
                let rule = "─".repeat(self.width.min(60));
                self.lines.push(Line::from(vec![Span::styled(
                    rule,
                    Style::default().fg(self.t.muted),
                )]));
            }
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => {
                self.flush_line();
                // Ensure blank line before headings for visual separation
                if !self.lines.is_empty() && self.lines.last().is_some_and(|l| !l.spans.is_empty())
                {
                    self.lines.push(Line::from(""));
                }
                self.in_heading = Some(level);

                let prefix = match level {
                    HeadingLevel::H1 => "█ ",
                    HeadingLevel::H2 => "▓ ",
                    HeadingLevel::H3 => "▒ ",
                    HeadingLevel::H4 => "░ ",
                    HeadingLevel::H5 => "• ",
                    HeadingLevel::H6 => "· ",
                };
                self.current_line.push(Span::styled(
                    prefix.to_string(),
                    Style::default().fg(self.t.primary),
                ));
            }
            Tag::BlockQuote(_) => {
                self.flush_line();
                self.in_blockquote = true;
            }
            Tag::CodeBlock(_) => {
                self.flush_line();
                self.in_code_block = true;
                self.code_block_content.clear();
                self.lines.push(Line::from(vec![Span::styled(
                    "```",
                    Style::default().fg(self.t.muted),
                )]));
            }
            Tag::List(_) => {
                self.flush_line();
                if self.in_list {
                    self.list_indent += 1;
                } else {
                    self.lines.push(Line::from(""));
                }
                self.in_list = true;
            }
            Tag::Item => {
                self.flush_line();
                self.in_item = true;
                let indent = "  ".repeat(self.list_indent);
                self.current_line.push(Span::styled(
                    format!("{}• ", indent),
                    Style::default().fg(self.t.accent),
                ));
            }
            Tag::Emphasis => {
                self.style.italic = true;
            }
            Tag::Strong => {
                self.style.bold = true;
            }
            Tag::Strikethrough => {
                self.style.strikethrough = true;
            }
            Tag::Link { dest_url, .. } => {
                self.link.active = true;
                self.link.url = dest_url.to_string();
                self.link.text.clear();
            }
            Tag::Table(_alignments) => {
                self.flush_line();
                self.table.active = true;
                self.table.rows.clear();
                self.table.col_widths.clear();
                self.table.is_header.clear();
            }
            Tag::TableHead => {
                self.table.in_head = true;
                self.table.row.clear();
            }
            Tag::TableRow => {
                self.table.row.clear();
            }
            Tag::TableCell => {
                self.table.row.push(String::new());
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush_line();
                // Don't add blank line after paragraphs inside list items
                if !self.in_item {
                    self.lines.push(Line::from(""));
                }
            }
            TagEnd::Heading(_) => {
                self.flush_line();
                self.in_heading = None;
                self.lines.push(Line::from(""));
            }
            TagEnd::BlockQuote(_) => {
                self.flush_line();
                self.in_blockquote = false;
            }
            TagEnd::CodeBlock => {
                for line in self.code_block_content.lines() {
                    self.lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(line.to_string(), Style::default().fg(self.t.primary)),
                    ]));
                }
                self.lines.push(Line::from(vec![Span::styled(
                    "```",
                    Style::default().fg(self.t.muted),
                )]));
                self.in_code_block = false;
                self.code_block_content.clear();
            }
            TagEnd::List(_) => {
                self.flush_line();
                if self.list_indent > 0 {
                    self.list_indent -= 1;
                } else {
                    self.in_list = false;
                    self.lines.push(Line::from(""));
                }
            }
            TagEnd::Item => {
                self.flush_line();
                self.in_item = false;
            }
            TagEnd::Emphasis => {
                self.style.italic = false;
            }
            TagEnd::Strong => {
                self.style.bold = false;
            }
            TagEnd::Strikethrough => {
                self.style.strikethrough = false;
            }
            TagEnd::Link => {
                if self.table.active {
                    // Inside table: append link text to current cell
                    if let Some(cell) = self.table.row.last_mut() {
                        cell.push_str(&self.link.text);
                    }
                } else {
                    self.current_line.push(Span::styled(
                        self.link.text.clone(),
                        Style::default()
                            .fg(self.t.accent)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    self.current_line.push(Span::styled(
                        format!(" ({})", self.link.url),
                        Style::default().fg(self.t.muted),
                    ));
                }
                self.link.active = false;
            }
            TagEnd::TableHead => {
                // Save header row
                let row = std::mem::take(&mut self.table.row);
                // Update column widths
                for (i, cell) in row.iter().enumerate() {
                    let w = cell.chars().count();
                    if i >= self.table.col_widths.len() {
                        self.table.col_widths.push(w);
                    } else if w > self.table.col_widths[i] {
                        self.table.col_widths[i] = w;
                    }
                }
                self.table.rows.push(row);
                self.table.is_header.push(true);
                self.table.in_head = false;
            }
            TagEnd::TableRow => {
                let row = std::mem::take(&mut self.table.row);
                for (i, cell) in row.iter().enumerate() {
                    let w = cell.chars().count();
                    if i >= self.table.col_widths.len() {
                        self.table.col_widths.push(w);
                    } else if w > self.table.col_widths[i] {
                        self.table.col_widths[i] = w;
                    }
                }
                self.table.rows.push(row);
                self.table.is_header.push(false);
            }
            TagEnd::TableCell => {
                // Cell text already accumulated via push_text
            }
            TagEnd::Table => {
                self.render_table();
                self.table.active = false;
                self.lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    /// Shrink column widths proportionally so the table fits within `max_width`.
    fn fit_col_widths(col_widths: &[usize], max_width: usize) -> Vec<usize> {
        let ncols = col_widths.len();
        if ncols == 0 {
            return vec![];
        }

        // Total width = borders (ncols + 1) + padding (2 per col) + content
        let border_overhead = ncols + 1 + ncols * 2;
        let available = max_width.saturating_sub(border_overhead);
        let content_total: usize = col_widths.iter().sum();

        if content_total <= available {
            return col_widths.to_vec();
        }

        // Minimum 3 chars per column (enough for "ab…")
        let min_col = 3;
        // Shrink proportionally, respecting minimums
        let mut fitted: Vec<usize> = col_widths
            .iter()
            .map(|&w| {
                let scaled = (w as f64 * available as f64 / content_total as f64).floor() as usize;
                scaled.max(min_col)
            })
            .collect();

        // If rounding over-allocated, trim the widest columns
        let mut total: usize = fitted.iter().sum();
        while total > available {
            if let Some((idx, _)) = fitted.iter().enumerate().max_by_key(|(_, w)| *w) {
                if fitted[idx] > min_col {
                    fitted[idx] -= 1;
                    total -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        fitted
    }

    /// Wrap a cell's text into lines that fit within `max_chars`.
    fn wrap_cell(s: &str, max_chars: usize) -> Vec<String> {
        if max_chars == 0 {
            return vec![String::new()];
        }
        if s.is_empty() {
            return vec![String::new()];
        }
        use textwrap::wrap;
        let wrapped = wrap(s, max_chars);
        wrapped.iter().map(|l| l.to_string()).collect()
    }

    /// Render the accumulated table rows into styled lines.
    fn render_table(&mut self) {
        if self.table.rows.is_empty() {
            return;
        }

        let col_widths = Self::fit_col_widths(&self.table.col_widths, self.width);
        let border_style = Style::default().fg(self.t.muted);
        let header_style = Style::default()
            .fg(self.t.primary)
            .add_modifier(Modifier::BOLD);
        let cell_style = Style::default().fg(self.t.foreground);

        // Top border: ┌──────┬──────┐
        let mut top = String::from("┌");
        for (i, w) in col_widths.iter().enumerate() {
            top.push_str(&"─".repeat(w + 2));
            if i < col_widths.len() - 1 {
                top.push('┬');
            }
        }
        top.push('┐');
        self.lines
            .push(Line::from(vec![Span::styled(top, border_style)]));

        let total_rows = self.table.rows.len();
        for (row_idx, row) in self.table.rows.iter().enumerate() {
            let is_header = self.table.is_header.get(row_idx).copied().unwrap_or(false);
            let is_last = row_idx == total_rows - 1;
            let style = if is_header { header_style } else { cell_style };

            // Wrap each cell and find the max number of visual lines
            let wrapped_cells: Vec<Vec<String>> = col_widths
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                    Self::wrap_cell(cell, *w)
                })
                .collect();
            let max_lines = wrapped_cells.iter().map(|c| c.len()).max().unwrap_or(1);

            // Render each visual line of this row
            for line_idx in 0..max_lines {
                let mut spans = vec![Span::styled("│", border_style)];
                for (i, w) in col_widths.iter().enumerate() {
                    let text = wrapped_cells
                        .get(i)
                        .and_then(|lines| lines.get(line_idx))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let padded = format!(" {:<width$} ", text, width = w);
                    spans.push(Span::styled(padded, style));
                    spans.push(Span::styled("│", border_style));
                }
                self.lines.push(Line::from(spans));
            }

            // Row separator (skip after last row — bottom border handles it)
            if !is_last {
                let mut sep = String::from("├");
                for (i, w) in col_widths.iter().enumerate() {
                    sep.push_str(&"─".repeat(w + 2));
                    if i < col_widths.len() - 1 {
                        sep.push('┼');
                    }
                }
                sep.push('┤');
                self.lines
                    .push(Line::from(vec![Span::styled(sep, border_style)]));
            }
        }

        // Bottom border: └──────┴──────┘
        let mut bottom = String::from("└");
        for (i, w) in col_widths.iter().enumerate() {
            bottom.push_str(&"─".repeat(w + 2));
            if i < col_widths.len() - 1 {
                bottom.push('┴');
            }
        }
        bottom.push('┘');
        self.lines
            .push(Line::from(vec![Span::styled(bottom, border_style)]));
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_line();
        self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_text() {
        let lines = render_markdown("Hello world", 80, &Theme::default());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_bold() {
        let lines = render_markdown("**bold text**", 80, &Theme::default());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_code_block() {
        let lines = render_markdown("```\ncode here\n```", 80, &Theme::default());
        assert!(lines.len() >= 3);
    }
}
