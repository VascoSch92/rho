//! Conversation scanning — reads stored conversations from the shared
//! conversations directory (`~/.openhands/conversations/` or `.rho/conversations/`).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::conversations_dir;
use crate::events::Event;

/// A conversation entry read from disk.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConversationEntry {
    pub id: String,
    pub title: String,
    pub first_message: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Raw meta.json structure (only the fields we need).
#[derive(Deserialize)]
struct MetaJson {
    id: Option<String>,
    title: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    initial_message: Option<InitialMessage>,
}

#[derive(Deserialize)]
struct InitialMessage {
    #[serde(default)]
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(default)]
    text: Option<String>,
}

/// Scan the conversations directory and return a list of entries, newest first.
///
/// Handles both rho conversations (which have `meta.json`) and openhands-cli
/// conversations (which only have `events/`). For the latter, metadata is
/// synthesised from the events directory.
pub fn scan_conversations() -> Vec<ConversationEntry> {
    let base = conversations_dir();
    if !base.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(&base) {
        for dir_entry in read_dir.flatten() {
            let conv_dir = dir_entry.path();
            if !conv_dir.is_dir() {
                continue;
            }
            // At minimum the conversation needs an events/ dir to be useful
            if !conv_dir.join("events").is_dir() {
                continue;
            }
            let entry = read_conversation_entry(&conv_dir, &dir_entry);
            entries.push(entry);
        }
    }

    // Sort by updated_at descending (newest first)
    entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    entries
}

/// Read a single conversation entry from a directory.
///
/// Prefers `meta.json` if present, otherwise synthesises metadata from
/// the filesystem and events.
fn read_conversation_entry(
    conv_dir: &Path,
    dir_entry: &std::fs::DirEntry,
) -> ConversationEntry {
    let dir_name = dir_entry.file_name().to_string_lossy().to_string();
    let meta_path = conv_dir.join("meta.json");

    if meta_path.exists() {
        if let Some(entry) = try_read_meta_json(&meta_path, &dir_name) {
            return entry;
        }
    }

    // Synthesise metadata from the filesystem
    let mtime = dir_entry
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        })
        .unwrap_or_default();

    let first_message = extract_first_user_message(conv_dir).unwrap_or_default();

    ConversationEntry {
        id: dir_name,
        title: "(untitled)".into(),
        first_message,
        created_at: mtime.clone(),
        updated_at: mtime,
    }
}

/// Try to read and parse a `meta.json` file.
fn try_read_meta_json(path: &Path, fallback_id: &str) -> Option<ConversationEntry> {
    let contents = std::fs::read_to_string(path).ok()?;
    let meta: MetaJson = serde_json::from_str(&contents).ok()?;

    let id = meta
        .id
        .unwrap_or_else(|| fallback_id.to_string());
    let title = meta.title.unwrap_or_else(|| "(untitled)".into());
    let first_message = meta
        .initial_message
        .and_then(|im| im.content.into_iter().find_map(|b| b.text))
        .unwrap_or_default();

    Some(ConversationEntry {
        id,
        title,
        first_message,
        created_at: meta.created_at.unwrap_or_default(),
        updated_at: meta.updated_at.unwrap_or_default(),
    })
}

/// Scan the first few event files looking for the first user message text.
fn extract_first_user_message(conv_dir: &Path) -> Option<String> {
    let events_dir = conv_dir.join("events");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&events_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    files.sort();

    // Only check the first 10 events to avoid scanning huge histories
    for file in files.into_iter().take(10) {
        let contents = std::fs::read_to_string(&file).ok()?;
        let val: serde_json::Value = serde_json::from_str(&contents).ok()?;
        if val.get("kind").and_then(|k| k.as_str()) == Some("MessageEvent")
            && val
                .get("source")
                .or_else(|| val.pointer("/base/source"))
                .and_then(|s| s.as_str())
                == Some("user")
        {
            // Extract text from llm_message.content[0].text
            if let Some(text) = val
                .pointer("/llm_message/content/0/text")
                .and_then(|t| t.as_str())
            {
                return Some(text.to_string());
            }
        }
    }
    None
}

/// Load events from a conversation's events/ directory, sorted by filename.
/// Returns parsed Event objects that can be replayed via state.process_event().
pub fn load_events(id: &str) -> Vec<Event> {
    let events_dir = conversations_dir().join(id).join("events");

    if !events_dir.is_dir() {
        return Vec::new();
    }

    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(&events_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                files.push(path);
            }
        }
    }

    // Sort by filename (event-00000-..., event-00001-..., etc.)
    files.sort();

    let mut events = Vec::new();
    for file in files {
        if let Ok(contents) = std::fs::read_to_string(&file) {
            match serde_json::from_str::<Event>(&contents) {
                Ok(event) => {
                    // Skip unknown/system events
                    if !matches!(event, Event::Unknown) {
                        events.push(event);
                    }
                }
                Err(_) => {
                    // Silently skip unparseable events
                }
            }
        }
    }

    events
}

/// Update the title in a conversation's meta.json.
///
/// Creates the file if it doesn't exist (openhands conversations lack meta.json).
pub fn update_title(id: &str, new_title: &str) -> Result<(), String> {
    let meta_path = conversations_dir().join(id).join("meta.json");

    let mut value: serde_json::Value = if meta_path.exists() {
        let contents =
            std::fs::read_to_string(&meta_path).map_err(|e| format!("Failed to read: {}", e))?;
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse: {}", e))?
    } else {
        serde_json::json!({ "id": id })
    };

    value["title"] = serde_json::Value::String(new_title.to_string());

    let updated =
        serde_json::to_string_pretty(&value).map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&meta_path, updated).map_err(|e| format!("Failed to write: {}", e))?;

    Ok(())
}

/// Delete a conversation directory from `~/.openhands/conversations/`.
pub fn delete_conversation(id: &str) -> Result<(), String> {
    let conv_dir = conversations_dir().join(id);
    if conv_dir.is_dir() {
        std::fs::remove_dir_all(&conv_dir).map_err(|e| format!("Failed to delete: {}", e))
    } else {
        Err("Conversation not found".into())
    }
}
