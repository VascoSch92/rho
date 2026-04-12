//! Help and policy modal widgets.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use super::tabs::build_tabbed_lines;
use crate::config::theme::Theme;
use crate::state::{AppState, ConfirmationPolicy};

const TABS: &[&str] = &["Commands", "Shortcuts"];

// ── Help line builders ──────────────────────────────────────────────────────

/// Build a help line: "  key   description" with key in `key_color` and desc in `desc_color`.
fn help_line(key: &str, desc: &str, key_color: Color, desc_color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<13}", key), Style::default().fg(key_color)),
        Span::styled(desc.to_string(), Style::default().fg(desc_color)),
    ])
}

/// Horizontal separator line.
fn separator(t: &Theme) -> Line<'static> {
    Line::from(vec![Span::styled(
        format!("  {}", "─".repeat(44)),
        Style::default().fg(t.muted),
    )])
}

// ── Help modal ──────────────────────────────────────────────────────────────

/// Help modal showing available commands
pub struct HelpModal<'a> {
    state: &'a AppState,
}

impl<'a> HelpModal<'a> {
    const TITLE: &'static str = "Help";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

/// Keyboard shortcuts: (key, description)
const SHORTCUTS: &[(&str, &str)] = &[
    ("!<cmd>", "Run bash command (e.g. !ls, !pwd)"),
    ("@<path>", "Autocomplete file path (e.g. @src/main.rs)"),
];

/// Key bindings: (key, description)
const KEYBINDINGS: &[(&str, &str)] = &[
    ("Alt+Enter", "New line in input (or Shift+Enter)"),
    ("Ctrl+Q", "Quit"),
    ("↑↓ PgUp/Dn", "Scroll messages"),
    ("Ctrl+E", "Expand/collapse all actions"),
    ("Ctrl+T", "Toggle task list panel"),
    ("Mouse wheel", "Scroll messages"),
];

/// Text selection modifiers: (terminal, key)
const TEXT_SELECTION: &[(&str, &str)] = &[
    ("macOS", "Fn"),
    ("iTerm2", "Option / Cmd"),
    ("Linux", "Shift"),
];

impl HelpModal<'_> {
    /// Commands tab: slash commands (sourced from the command menu).
    fn commands_pane(t: &Theme) -> Vec<Line<'static>> {
        crate::ui::command_menu::COMMANDS
            .iter()
            .map(|(key, desc)| help_line(&format!("/{}", key), desc, t.primary, t.foreground))
            .collect()
    }

    /// Shortcuts tab: input shortcuts + key bindings + text selection.
    fn shortcuts_pane(t: &Theme) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Input shortcuts (@, !)
        for (key, desc) in SHORTCUTS {
            lines.push(help_line(key, desc, t.primary, t.foreground));
        }

        lines.push(Line::from(""));
        lines.push(separator(t));
        lines.push(Line::from(""));

        // Key bindings
        for (key, desc) in KEYBINDINGS {
            lines.push(help_line(key, desc, t.accent, t.muted));
        }

        lines.push(Line::from(""));
        lines.push(separator(t));
        lines.push(Line::from(""));

        // Text selection
        lines.push(Line::from(vec![Span::styled(
            "  Text selection (hold modifier + click/drag):".to_string(),
            Style::default().fg(t.muted),
        )]));
        lines.push(Line::from(""));
        for (terminal, key) in TEXT_SELECTION {
            lines.push(help_line(
                &format!("  {}", terminal),
                key,
                t.accent,
                t.muted,
            ));
        }

        lines
    }
}

impl Widget for HelpModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;

        let panes = vec![Self::commands_pane(t), Self::shortcuts_pane(t)];

        let footer = vec![Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Tab", Style::default().fg(t.primary)),
            Span::styled("/", Style::default().fg(t.muted)),
            Span::styled("←→", Style::default().fg(t.primary)),
            Span::styled(" switch tab  ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" close", Style::default().fg(t.muted)),
        ])];

        let lines = build_tabbed_lines(TABS, self.state.help_modal_tab, panes, footer, t);
        render_modal(area, buf, Self::TITLE, lines, t);
    }
}

// ── Policy modal ────────────────────────────────────────────────────────────

/// Policy modal — navigate with ↑/↓, Enter to apply, Esc to cancel
pub struct PolicyModal<'a> {
    state: &'a AppState,
}

impl<'a> PolicyModal<'a> {
    const TITLE: &'static str = "Confirmation Policy";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

const POLICIES: &[ConfirmationPolicy] = &[
    ConfirmationPolicy::AlwaysConfirm,
    ConfirmationPolicy::ConfirmRisky,
    ConfirmationPolicy::NeverConfirm,
];

fn policy_color(policy: ConfirmationPolicy, t: &Theme) -> Color {
    match policy {
        ConfirmationPolicy::AlwaysConfirm => t.success,
        ConfirmationPolicy::ConfirmRisky => t.primary,
        ConfirmationPolicy::NeverConfirm => t.error,
    }
}

fn policy_description(policy: ConfirmationPolicy) -> &'static str {
    match policy {
        ConfirmationPolicy::AlwaysConfirm => "   Confirm all actions",
        ConfirmationPolicy::ConfirmRisky => "   Only risky actions",
        ConfirmationPolicy::NeverConfirm => "   Auto-approve all",
    }
}

impl Widget for PolicyModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        for (i, policy) in POLICIES.iter().enumerate() {
            let is_selected = i == self.state.policy_selected;
            let is_current = *policy == self.state.confirmation_policy;
            let color = policy_color(*policy, t);

            let indicator = format!(
                " {}",
                crate::ui::formatting::selector_prefix(is_selected, &self.state.selector_indicator)
            );
            let name_style = if is_selected {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.foreground)
            };

            let mut spans = vec![
                Span::styled(indicator, name_style),
                Span::styled(format!("{:<16}", policy), name_style),
                Span::styled(policy_description(*policy), Style::default().fg(t.muted)),
            ];

            if is_current {
                spans.push(Span::styled("  (current)", Style::default().fg(t.muted)));
            }

            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  ↑/↓", Style::default().fg(t.primary)),
            Span::styled(" navigate  ", Style::default().fg(t.muted)),
            Span::styled("Enter", Style::default().fg(t.primary)),
            Span::styled(" apply  ", Style::default().fg(t.muted)),
            Span::styled("Esc", Style::default().fg(t.primary)),
            Span::styled(" cancel", Style::default().fg(t.muted)),
        ]));

        render_modal(area, buf, Self::TITLE, lines, t);
    }
}
