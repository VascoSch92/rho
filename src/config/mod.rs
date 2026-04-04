//! Configuration — theme, keybindings, spinner, fun facts.
//!
//! The embedded `config.toml` is the single source of truth for all defaults.
//! User config at `~/.config/rho/config.toml` is merged on top.

pub mod keybindings;
pub mod theme;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::config::keybindings::{KeyBindingsConfig, RawKeyBindingsConfig};
use crate::config::theme::{Theme, ThemeColors};

/// Embedded default configuration — the single source of truth.
const DEFAULT_CONFIG: &str = include_str!("../../config.toml");

/// Top-level configuration (runtime, fully resolved).
#[derive(Debug, Clone)]
pub struct RhoConfig {
    /// Name of the active theme
    pub theme_name: String,
    /// All available themes, resolved to runtime Theme
    pub themes: HashMap<String, Theme>,
    /// Ordered list of theme names for the picker
    pub theme_names: Vec<String>,
    /// Active spinner style name
    pub spinner_style: String,
    /// All spinner styles: name → frames
    pub spinners: HashMap<String, Vec<String>>,
    /// Ordered list of spinner names (for cycling)
    pub spinner_names: Vec<String>,
    /// Thinking messages
    pub fun_facts: Vec<String>,
    /// Key bindings
    pub keybindings: KeyBindingsConfig,
    /// Scroll lines (arrow keys)
    pub scroll_lines: usize,
    /// Scroll lines (page up/down)
    pub scroll_lines_large: usize,
}

impl Default for RhoConfig {
    fn default() -> Self {
        match toml::from_str::<RawConfig>(DEFAULT_CONFIG) {
            Ok(raw) => Self::from_raw(raw, None),
            Err(e) => {
                // Embedded config must parse — this is a build-time bug.
                panic!("Failed to parse embedded config.toml: {}", e);
            }
        }
    }
}

