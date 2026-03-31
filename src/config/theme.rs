//! Rho theme colors and styles.
//!
//! Themes are configurable via `--theme` CLI flag or `/theme` slash command.

use ratatui::style::Color;

/// Available theme names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum ThemeName {
    #[default]
    Rho,
    Dracula,
    Catppuccin,
    Tokyonight,
    Solarized,
    Gruvbox,
}

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeName::Rho => write!(f, "rho"),
            ThemeName::Dracula => write!(f, "dracula"),
            ThemeName::Catppuccin => write!(f, "catppuccin"),
            ThemeName::Tokyonight => write!(f, "tokyonight"),
            ThemeName::Solarized => write!(f, "solarized"),
            ThemeName::Gruvbox => write!(f, "gruvbox"),
        }
    }
}

impl std::str::FromStr for ThemeName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rho" => Ok(ThemeName::Rho),
            "dracula" => Ok(ThemeName::Dracula),
            "catppuccin" | "catppuccin-mocha" => Ok(ThemeName::Catppuccin),
            "tokyonight" | "tokyo-night" | "tokyo" => Ok(ThemeName::Tokyonight),
            "solarized" | "solarized-dark" => Ok(ThemeName::Solarized),
            "gruvbox" | "gruvbox-dark" => Ok(ThemeName::Gruvbox),
            _ => Err(format!("unknown theme: {}", s)),
        }
    }
}

impl ThemeName {
    pub fn all() -> &'static [ThemeName] {
        &[
            ThemeName::Rho,
            ThemeName::Dracula,
            ThemeName::Catppuccin,
            ThemeName::Tokyonight,
            ThemeName::Solarized,
            ThemeName::Gruvbox,
        ]
    }

    pub fn to_theme(self) -> Theme {
        match self {
            ThemeName::Rho => Theme::rho(),
            ThemeName::Dracula => Theme::dracula(),
            ThemeName::Catppuccin => Theme::catppuccin(),
            ThemeName::Tokyonight => Theme::tokyonight(),
            ThemeName::Solarized => Theme::solarized(),
            ThemeName::Gruvbox => Theme::gruvbox(),
        }
    }
}

/// A complete color theme for the TUI.
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

impl Theme {
    /// Default Rho theme (yellow accent on dark terminal).
    pub fn rho() -> Self {
        Self {
            primary: Color::Rgb(255, 225, 101),    // #ffe165
            accent: Color::Rgb(39, 125, 255),      // #277dff
            foreground: Color::Rgb(255, 255, 255), // #ffffff
            background: Color::Reset,
            muted: Color::Rgb(114, 121, 135),   // #727987
            border: Color::Rgb(80, 80, 80),     // #505050
            error: Color::Rgb(255, 107, 107),   // #ff6b6b
            success: Color::Rgb(107, 255, 107), // #6bff6b
        }
    }

    /// Dracula theme.
    pub fn dracula() -> Self {
        Self {
            primary: Color::Rgb(189, 147, 249),    // purple
            accent: Color::Rgb(139, 233, 253),     // cyan
            foreground: Color::Rgb(248, 248, 242), // fg
            background: Color::Reset,
            muted: Color::Rgb(98, 114, 164),   // comment
            border: Color::Rgb(68, 71, 90),    // current line
            error: Color::Rgb(255, 85, 85),    // red
            success: Color::Rgb(80, 250, 123), // green
        }
    }

    /// Catppuccin Mocha theme.
    pub fn catppuccin() -> Self {
        Self {
            primary: Color::Rgb(203, 166, 247),    // mauve
            accent: Color::Rgb(137, 180, 250),     // blue
            foreground: Color::Rgb(205, 214, 244), // text
            background: Color::Reset,
            muted: Color::Rgb(127, 132, 156),   // overlay0
            border: Color::Rgb(88, 91, 112),    // surface2
            error: Color::Rgb(243, 139, 168),   // red
            success: Color::Rgb(166, 227, 161), // green
        }
    }

    /// Tokyo Night theme.
    pub fn tokyonight() -> Self {
        Self {
            primary: Color::Rgb(122, 162, 247),    // blue
            accent: Color::Rgb(187, 154, 247),     // purple
            foreground: Color::Rgb(192, 202, 245), // fg
            background: Color::Reset,
            muted: Color::Rgb(86, 95, 137),     // comment
            border: Color::Rgb(61, 89, 161),    // dark blue
            error: Color::Rgb(247, 118, 142),   // red
            success: Color::Rgb(158, 206, 106), // green
        }
    }

