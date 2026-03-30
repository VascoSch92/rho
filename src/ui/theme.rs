//! Rho theme colors and styles.
//!
//! Themes are configurable via `--theme` CLI flag or `/theme` slash command.

use ratatui::style::Color;

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
    /// Warning color - caution indicators
    pub warning: Color,
}

impl Theme {
    /// List all available theme names.
    pub fn available() -> &'static [&'static str] {
        &[
            "rho",
            "dracula",
            "catppuccin",
            "tokyonight",
            "solarized",
            "gruvbox",
        ]
    }

    /// Get a theme by name (case-insensitive). Returns default if unknown.
    pub fn by_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "dracula" => Self::dracula(),
            "catppuccin" | "catppuccin-mocha" => Self::catppuccin(),
            "tokyonight" | "tokyo-night" | "tokyo" => Self::tokyonight(),
            "solarized" | "solarized-dark" => Self::solarized(),
            "gruvbox" | "gruvbox-dark" => Self::gruvbox(),
            _ => Self::default(),
        }
    }

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
            warning: Color::Rgb(255, 225, 101), // #ffe165
        }
    }

    /// Dracula theme.
    pub fn dracula() -> Self {
        Self {
            primary: Color::Rgb(189, 147, 249),    // purple
            accent: Color::Rgb(139, 233, 253),     // cyan
            foreground: Color::Rgb(248, 248, 242), // fg
            background: Color::Reset,
            muted: Color::Rgb(98, 114, 164),    // comment
            border: Color::Rgb(68, 71, 90),     // current line
            error: Color::Rgb(255, 85, 85),     // red
            success: Color::Rgb(80, 250, 123),  // green
            warning: Color::Rgb(241, 250, 140), // yellow
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
            warning: Color::Rgb(249, 226, 175), // yellow
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
            warning: Color::Rgb(224, 175, 104), // yellow
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
            warning: Color::Rgb(203, 75, 22), // orange
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
            warning: Color::Rgb(254, 128, 25), // orange
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::rho()
    }
}

/// Rho full ASCII banner for splash screen
pub const RHO_BANNER: &[&str] = &[
    r"    ____  _           ",
    r"   |  _ \| |__   ___  ",
    r"   | |_) | '_ \ / _ \ ",
    r"   |  _ <| | | | (_) |",
    r"   |_| \_\_| |_|\___/ ",
    r"                      ",
];

/// Rho compact logo (for header)
pub const RHO_LOGO: &[&str] = &[r"|  _ \ ", r"| |_) |", r"|  _ < ", r"|_| \_\"];

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

    pub fn random_with_seed(seed: usize) -> Self {
        match seed % 4 {
            0 => SpinnerStyle::Braille,
            1 => SpinnerStyle::BoxBuilding,
            2 => SpinnerStyle::ArrowSpin,
            _ => SpinnerStyle::BouncingBar,
        }
    }

    pub fn all() -> [SpinnerStyle; 4] {
        [
            SpinnerStyle::Braille,
            SpinnerStyle::BoxBuilding,
            SpinnerStyle::ArrowSpin,
            SpinnerStyle::BouncingBar,
        ]
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

/// Fun facts and dev jokes to display while waiting
pub const FUN_FACTS: &[&str] = &[
    // Dev jokes
    "Why do programmers prefer dark mode? Because light attracts bugs 🐛",
    "There are only 10 types of people: those who understand binary...",
    "It works on my machine! 🤷",
    "// TODO: write a better TODO",
    "99 little bugs in the code... patch one down, 127 bugs around 🐜",
    "A SQL query walks into a bar, walks up to two tables and asks: 'Can I join you?'",
    "!false — It's funny because it's true",
    "Programming is 10% writing code and 90% figuring out why it doesn't work",
    "The best thing about a boolean is even if you're wrong, you're only off by a bit",
    "Debugging: Being the detective in a crime movie where you're also the murderer 🔍",
    "I don't always test my code, but when I do, I do it in production 🚀",
    "git commit -m 'I have no idea what I just did'",
    "Semicolons: The difference between 'Hello World' and 'Hello; World'",
    "In theory, theory and practice are the same. In practice, they're not.",
    "Talk is cheap. Show me the code. — Linus Torvalds",
    // Rho facts
    "Rho (ρ) is the 17th letter of the Greek alphabet 🔤",
    "This TUI is built with Ratatui 🐀 + Rust 🦀",
    "Rho connects to the OpenHands Agent Server 🔧",
    "Did you know? The agent can browse the web for you 🌐",
    "Pro tip: Use /policy to control agent autonomy 🎮",
    "Fun fact: This Rust TUI connects to a Python backend 🦀🐍",
    // Motivational
    "Sit back, relax, code is being written ✨",
    "Pair programming with AI: the future is now 🤖",
    "You're doing great! Let the AI handle the boring stuff 💪",
];
