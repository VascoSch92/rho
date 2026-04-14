//! Settings modal widget with Basic / Advanced tabs.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::frame::render_modal;
use super::tabs::build_tabbed_lines;
use crate::config::theme::Theme;
use crate::state::{AppState, LlmProvider};

const TABS: &[&str] = &["Basic", "Advanced"];

/// Field IDs:
///   0 Provider, 1 Model, 2 API Key, 3 Base URL, 4 Custom Model,
///   5 LLM Timeout (seconds), 6 LLM Max Input Tokens,
///   7 Condenser Max Size, 8 Memory Condensation.
pub const BASIC_FIELDS: &[usize] = &[0, 1, 2];
pub const ADVANCED_FIELDS: &[usize] = &[4, 3, 5, 6, 7, 2, 8];

pub fn tab_fields(tab: usize) -> &'static [usize] {
    match tab {
        1 => ADVANCED_FIELDS,
        _ => BASIC_FIELDS,
    }
}

pub struct SettingsModal<'a> {
    state: &'a AppState,
}

impl<'a> SettingsModal<'a> {
    const TITLE: &'static str = "Settings";

    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    fn mask_api_key(key: &str) -> String {
        if key.is_empty() {
            "(not set)".to_string()
        } else if key.len() <= 4 {
            "*".repeat(key.len())
        } else {
            format!("{}{}", &key[..4], "*".repeat(key.len().min(12) - 4))
        }
    }

    /// Render a single field's line(s). Appends to `lines`.
    fn push_field(&self, lines: &mut Vec<Line<'static>>, field: usize, t: &Theme) {
        let selected = self.state.settings.field;
        let editing = self.state.settings.editing;
        let dropdown = self.state.settings.dropdown;
        let is_selected = selected == field;
        let label_style = field_label_style(is_selected, t);

        match field {
            0 => {
                lines.push(Line::from(vec![
                    Span::styled(
                        indicator(is_selected, &self.state.selector_indicator),
                        label_style,
                    ),
                    Span::styled("Provider:  ", label_style),
                    Span::styled(
                        self.state.llm.provider.display_name().to_string(),
                        Style::default().fg(t.foreground),
                    ),
                    if is_selected && !dropdown {
                        Span::styled("  Enter to select", Style::default().fg(t.muted))
                    } else {
                        Span::raw("")
                    },
                ]));
                if is_selected && dropdown {
                    let providers = LlmProvider::all();
                    let dd_indicator = format!(
                        "    {}",
                        crate::ui::formatting::selector_prefix(
                            true,
                            &self.state.selector_indicator
                        )
                    );
                    for (i, p) in providers.iter().enumerate() {
                        let is_dd_selected = i == self.state.settings.dropdown_selected;
                        let dd_style = if is_dd_selected {
                            Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(t.foreground)
                        };
                        let prefix = if is_dd_selected {
                            dd_indicator.clone()
                        } else {
                            "       ".to_string()
                        };
                        lines.push(Line::from(vec![
                            Span::raw(prefix),
                            Span::styled(p.display_name().to_string(), dd_style),
                        ]));
                    }
                }
            }
            1 => {
                let model_display = if self.state.llm.model.is_empty() {
                    "(select a model)".to_string()
                } else {
                    self.state.llm.model.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        indicator(is_selected, &self.state.selector_indicator),
                        label_style,
                    ),
                    Span::styled("Model:     ", label_style),
                    Span::styled(model_display, Style::default().fg(t.foreground)),
                    if is_selected && !dropdown {
                        Span::styled("  Enter to select", Style::default().fg(t.muted))
                    } else {
                        Span::raw("")
                    },
                ]));
                if is_selected && dropdown {
                    let models = self.state.llm.provider.models();
                    let dd_indicator = format!(
                        "    {}",
                        crate::ui::formatting::selector_prefix(
                            true,
                            &self.state.selector_indicator
                        )
                    );
                    for (i, m) in models.iter().enumerate() {
                        let is_dd_selected = i == self.state.settings.dropdown_selected;
                        let dd_style = if is_dd_selected {
                            Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(t.foreground)
                        };
                        let prefix = if is_dd_selected {
                            dd_indicator.clone()
                        } else {
                            "       ".to_string()
                        };
                        lines.push(Line::from(vec![
                            Span::raw(prefix),
                            Span::styled(m.to_string(), dd_style),
                        ]));
                    }
                }
            }
            2 => {
                let key_display = if is_selected && editing {
                    format!("{}_", &self.state.settings.edit_buffer)
                } else {
                    Self::mask_api_key(&self.state.llm.api_key)
                };
                let value_style = if is_selected && editing {
                    Style::default().fg(t.success)
                } else {
                    Style::default().fg(t.foreground)
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        indicator(is_selected, &self.state.selector_indicator),
                        label_style,
                    ),
                    Span::styled("API Key:   ", label_style),
                    Span::styled(key_display, value_style),
                    if is_selected && !editing {
                        Span::styled("  Enter to edit", Style::default().fg(t.muted))
                    } else {
                        Span::raw("")
                    },
                ]));
            }
            3 => self.push_text_field(
                lines,
                field,
                "Base URL:            ",
                self.state
                    .llm
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "(default)".to_string()),
                t,
            ),
            4 => self.push_text_field(
                lines,
                field,
                "Custom Model:        ",
                if self.state.llm.custom_model.is_empty() {
                    "(none)".to_string()
                } else {
                    self.state.llm.custom_model.clone()
                },
                t,
            ),
            5 => self.push_text_field(
                lines,
                field,
                "LLM Timeout (s):     ",
                self.state.llm.llm_timeout_seconds.to_string(),
                t,
            ),
            6 => self.push_text_field(
                lines,
                field,
                "Max Input Tokens:    ",
                match self.state.llm.llm_max_input_tokens {
                    Some(v) => v.to_string(),
                    None => "(unset)".to_string(),
                },
                t,
            ),
            7 => self.push_text_field(
                lines,
                field,
                "Condenser Max Size:  ",
                match self.state.llm.condenser_max_size {
                    Some(v) => v.to_string(),
                    None => "(unset)".to_string(),
                },
                t,
            ),
            8 => {
                let value_display = if self.state.llm.memory_condensation {
                    "On"
                } else {
                    "Off"
                };
                let value_style = if self.state.llm.memory_condensation {
                    Style::default().fg(t.success)
                } else {
                    Style::default().fg(t.muted)
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        indicator(is_selected, &self.state.selector_indicator),
                        label_style,
                    ),
                    Span::styled("Memory Condensation: ", label_style),
                    Span::styled(value_display.to_string(), value_style),
                    if is_selected {
                        Span::styled("  Enter to toggle", Style::default().fg(t.muted))
                    } else {
                        Span::raw("")
                    },
                ]));
            }
            _ => {}
        }
    }

    /// Render a text-editable field (used for fields 3-7). The currently-edited
    /// buffer takes precedence over `value` when the field is selected and the
    /// settings modal is in editing mode.
    fn push_text_field(
        &self,
        lines: &mut Vec<Line<'static>>,
        field: usize,
        label: &str,
        value: String,
        t: &Theme,
    ) {
        let selected = self.state.settings.field;
        let editing = self.state.settings.editing;
        let is_selected = selected == field;
        let label_style = field_label_style(is_selected, t);
        let display = if is_selected && editing {
            format!("{}_", &self.state.settings.edit_buffer)
        } else {
            value
        };
        let value_style = if is_selected && editing {
            Style::default().fg(t.success)
        } else {
            Style::default().fg(t.foreground)
        };
        lines.push(Line::from(vec![
            Span::styled(
                indicator(is_selected, &self.state.selector_indicator),
                label_style,
            ),
            Span::styled(label.to_string(), label_style),
            Span::styled(display, value_style),
            if is_selected && !editing {
                Span::styled("  Enter to edit", Style::default().fg(t.muted))
            } else {
                Span::raw("")
            },
        ]));
    }

    fn tab_lines(&self, fields: &[usize], t: &Theme) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        for (i, field) in fields.iter().enumerate() {
            self.push_field(&mut lines, *field, t);
            if i < fields.len() - 1 {
                lines.push(Line::from(""));
            }
        }
        lines
    }
}

