//! Key binding configuration — maps key sequences to actions.
//!
//! Key strings use a human-readable format:
//! - Simple keys: `"q"`, `"enter"`, `"esc"`, `"up"`, `"pageup"`, `"tab"`
//! - Modifiers: `"ctrl-q"`, `"alt-enter"`, `"shift-enter"`, `"ctrl-c"`
//! - Function keys: `"f1"` .. `"f12"`

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::de::Deserializer;
use serde::Deserialize;

/// Actions that can be bound to keys.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Global
    Quit,
    ForceQuit,
    ToggleCollapseAll,

    // Scrolling
    ScrollUp,
    ScrollDown,
    ScrollUpLarge,
    ScrollDownLarge,

    // Cursor
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,

    // Input
    Submit,
    NewLine,
    Backspace,
    Delete,

    // Agent control
    Pause,

    // Modal navigation
    Dismiss,
    NavUp,
    NavDown,
    NavLeft,
    NavRight,
    Confirm,
    ConfirmAll,
    Reject,
}

/// Key bindings for a single mode, mapping key events to actions.
#[derive(Debug, Clone, Default)]
pub struct ModeBindings(pub HashMap<KeyEvent, Action>);

/// Complete key binding configuration across all modes.
#[derive(Debug, Clone, Default)]
pub struct KeyBindingsConfig {
    /// Global bindings that work in any mode
    pub global: ModeBindings,
    /// Normal input mode bindings
    pub normal: ModeBindings,
    /// Confirmation dialog bindings
    pub confirmation: ModeBindings,
    /// Modal navigation bindings (help, token, theme, policy)
    pub modal: ModeBindings,
}

/// TOML-level representation before parsing keys
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawKeyBindingsConfig {
    #[serde(default)]
    pub global: HashMap<String, Action>,
    #[serde(default)]
    pub normal: HashMap<String, Action>,
    #[serde(default)]
    pub confirmation: HashMap<String, Action>,
    #[serde(default)]
    pub modal: HashMap<String, Action>,
}

impl RawKeyBindingsConfig {
    pub fn into_keybindings(self) -> KeyBindingsConfig {
        KeyBindingsConfig {
            global: parse_mode_bindings(self.global),
            normal: parse_mode_bindings(self.normal),
            confirmation: parse_mode_bindings(self.confirmation),
            modal: parse_mode_bindings(self.modal),
        }
    }
}

fn parse_mode_bindings(raw: HashMap<String, Action>) -> ModeBindings {
    let mut map = HashMap::new();
    for (key_str, action) in raw {
        match parse_key_event(&key_str) {
            Ok(key_event) => {
                map.insert(key_event, action);
            }
            Err(e) => {
                tracing::warn!("Invalid key binding '{}': {}", key_str, e);
            }
        }
    }
    ModeBindings(map)
}

impl<'de> Deserialize<'de> for KeyBindingsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawKeyBindingsConfig::deserialize(deserializer)?;
        Ok(raw.into_keybindings())
    }
}

impl ModeBindings {
    pub fn get(&self, key: &KeyEvent) -> Option<&Action> {
        self.0.get(key)
    }
}

// ── Key string parsing ──────────────────────────────────────────────────────

/// Parse a key string like "ctrl-q", "enter", "pageup", "f1" into a KeyEvent.
pub fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.to_ascii_lowercase();
    // Strip angle brackets if present: "<ctrl-q>" -> "ctrl-q"
    let stripped = raw_lower
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(&raw_lower);

    let (remaining, modifiers) = extract_modifiers(stripped);
    parse_key_code_with_modifiers(remaining, modifiers)
}

fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut current = raw;

    loop {
        if let Some(rest) = current.strip_prefix("ctrl-") {
            modifiers.insert(KeyModifiers::CONTROL);
            current = rest;
        } else if let Some(rest) = current.strip_prefix("alt-") {
            modifiers.insert(KeyModifiers::ALT);
            current = rest;
        } else if let Some(rest) = current.strip_prefix("shift-") {
            modifiers.insert(KeyModifiers::SHIFT);
            current = rest;
        } else {
            break;
        }
    }

    (current, modifiers)
}

fn parse_key_code_with_modifiers(
    raw: &str,
    mut modifiers: KeyModifiers,
) -> Result<KeyEvent, String> {
    let code = match raw {
        "esc" | "escape" => KeyCode::Esc,
        "enter" | "return" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backtab" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "tab" => KeyCode::Tab,
        "space" => KeyCode::Char(' '),
        "minus" | "hyphen" => KeyCode::Char('-'),
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        c if c.len() == 1 => {
            let mut ch = c.chars().next().unwrap();
            if modifiers.contains(KeyModifiers::SHIFT) {
                ch = ch.to_ascii_uppercase();
            }
            KeyCode::Char(ch)
        }
        _ => return Err(format!("unknown key: '{}'", raw)),
    };
    Ok(KeyEvent::new(code, modifiers))
}

/// Convert a KeyEvent back to a human-readable string (for display/help).
#[allow(dead_code)]
pub fn key_event_to_string(key: &KeyEvent) -> String {
    let mut parts = Vec::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("shift");
    }

    let key_name = match key.code {
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::BackTab => "backtab".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::F(n) => format!("f{}", n),
        _ => "?".to_string(),
    };

    if parts.is_empty() {
        key_name
    } else {
        parts.push(&key_name);
        parts.join("-")
    }
}

