//! Rho theme colors and UI helpers.
//!
//! All theme definitions, spinner frames, and fun facts live in `config.toml`.
//! This module provides the runtime types and parsing utilities.

use ratatui::style::Color;
use serde::Deserialize;

/// A complete color theme for the TUI (runtime representation).
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Primary color - branding, cursors, highlights
    pub primary: Color,
    /// Accent color - links, special text, modal borders
    pub accent: Color,
    /// Foreground color - default text
    pub foreground: Color,
    /// Background color - terminal default
    pub background: Color,
    /// Muted color - placeholder text, secondary info
    pub muted: Color,
    /// Border color - panel borders
    pub border: Color,
    /// Error color - errors, dangerous actions
    pub error: Color,
    /// Success color - checkmarks, positive indicators
    pub success: Color,
}

impl Default for Theme {
    fn default() -> Self {
        // Hardcoded rho fallback — only used if config.toml is completely broken.
        Self {
            primary: Color::Rgb(255, 225, 101),
            accent: Color::Rgb(39, 125, 255),
            foreground: Color::Rgb(255, 255, 255),
            background: Color::Reset,
            muted: Color::Rgb(114, 121, 135),
            border: Color::Rgb(80, 80, 80),
            error: Color::Rgb(255, 107, 107),
            success: Color::Rgb(107, 255, 107),
        }
    }
}

/// Deserializable theme definition using hex color strings.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeColors {
    pub primary: String,
    pub accent: String,
    pub foreground: String,
    #[serde(default = "default_background")]
    pub background: String,
    pub muted: String,
    pub border: String,
    pub error: String,
    pub success: String,
}

fn default_background() -> String {
    "reset".to_string()
}

impl ThemeColors {
    /// Convert to a runtime Theme by parsing hex color strings.
    pub fn to_theme(&self) -> Result<Theme, String> {
        Ok(Theme {
            primary: parse_hex_color(&self.primary)?,
            accent: parse_hex_color(&self.accent)?,
            foreground: parse_hex_color(&self.foreground)?,
            background: parse_hex_color(&self.background)?,
            muted: parse_hex_color(&self.muted)?,
            border: parse_hex_color(&self.border)?,
            error: parse_hex_color(&self.error)?,
            success: parse_hex_color(&self.success)?,
        })
    }
}

/// Parse a hex color string like "#ff6b6b" or "reset" into a ratatui Color.
pub fn parse_hex_color(s: &str) -> Result<Color, String> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("reset") {
        return Ok(Color::Reset);
    }
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() != 6 {
        return Err(format!("invalid hex color: {}", s));
    }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| format!("invalid hex color: {}", s))?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| format!("invalid hex color: {}", s))?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| format!("invalid hex color: {}", s))?;
    Ok(Color::Rgb(r, g, b))
}

/// Build the Rho ASCII banner with the version embedded above the "o".
pub fn rho_banner(version: &str) -> Vec<String> {
    vec![
        " ▄▄▄▄▄▄".to_string(),
        "█▀██▀▀▀█▄  █▄".to_string(),
        format!("  ██▄▄▄█▀  ██  v{}", version),
        "  ██▀▀█▄   ████▄ ▄███▄".to_string(),
        "▄ ██  ██   ██ ██ ██ ██".to_string(),
        "▀██▀  ▀██▀▄██ ██▄▀███▀".to_string(),
    ]
}

/// Build styled spans for the thinking message with a 3-letter accent window that sweeps across.
pub fn animated_thinking_spans(
    text: &str,
    tick: usize,
    theme: &Theme,
) -> Vec<ratatui::text::Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return vec![];
    }

    let window_pos = tick % (len + 3);
    let window_size = 3;

    let mut spans = Vec::new();
    let mut i = 0;

    while i < len {
        let in_window = i >= window_pos.saturating_sub(0)
            && i < window_pos + window_size
            && (i as isize) >= (window_pos as isize);

        let start = i;
        while i < len {
            let this_in_window = i >= window_pos && i < window_pos + window_size;
            if this_in_window != in_window {
                break;
            }
            i += 1;
        }

        let chunk: String = chars[start..i].iter().collect();
        let color = if in_window {
            theme.accent
        } else {
            theme.primary
        };
        spans.push(ratatui::text::Span::styled(
            chunk,
            ratatui::style::Style::default().fg(color),
        ));
    }

    spans
}
