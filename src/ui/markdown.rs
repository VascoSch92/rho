//! Markdown rendering for the TUI.
//!
//! Converts markdown text to styled ratatui Lines.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::theme::Theme;

/// Render markdown text to styled Lines
pub fn render_markdown(text: &str, width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let parser = Parser::new(text);
    let mut renderer = MarkdownRenderer::new(width, theme);

    for event in parser {
        renderer.process_event(event);
    }

    renderer.finish()
}

struct MarkdownRenderer {
    lines: Vec<Line<'static>>,
    current_line: Vec<Span<'static>>,
    width: usize,

    // Cached theme colors (Copy so we own them)
    t: Theme,

    // Style state
    bold: bool,
    italic: bool,
    strikethrough: bool,

    // Block state
    in_code_block: bool,
    code_block_content: String,
    in_list: bool,
    list_indent: usize,
    in_heading: Option<HeadingLevel>,
    in_link: bool,
    link_url: String,
    link_text: String,
    in_blockquote: bool,
}

impl MarkdownRenderer {
    fn new(width: usize, theme: &Theme) -> Self {
        Self {
            lines: Vec::new(),
            current_line: Vec::new(),
            width,
            t: *theme,
            bold: false,
            italic: false,
            strikethrough: false,
            in_code_block: false,
            code_block_content: String::new(),
            in_list: false,
            list_indent: 0,
            in_heading: None,
            in_link: false,
            link_url: String::new(),
            link_text: String::new(),
            in_blockquote: false,
        }
    }

    fn current_style(&self) -> Style {
        let mut style = Style::default().fg(self.t.foreground);

        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.strikethrough {
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

        if self.in_link {
            self.link_text.push_str(text);
            return;
        }

        let style = self.current_style();
        self.current_line.push(Span::styled(text.to_string(), style));
    }

    fn flush_line(&mut self) {
        if !self.current_line.is_empty() {
            let mut line_spans = Vec::new();

            if self.in_blockquote {
                line_spans.push(Span::styled("│ ", Style::default().fg(self.t.muted)));
            }

            line_spans.append(&mut self.current_line);
            self.lines.push(Line::from(line_spans));
        }
        self.current_line = Vec::new();
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.push_text(&text),
            Event::Code(code) => {
                self.current_line.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(self.t.primary),
                ));
            }
            Event::SoftBreak => {
                self.current_line.push(Span::raw(" "));
            }
            Event::HardBreak => {
                self.flush_line();
            }
            Event::Rule => {
                self.flush_line();
                let rule = "─".repeat(self.width.min(60));
                self.lines.push(Line::from(vec![
                    Span::styled(rule, Style::default().fg(self.t.muted)),
                ]));
            }
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => {
                self.flush_line();
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
                self.lines.push(Line::from(vec![
                    Span::styled("```", Style::default().fg(self.t.muted)),
                ]));
            }
            Tag::List(_) => {
                self.in_list = true;
            }
            Tag::Item => {
                self.flush_line();
                let indent = "  ".repeat(self.list_indent);
                self.current_line.push(Span::styled(
                    format!("{}• ", indent),
                    Style::default().fg(self.t.accent),
                ));
            }
            Tag::Emphasis => {
                self.italic = true;
            }
            Tag::Strong => {
                self.bold = true;
            }
            Tag::Strikethrough => {
                self.strikethrough = true;
            }
            Tag::Link { dest_url, .. } => {
                self.in_link = true;
                self.link_url = dest_url.to_string();
                self.link_text.clear();
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush_line();
                self.lines.push(Line::from(""));
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
                        Span::styled(
                            line.to_string(),
                            Style::default().fg(self.t.primary),
                        ),
                    ]));
                }
                self.lines.push(Line::from(vec![
                    Span::styled("```", Style::default().fg(self.t.muted)),
                ]));
                self.in_code_block = false;
                self.code_block_content.clear();
            }
            TagEnd::List(_) => {
                self.in_list = false;
                self.list_indent = 0;
            }
            TagEnd::Item => {
                self.flush_line();
            }
            TagEnd::Emphasis => {
                self.italic = false;
            }
            TagEnd::Strong => {
                self.bold = false;
            }
            TagEnd::Strikethrough => {
                self.strikethrough = false;
            }
            TagEnd::Link => {
                self.current_line.push(Span::styled(
                    self.link_text.clone(),
                    Style::default().fg(self.t.accent).add_modifier(Modifier::UNDERLINED),
                ));
                self.current_line.push(Span::styled(
                    format!(" ({})", self.link_url),
                    Style::default().fg(self.t.muted),
                ));
                self.in_link = false;
            }
            _ => {}
        }
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