    /// Solarized Dark theme.
    pub fn solarized() -> Self {
        Self {
            primary: Color::Rgb(181, 137, 0),      // yellow
            accent: Color::Rgb(38, 139, 210),      // blue
            foreground: Color::Rgb(131, 148, 150), // base0
            background: Color::Reset,
            muted: Color::Rgb(88, 110, 117),  // base01
            border: Color::Rgb(7, 54, 66),    // base02
            error: Color::Rgb(220, 50, 47),   // red
            success: Color::Rgb(133, 153, 0), // green
        }
    }

    /// Gruvbox Dark theme.
    pub fn gruvbox() -> Self {
        Self {
            primary: Color::Rgb(250, 189, 47),     // yellow
            accent: Color::Rgb(131, 165, 152),     // aqua
            foreground: Color::Rgb(235, 219, 178), // fg
            background: Color::Reset,
            muted: Color::Rgb(146, 131, 116),  // gray
            border: Color::Rgb(80, 73, 69),    // bg2
            error: Color::Rgb(251, 73, 52),    // red
            success: Color::Rgb(184, 187, 38), // green
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::rho()
    }
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

/// Spinner style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerStyle {
    Braille,
    BoxBuilding,
    ArrowSpin,
    BouncingBar,
}

impl SpinnerStyle {
    pub fn frames(&self) -> &'static [&'static str] {
        match self {
            SpinnerStyle::Braille => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"],
            SpinnerStyle::BoxBuilding => {
                &["▖", "▘", "▝", "▗", "▚", "▞", "█", "▞", "▚", "▗", "▝", "▘"]
            }
            SpinnerStyle::ArrowSpin => &["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
            SpinnerStyle::BouncingBar => &[
                "[=    ]", "[==   ]", "[===  ]", "[ === ]", "[  ===]", "[   ==]", "[    =]",
                "[   ==]", "[  ===]", "[ === ]", "[===  ]", "[==   ]",
            ],
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SpinnerStyle::Braille => SpinnerStyle::BoxBuilding,
            SpinnerStyle::BoxBuilding => SpinnerStyle::ArrowSpin,
            SpinnerStyle::ArrowSpin => SpinnerStyle::BouncingBar,
            SpinnerStyle::BouncingBar => SpinnerStyle::Braille,
        }
    }
}

/// Thinking status messages — open-source and dev themed
pub const FUN_FACTS: &[&str] = &[
    "Compiling ideas...",
    "Rebasing thoughts...",
    "Resolving merge conflicts in my brain...",
    "git blame: it was me all along",
    "Consulting the man pages...",
    "Parsing your intent...",
    "Running cargo build on a solution...",
    "Crafting artisanal bytes...",
    "Grepping the knowledge base...",
    "Forking a new thought process...",
    "Traversing the AST of possibilities...",
    "Borrowing ideas (don't worry, I'll return them)...",
    "Unwrapping Options...",
    "Pattern matching on your request...",
    "Spawning a background task...",
    "Allocating brain cycles...",
    "Piping stdout to my response buffer...",
    "chmod +x solution.sh...",
    "Reading the source, Luke...",
    "Talk is cheap. Generating the code.",
    "sudo think harder...",
    "Opening a PR against my own assumptions...",
    "Diffing reality vs expectations...",
    "Built with Ratatui + Rust + OpenHands...",
    "Free as in freedom, smart as in AI...",
    "Upstream looks good, merging thoughts...",
    "LGTM — Let's Go Think More...",
    "This commit will fix everything (famous last words)...",
];

/// Build styled spans for the thinking message with a 3-letter accent window that sweeps across.
/// The window moves one position per tick, cycling through the text.
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

    let window_pos = tick % (len + 3); // sweep past the end before restarting
    let window_size = 3;

    let mut spans = Vec::new();
    let mut i = 0;

    while i < len {
        let in_window = i >= window_pos.saturating_sub(0)
            && i < window_pos + window_size
            && (i as isize) >= (window_pos as isize);

        // Collect consecutive chars with the same style
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
