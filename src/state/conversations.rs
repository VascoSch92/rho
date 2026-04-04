//! Conversation scanning — reads stored conversations from .rho/conversations/.

use std::path::Path;

use serde::Deserialize;

use crate::events::Event;

/// A conversation entry read from meta.json.
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

/// Scan .rho/conversations/ and return a list of conversations, newest first.
pub fn scan_conversations() -> Vec<ConversationEntry> {
    let rho_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".rho/conversations");
    if !rho_dir.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(&rho_dir) {
        for dir_entry in read_dir.flatten() {
            let meta_path = dir_entry.path().join("meta.json");
            if !meta_path.exists() {
                continue;
            }
            if let Ok(contents) = std::fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<MetaJson>(&contents) {
                    let id = meta
                        .id
                        .unwrap_or_else(|| dir_entry.file_name().to_string_lossy().to_string());
                    let title = meta.title.unwrap_or_else(|| "(untitled)".into());
                    let first_message = meta
                        .initial_message
                        .and_then(|im| im.content.into_iter().find_map(|b| b.text))
                        .unwrap_or_default();
                    entries.push(ConversationEntry {
                        id,
                        title,
                        first_message,
                        created_at: meta.created_at.unwrap_or_default(),
                        updated_at: meta.updated_at.unwrap_or_default(),
                    });
                }
            }
        }
    }

    // Sort by updated_at descending (newest first)
    entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    entries
}

/// Load events from a conversation's events/ directory, sorted by filename.
/// Returns parsed Event objects that can be replayed via state.process_event().
pub fn load_events(id: &str) -> Vec<Event> {
    let events_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".rho/conversations")
        .join(id)
        .join("events");

    if !events_dir.is_dir() {
        return Vec::new();
    }

    let mut files: Vec<std::path::PathBuf> = Vec::new();
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

/// Delete a conversation directory from .rho/conversations/.
pub fn delete_conversation(id: &str) -> Result<(), String> {
    let conv_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".rho/conversations")
        .join(id);
    if conv_dir.is_dir() {
        std::fs::remove_dir_all(&conv_dir).map_err(|e| format!("Failed to delete: {}", e))
    } else {
        Err("Conversation not found".into())
    }
}
