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

/// Persisted LLM settings (from config file).
#[derive(Debug, Clone, Default)]
pub struct LlmSettings {
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

/// Top-level configuration (runtime, fully resolved).
#[derive(Debug, Clone)]
pub struct RhoConfig {
    /// Pinned agent server version from config
    pub agent_server_version: String,
    /// LLM settings from config (lowest priority — CLI/env override)
    pub llm: LlmSettings,
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
    /// Selector indicator symbol (shown next to selected items in modals)
    pub selector_indicator: String,
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
    agent_server: RawAgentServerConfig,
    #[serde(default)]
    llm: RawLlmConfig,
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
    #[serde(default)]
    ui: RawUiConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawAgentServerConfig {
    #[serde(default)]
    version: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawLlmConfig {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
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
struct RawUiConfig {
    #[serde(default)]
    selector_indicator: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawScrollConfig {
    #[serde(default)]
    lines: Option<usize>,
    #[serde(default)]
    lines_large: Option<usize>,
}

impl RhoConfig {
    /// Load config: parse embedded defaults, layer openhands, then user TOML.
    ///
    /// Priority for LLM settings (highest wins):
    ///   `~/.rho/config.toml [llm]`  >  `~/.rho/agent_settings.json`  >  embedded defaults
    pub fn load() -> Self {
        // Parse embedded defaults (must succeed)
        let defaults_raw: RawConfig =
            toml::from_str(DEFAULT_CONFIG).expect("embedded config.toml must parse");

        let config_path = config_file_path();
        let mut config = if let Some(path) = &config_path {
            if path.exists() {
                info!("Loading config from {}", path.display());
                match std::fs::read_to_string(path) {
                    Ok(contents) => match toml::from_str::<RawConfig>(&contents) {
                        Ok(user_raw) => Self::from_raw(defaults_raw, Some(user_raw)),
                        Err(e) => {
                            warn!("Failed to parse config file: {}", e);
                            Self::from_raw(defaults_raw, None)
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read config file: {}", e);
                        Self::from_raw(defaults_raw, None)
                    }
                }
            } else {
                debug!("No config file at {}", path.display());
                Self::from_raw(defaults_raw, None)
            }
        } else {
            Self::from_raw(defaults_raw, None)
        };

        // Layer openhands LLM settings underneath the TOML config:
        // only fill in fields that are still empty (TOML wins if set).
        if let Some(oh_llm) = load_openhands_llm() {
            if config.llm.model.is_none() {
                config.llm.model = oh_llm.model;
            }
            if config.llm.api_key.is_none() {
                config.llm.api_key = oh_llm.api_key;
            }
            if config.llm.base_url.is_none() {
                config.llm.base_url = oh_llm.base_url;
            }
        }

        config
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

        // ── LLM ──────────────────────────────────────────────────────────
        let llm = {
            let base = defaults.llm;
            if let Some(ref u) = user {
                LlmSettings {
                    model: u.llm.model.clone().or(base.model),
                    api_key: u.llm.api_key.clone().or(base.api_key),
                    base_url: u.llm.base_url.clone().or(base.base_url),
                }
            } else {
                LlmSettings {
                    model: base.model,
                    api_key: base.api_key,
                    base_url: base.base_url,
                }
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

        // ── UI ────────────────────────────────────────────────────────────
        let selector_indicator = user
            .as_ref()
            .and_then(|u| u.ui.selector_indicator.clone())
            .or(defaults.ui.selector_indicator)
            .unwrap_or_else(|| "❯".into());

        // ── Agent Server ─────────────────────────────────────────────────
        let agent_server_version = defaults
            .agent_server
            .version
            .unwrap_or_else(|| "0.0.0".into());

        Self {
            agent_server_version,
            llm,
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
            selector_indicator,
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

/// Get the path to the user config file: `~/.rho/config.toml`.
pub fn config_file_path() -> Option<PathBuf> {
    rho_dir().map(|d| d.join("config.toml"))
}

/// Save LLM settings to the user config file using toml_edit for surgical writes.
/// Creates the file and parent directories if they don't exist.
pub fn save_llm(model: &str, api_key: &str, base_url: Option<&str>) -> Result<(), String> {
    let path = config_file_path().ok_or("Could not determine config directory")?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Read existing file or start with empty document
    let contents = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Ensure [llm] table exists
    if !doc.contains_key("llm") {
        doc["llm"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    // Update fields
    doc["llm"]["model"] = toml_edit::value(model);
    doc["llm"]["api_key"] = toml_edit::value(api_key);
    match base_url {
        Some(url) if !url.is_empty() => {
            doc["llm"]["base_url"] = toml_edit::value(url);
        }
        _ => {
            doc["llm"]["base_url"] = toml_edit::value("");
        }
    }

    std::fs::write(&path, doc.to_string())
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    tracing::info!("Saved LLM settings to {}", path.display());

    // Also write to ~/.rho/agent_settings.json so openhands-cli stays in sync
    if let Err(e) = save_openhands_llm(model, api_key, base_url) {
        tracing::warn!("Failed to sync LLM settings to OpenHands: {}", e);
    }

    Ok(())
}

/// Save the active theme name to the user config file.
pub fn save_theme(theme_name: &str) -> Result<(), String> {
    let path = config_file_path().ok_or("Could not determine config directory")?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let contents = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    if !doc.contains_key("theme") {
        doc["theme"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    doc["theme"]["name"] = toml_edit::value(theme_name);

    std::fs::write(&path, doc.to_string())
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    tracing::info!("Saved theme '{}' to {}", theme_name, path.display());
    Ok(())
}

// ── OpenHands interoperability ─────────────────────────────────────────────
//
// Rho stores all persistent data under `~/.rho/`. LLM settings come from
// `agent_settings.json` and conversations live in `~/.rho/conversations/`.

/// Return the `~/.rho` directory, if the home dir can be determined.
pub fn rho_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".rho"))
}

/// Return the conversations directory: `~/.rho/conversations/`.
pub fn conversations_dir() -> PathBuf {
    rho_dir()
        .unwrap_or_else(|| PathBuf::from(".rho"))
        .join("conversations")
}

/// Return the data directory used as the agent server's working directory.
///
/// Always uses `~/.rho`, creating it if it doesn't exist.
pub fn data_dir() -> PathBuf {
    let dir = rho_dir().unwrap_or_else(|| PathBuf::from(".rho"));
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Load LLM settings from `~/.rho/agent_settings.json`.
///
/// Returns `None` if the file doesn't exist or fails to parse.
pub fn load_openhands_llm() -> Option<LlmSettings> {
    let path = rho_dir()?.join("agent_settings.json");
    let contents = std::fs::read_to_string(&path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&contents).ok()?;

    let llm = val.get("llm")?;
    let model = llm.get("model").and_then(|v| v.as_str()).map(String::from);
    let api_key = llm
        .get("api_key")
        .and_then(|v| v.as_str())
        .map(String::from);
    let base_url = llm
        .get("base_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    tracing::debug!("Loaded OpenHands LLM settings: model={:?}", model);
    Some(LlmSettings {
        model,
        api_key,
        base_url,
    })
}

/// Write LLM settings back to `~/.rho/agent_settings.json`.
///
/// Uses read-modify-write with `serde_json::Value` to preserve all other
/// fields in the file (tools, condenser, etc.).
pub fn save_openhands_llm(
    model: &str,
    api_key: &str,
    base_url: Option<&str>,
) -> Result<(), String> {
    let Some(dir) = rho_dir() else {
        return Ok(()); // No ~/.rho — nothing to write
    };
    let path = dir.join("agent_settings.json");
    let mut val: serde_json::Value = if path.exists() {
        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read agent_settings.json: {}", e))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse agent_settings.json: {}", e))?
    } else {
        serde_json::json!({})
    };

    // Ensure the "llm" object exists
    if !val.get("llm").is_some_and(|v| v.is_object()) {
        val["llm"] = serde_json::json!({});
    }
    val["llm"]["model"] = serde_json::json!(model);
    val["llm"]["api_key"] = serde_json::json!(api_key);
    val["llm"]["base_url"] = serde_json::json!(base_url.unwrap_or(""));

    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create ~/.rho: {}", e))?;
    let json = serde_json::to_string_pretty(&val)
        .map_err(|e| format!("Failed to serialize agent_settings.json: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write agent_settings.json: {}", e))?;

    tracing::info!("Saved OpenHands LLM settings to {}", path.display());
    Ok(())
}