/// TOML-level representation before converting to runtime config.
#[derive(Debug, Clone, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    theme: RawThemeConfig,
    #[serde(default)]
    spinner: RawSpinnerConfig,
    #[serde(default)]
    fun_facts: RawFunFactsConfig,
    #[serde(default)]
    keybindings: RawKeyBindingsConfig,
    #[serde(default)]
    scroll: RawScrollConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawThemeConfig {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    themes: HashMap<String, ThemeColors>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawSpinnerConfig {
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    styles: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawFunFactsConfig {
    #[serde(default)]
    messages: Option<Vec<String>>,
    #[serde(default)]
    append: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawScrollConfig {
    #[serde(default)]
    lines: Option<usize>,
    #[serde(default)]
    lines_large: Option<usize>,
}

impl RhoConfig {
    /// Load config: parse embedded defaults, then merge user config on top.
    pub fn load() -> Self {
        // Parse embedded defaults (must succeed)
        let defaults_raw: RawConfig =
            toml::from_str(DEFAULT_CONFIG).expect("embedded config.toml must parse");

        let config_path = config_file_path();
        if let Some(path) = &config_path {
            if path.exists() {
                info!("Loading config from {}", path.display());
                match std::fs::read_to_string(path) {
                    Ok(contents) => match toml::from_str::<RawConfig>(&contents) {
                        Ok(user_raw) => return Self::from_raw(defaults_raw, Some(user_raw)),
                        Err(e) => {
                            warn!("Failed to parse config file: {}", e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read config file: {}", e);
                    }
                }
            } else {
                debug!("No config file at {}", path.display());
            }
        }

        Self::from_raw(defaults_raw, None)
    }

    /// Resolve a theme name to a Theme, falling back to rho.
    pub fn resolve_theme(&self, name: &str) -> Theme {
        self.themes.get(name).copied().unwrap_or_default()
    }

    /// Build from parsed TOML. `user` overrides `defaults` where set.
    fn from_raw(defaults: RawConfig, user: Option<RawConfig>) -> Self {
        // ── Themes ───────────────────────────────────────────────────────
        let mut theme_defs = defaults.theme.themes;
        let mut theme_name = defaults.theme.name.unwrap_or_else(|| "rho".into());
        if let Some(ref u) = user {
            for (name, colors) in &u.theme.themes {
                theme_defs.insert(name.clone(), colors.clone());
            }
            if let Some(ref name) = u.theme.name {
                theme_name = name.clone();
            }
        }
        let (themes, theme_names) = resolve_themes(&theme_defs);

        // ── Spinners ─────────────────────────────────────────────────────
        let mut spinner_defs = defaults.spinner.styles;
        let mut spinner_style = defaults.spinner.style.unwrap_or_else(|| "braille".into());
        if let Some(ref u) = user {
            for (name, frames) in &u.spinner.styles {
                spinner_defs.insert(name.clone(), frames.clone());
            }
            if let Some(ref style) = u.spinner.style {
                spinner_style = style.clone();
            }
        }
        let spinner_names = build_spinner_names(&spinner_defs);

        // ── Fun facts ────────────────────────────────────────────────────
        let default_facts = defaults.fun_facts.messages.unwrap_or_default();
        let fun_facts = match user.as_ref().and_then(|u| u.fun_facts.messages.clone()) {
            Some(msgs) if !msgs.is_empty() => {
                if user.as_ref().is_some_and(|u| u.fun_facts.append) {
                    let mut combined = default_facts;
                    combined.extend(msgs);
                    combined
                } else {
                    msgs
                }
            }
            _ => default_facts,
        };

        // ── Keybindings ──────────────────────────────────────────────────
        let keybindings = {
            let base = defaults.keybindings.into_keybindings();
            if let Some(ref u) = user {
                let user_bindings = u.keybindings.clone().into_keybindings();
                merge_keybindings(base, user_bindings)
            } else {
                base
            }
        };

        // ── Scroll ───────────────────────────────────────────────────────
        let scroll_lines = user
            .as_ref()
            .and_then(|u| u.scroll.lines)
            .or(defaults.scroll.lines)
            .unwrap_or(3);
        let scroll_lines_large = user
            .as_ref()
            .and_then(|u| u.scroll.lines_large)
            .or(defaults.scroll.lines_large)
            .unwrap_or(10);

        Self {
            theme_name,
            themes,
            theme_names,
            spinner_style,
            spinners: spinner_defs,
            spinner_names,
            fun_facts,
            keybindings,
            scroll_lines,
            scroll_lines_large,
        }
    }
}

/// Resolve theme definitions into runtime Themes.
fn resolve_themes(defs: &HashMap<String, ThemeColors>) -> (HashMap<String, Theme>, Vec<String>) {
    let mut themes = HashMap::new();
    for (name, colors) in defs {
        match colors.to_theme() {
            Ok(theme) => {
                themes.insert(name.clone(), theme);
            }
            Err(e) => {
                warn!("Failed to parse theme '{}': {}", name, e);
            }
        }
    }

    // Build ordered list: known names first (in config insertion order is lost,
    // so sort alphabetically), then remaining.
    let mut names: Vec<String> = themes.keys().cloned().collect();
    names.sort();
    // Bump "rho" to front if present
    if let Some(pos) = names.iter().position(|n| n == "rho") {
        let rho = names.remove(pos);
        names.insert(0, rho);
    }

    (themes, names)
}

/// Build an ordered list of spinner names (alphabetical, "braille" first).
fn build_spinner_names(defs: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut names: Vec<String> = defs.keys().cloned().collect();
    names.sort();
    if let Some(pos) = names.iter().position(|n| n == "braille") {
        let braille = names.remove(pos);
        names.insert(0, braille);
    }
    names
}

/// Merge user keybindings on top of defaults (user wins on conflict).
fn merge_keybindings(defaults: KeyBindingsConfig, user: KeyBindingsConfig) -> KeyBindingsConfig {
    fn merge_mode(
        mut base: keybindings::ModeBindings,
        overlay: keybindings::ModeBindings,
    ) -> keybindings::ModeBindings {
        for (key, action) in overlay.0 {
            base.0.insert(key, action);
        }
        base
    }

    KeyBindingsConfig {
        global: merge_mode(defaults.global, user.global),
        normal: merge_mode(defaults.normal, user.normal),
        confirmation: merge_mode(defaults.confirmation, user.confirmation),
        modal: merge_mode(defaults.modal, user.modal),
    }
}

/// Get the path to the config file: `~/.config/rho/config.toml`
pub fn config_file_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "rho").map(|dirs| dirs.config_dir().join("config.toml"))
}
