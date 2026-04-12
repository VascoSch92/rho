//! Skills modal — tabbed compact list with an inline detail view.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use super::tabs::{max_line_width, tab_bar_line};
use crate::client::SkillInfo;
use crate::config::theme::Theme;
use crate::state::AppState;

const TABS: &[&str] = &["All", "User", "Project", "Public"];

/// Skills modal — tabbed list with an inline detail view.
///
/// The modal always renders the same tab bar; when `detail_open` is true
/// the list body is replaced with the detail view. Width and height are
/// computed across every possible body so switching tabs or toggling detail
/// never resizes the modal.
pub struct SkillsModal<'a> {
    state: &'a AppState,
}

impl<'a> SkillsModal<'a> {
    const TITLE: &'static str = "Skills";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn truncate(s: &str, max: usize) -> String {
        if s.chars().count() <= max {
            s.to_string()
        } else {
            let trunc: String = s.chars().take(max.saturating_sub(1)).collect();
            format!("{}…", trunc)
        }
    }

    /// Filter skills by the active tab index.
    fn filter_skills(skills: &[SkillInfo], tab: usize) -> Vec<&SkillInfo> {
        skills
            .iter()
            .filter(|s| match tab {
                0 => true,
                1 => s
                    .source
                    .as_deref()
                    .map(|src| src.to_lowercase().contains("user"))
                    .unwrap_or(false),
                2 => s
                    .source
                    .as_deref()
                    .map(|src| src.to_lowercase().contains("project"))
                    .unwrap_or(false),
                3 => s
                    .source
                    .as_deref()
                    .map(|src| {
                        let l = src.to_lowercase();
                        l.contains("public") || l.contains("marketplace")
                    })
                    .unwrap_or(false),
                _ => true,
            })
            .collect()
    }

    /// Build the list body for a given tab.
    fn list_lines(
        state: &AppState,
        skills: &[SkillInfo],
        tab: usize,
        t: &Theme,
    ) -> Vec<Line<'static>> {
        let filtered = Self::filter_skills(skills, tab);
        let mut lines: Vec<Line<'static>> = Vec::new();

        if state.skills_modal.loading && skills.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  Loading skills...",
                Style::default().fg(t.muted).add_modifier(Modifier::ITALIC),
            )]));
            return lines;
        }

        if let Some(ref err) = state.skills_modal.error {
            lines.push(Line::from(vec![Span::styled(
                format!("  Error: {}", err),
                Style::default().fg(t.error),
            )]));
            return lines;
        }

        if filtered.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  No skills in this tab.",
                Style::default().fg(t.muted),
            )]));
            lines.push(Line::from(vec![
                Span::styled("  Drop ", Style::default().fg(t.muted)),
                Span::styled("SKILL.md", Style::default().fg(t.accent)),
                Span::styled(" files in ", Style::default().fg(t.muted)),
                Span::styled(".rho/skills/", Style::default().fg(t.accent)),
            ]));
            return lines;
        }

        // Scrolling window around the selection
        let max_visible = 10;
        let total = filtered.len();
        let selected = state.skills_modal.selected.min(total.saturating_sub(1));

        let start = if total <= max_visible || selected < max_visible / 2 {
            0
        } else if selected + max_visible / 2 >= total {
            total.saturating_sub(max_visible)
        } else {
            selected.saturating_sub(max_visible / 2)
        };
        let end = (start + max_visible).min(total);

        for (i, skill) in filtered.iter().enumerate().take(end).skip(start) {
            let is_selected = i == selected;
            let indicator = format!(
                " {}",
                crate::ui::formatting::selector_prefix(is_selected, &state.selector_indicator)
            );
            let name_style = if is_selected {
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.foreground)
            };
            let source = skill
                .source
                .as_deref()
                .map(|s| Self::truncate(s, 16))
                .unwrap_or_else(|| "?".to_string());

            lines.push(Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(Self::truncate(&skill.name, 36), name_style),
                Span::raw("  "),
                Span::styled(format!("[{}]", source), Style::default().fg(t.muted)),
            ]));
        }

        if total > max_visible {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                format!("  {}/{}", selected + 1, total),
                Style::default().fg(t.muted),
            )]));
        }

        lines
    }

    /// Build the detail body for the currently selected skill.
    fn detail_lines(skill: &SkillInfo, t: &Theme) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("  Name:        ", Style::default().fg(t.muted)),
            Span::styled(
                skill.name.clone(),
                Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
            ),
        ]));
        if let Some(ref source) = skill.source {
            lines.push(Line::from(vec![
                Span::styled("  Source:      ", Style::default().fg(t.muted)),
                Span::styled(source.clone(), Style::default().fg(t.accent)),
            ]));
        }
        if let Some(ref st) = skill.skill_type {
            lines.push(Line::from(vec![
                Span::styled("  Type:        ", Style::default().fg(t.muted)),
                Span::styled(st.clone(), Style::default().fg(t.foreground)),
            ]));
        }
        if !skill.triggers.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  Triggers:    ", Style::default().fg(t.muted)),
                Span::styled(skill.triggers.join(", "), Style::default().fg(t.success)),
            ]));
        }

        lines.push(Line::from(""));

        if let Some(ref desc) = skill.description {
            lines.push(Line::from(vec![Span::styled(
                "  Description:",
                Style::default().fg(t.muted).add_modifier(Modifier::BOLD),
            )]));
            let wrap_width = 64;
            for line in textwrap::wrap(desc, wrap_width) {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(line.to_string(), Style::default().fg(t.foreground)),
                ]));
            }
        } else {
            lines.push(Line::from(vec![Span::styled(
                "  (no description)",
                Style::default().fg(t.muted).add_modifier(Modifier::ITALIC),
            )]));
        }

        lines
    }

    fn list_footer(t: &Theme) -> Line<'static> {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Tab", Style::default().fg(t.primary)),
            Span::styled("/", Style::default().fg(t.muted)),
            Span::styled("←→", Style::default().fg(t.primary)),
            Span::styled(" switch tab  ", Style::default().fg(t.muted)),
            Span::styled("↑↓", Style::default().fg(t.primary)),
            Span::styled(" navigate  ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" details  ", Style::default().fg(t.muted)),
            Span::styled("r", Style::default().fg(t.primary)),
            Span::styled(" sync  ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" close", Style::default().fg(t.muted)),
        ])
    }

    fn detail_footer(t: &Theme) -> Line<'static> {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" back to list", Style::default().fg(t.muted)),
        ])
    }
}