impl Widget for SettingsModal<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.state.theme;
        let dropdown = self.state.settings.dropdown;
        let editing = self.state.settings.editing;

        let panes = vec![
            self.tab_lines(BASIC_FIELDS, t),
            self.tab_lines(ADVANCED_FIELDS, t),
        ];

        // Help line
        let footer_line = if dropdown {
            Line::from(vec![
                Span::styled("  ↑/↓", Style::default().fg(t.primary)),
                Span::styled(" navigate  ", Style::default().fg(t.muted)),
                Span::styled("Enter", Style::default().fg(t.primary)),
                Span::styled(" select  ", Style::default().fg(t.muted)),
                Span::styled("Esc", Style::default().fg(t.primary)),
                Span::styled(" cancel", Style::default().fg(t.muted)),
            ])
        } else if editing {
            Line::from(vec![
                Span::styled("  Type to edit  ", Style::default().fg(t.muted)),
                Span::styled("Enter", Style::default().fg(t.primary)),
                Span::styled(" save  ", Style::default().fg(t.muted)),
                Span::styled("Esc", Style::default().fg(t.primary)),
                Span::styled(" cancel", Style::default().fg(t.muted)),
            ])
        } else {
            Line::from(vec![
                Span::styled("  Tab", Style::default().fg(t.primary)),
                Span::styled("/", Style::default().fg(t.muted)),
                Span::styled("←→", Style::default().fg(t.primary)),
                Span::styled(" switch tab  ", Style::default().fg(t.muted)),
                Span::styled("↑/↓", Style::default().fg(t.primary)),
                Span::styled(" navigate  ", Style::default().fg(t.muted)),
                Span::styled("Enter", Style::default().fg(t.primary)),
                Span::styled(" select/edit  ", Style::default().fg(t.muted)),
                Span::styled("Esc", Style::default().fg(t.primary)),
                Span::styled(" close & save", Style::default().fg(t.muted)),
            ])
        };

        let lines = build_tabbed_lines(TABS, self.state.settings.tab, panes, vec![footer_line], t);
        render_modal(area, buf, Self::TITLE, lines, t);
    }
}

fn indicator(is_selected: bool, selector: &str) -> String {
    format!(
        " {}",
        crate::ui::formatting::selector_prefix(is_selected, selector)
    )
}

fn field_label_style(is_selected: bool, t: &crate::config::theme::Theme) -> Style {
    if is_selected {
        Style::default().fg(t.primary).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.muted)
    }
}