impl Widget for SkillsModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let skills = &self.state.skills_modal.skills;

        // Build every possible body so we can compute stable dimensions.
        let list_panes: Vec<Vec<Line<'static>>> = (0..TABS.len())
            .map(|i| Self::list_lines(self.state, skills, i, t))
            .collect();

        let detail_body: Option<Vec<Line<'static>>> = if self.state.skills_modal.detail_open {
            let filtered = Self::filter_skills(skills, self.state.skills_modal.tab);
            filtered
                .get(self.state.skills_modal.selected)
                .map(|s| Self::detail_lines(s, t))
        } else {
            None
        };

        // Compute max dimensions across all list panes AND the detail body,
        // so the modal stays the same size when switching views.
        let list_footer = Self::list_footer(t);
        let detail_footer = Self::detail_footer(t);
        let footer_width = max_line_width(std::slice::from_ref(&list_footer))
            .max(max_line_width(std::slice::from_ref(&detail_footer)));

        let mut max_width = footer_width;
        let mut max_height = 0usize;
        for pane in &list_panes {
            max_width = max_width.max(max_line_width(pane));
            max_height = max_height.max(pane.len());
        }
        if let Some(ref body) = detail_body {
            max_width = max_width.max(max_line_width(body));
            max_height = max_height.max(body.len());
        }

        // Pick the active body: detail if open, otherwise active tab
        let mut body = if let Some(body) = detail_body {
            body
        } else {
            list_panes
                .into_iter()
                .nth(self.state.skills_modal.tab.min(TABS.len() - 1))
                .unwrap_or_default()
        };

        // Pad horizontally and vertically so the modal size stays stable
        for line in body.iter_mut() {
            let w: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            if w < max_width {
                line.spans.push(Span::raw(" ".repeat(max_width - w)));
            }
        }
        while body.len() < max_height {
            body.push(Line::from(Span::raw(" ".repeat(max_width))));
        }

        // Assemble final lines: spacer, tab bar, spacer, body, spacer, footer
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::from(Span::raw(" ".repeat(max_width))));
        lines.push(tab_bar_line(TABS, self.state.skills_modal.tab, t));
        lines.push(Line::from(Span::raw(" ".repeat(max_width))));
        lines.extend(body);
        lines.push(Line::from(Span::raw(" ".repeat(max_width))));
        lines.push(if self.state.skills_modal.detail_open {
            detail_footer
        } else {
            list_footer
        });

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
