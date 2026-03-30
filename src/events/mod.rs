//! Event types that mirror the OpenHands SDK event system.
//!
//! These types are deserialized from the Agent Server WebSocket stream.
//! The SDK uses `kind` field for discriminated unions.

use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

/// Deserialize a field that can be a string, null, or empty array
fn deserialize_string_or_empty<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrArray {
        String(String),
        Array(Vec<serde_json::Value>),
        Null,
    }

    match StringOrArray::deserialize(deserializer)? {
        StringOrArray::String(s) if !s.is_empty() => Ok(Some(s)),
        StringOrArray::String(_) => Ok(None),
        StringOrArray::Array(_) => Ok(None),
        StringOrArray::Null => Ok(None),
    }
}

/// Base event structure - all events share these fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBase {
    #[serde(default)]
    pub id: Option<String>, // Can be UUID or ULID string
    #[serde(default)]
    pub timestamp: Option<String>, // ISO timestamp string
    #[serde(default)]
    pub source: Option<String>,
}

/// Security risk levels for actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityRisk {
    #[default]
    Unknown,
    Low,
    Medium,
    High,
}

impl std::fmt::Display for SecurityRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityRisk::Unknown => write!(f, "UNKNOWN"),
            SecurityRisk::Low => write!(f, "LOW"),
            SecurityRisk::Medium => write!(f, "MEDIUM"),
            SecurityRisk::High => write!(f, "HIGH"),
        }
    }
}

/// Action event - represents an action the agent wants to take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEvent {
    #[serde(flatten)]
    pub base: EventBase,
    pub tool_call_id: String,
    pub tool_name: String,
    pub action: serde_json::Value,
    #[serde(default, deserialize_with = "deserialize_string_or_empty")]
    pub summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_empty")]
    pub thought: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_empty")]
    pub reasoning_content: Option<String>,
    pub security_risk: Option<SecurityRisk>,
}

/// Observation event - result of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationEvent {
    #[serde(flatten)]
    pub base: EventBase,
    pub tool_call_id: String,
    pub tool_name: String,
    pub action_id: Option<Uuid>,
    pub observation: serde_json::Value,
}

/// Message event - chat messages between user and agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEvent {
    #[serde(flatten)]
    pub base: EventBase,
    pub llm_message: Option<LLMMessage>,
    pub sender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: Vec<MessageContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

impl MessageEvent {
    pub fn get_text(&self) -> Option<String> {
        self.llm_message.as_ref().and_then(|msg| {
            msg.content
                .iter()
                .filter_map(|c| match c {
                    MessageContent::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .next()
        })
    }
}

/// Agent error event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentErrorEvent {
    #[serde(flatten)]
    pub base: EventBase,
    pub error: String,
}

/// Conversation state update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationStateUpdateEvent {
    #[serde(flatten)]
    pub base: EventBase,
    pub key: String,
    pub value: serde_json::Value,
}

/// Pause event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseEvent {
    #[serde(flatten)]
    pub base: EventBase,
}

/// User rejection observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRejectObservation {
    #[serde(flatten)]
    pub base: EventBase,
    pub action_id: Option<Uuid>,
    pub rejection_reason: String,
    pub tool_call_id: String,
    pub tool_name: String,
}

/// System prompt event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPromptEvent {
    #[serde(flatten)]
    pub base: EventBase,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<serde_json::Value>>,
}

/// Condensation event (history condensation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondensationEvent {
    #[serde(flatten)]
    pub base: EventBase,
    pub summary: Option<String>,
    pub forgotten_event_ids: Option<Vec<Uuid>>,
}

/// Unified event enum for all event types
/// SDK uses "kind" field for discriminated unions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Event {
    ActionEvent(ActionEvent),
    ObservationEvent(ObservationEvent),
    MessageEvent(MessageEvent),
    AgentErrorEvent(AgentErrorEvent),
    ConversationStateUpdateEvent(ConversationStateUpdateEvent),
    PauseEvent(PauseEvent),
    UserRejectObservation(UserRejectObservation),
    SystemPromptEvent(SystemPromptEvent),
    Condensation(CondensationEvent),
    TokenEvent(TokenEvent),
    #[serde(other)]
    Unknown,
}

/// Token event for streaming token updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEvent {
    #[serde(flatten)]
    pub base: EventBase,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub usage_id: Option<String>,
}

impl Event {
    /// Get the event ID if available
    pub fn id(&self) -> Option<&str> {
        match self {
            Event::ActionEvent(e) => e.base.id.as_deref(),
            Event::ObservationEvent(e) => e.base.id.as_deref(),
            Event::MessageEvent(e) => e.base.id.as_deref(),
            Event::AgentErrorEvent(e) => e.base.id.as_deref(),
            Event::ConversationStateUpdateEvent(e) => e.base.id.as_deref(),
            Event::PauseEvent(e) => e.base.id.as_deref(),
            Event::UserRejectObservation(e) => e.base.id.as_deref(),
            Event::SystemPromptEvent(e) => e.base.id.as_deref(),
            Event::Condensation(e) => e.base.id.as_deref(),
            Event::TokenEvent(e) => e.base.id.as_deref(),
            Event::Unknown => None,
        }
    }

    /// Get the timestamp if available
    pub fn timestamp(&self) -> Option<&str> {
        match self {
            Event::ActionEvent(e) => e.base.timestamp.as_deref(),
            Event::ObservationEvent(e) => e.base.timestamp.as_deref(),
            Event::MessageEvent(e) => e.base.timestamp.as_deref(),
            Event::AgentErrorEvent(e) => e.base.timestamp.as_deref(),
            Event::ConversationStateUpdateEvent(e) => e.base.timestamp.as_deref(),
            Event::PauseEvent(e) => e.base.timestamp.as_deref(),
            Event::UserRejectObservation(e) => e.base.timestamp.as_deref(),
            Event::SystemPromptEvent(e) => e.base.timestamp.as_deref(),
            Event::Condensation(e) => e.base.timestamp.as_deref(),
            Event::TokenEvent(e) => e.base.timestamp.as_deref(),
            Event::Unknown => None,
        }
    }

    /// Get a display-friendly type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Event::ActionEvent(_) => "Action",
            Event::ObservationEvent(_) => "Observation",
            Event::MessageEvent(_) => "Message",
            Event::AgentErrorEvent(_) => "Error",
            Event::ConversationStateUpdateEvent(_) => "State",
            Event::PauseEvent(_) => "Pause",
            Event::UserRejectObservation(_) => "Rejected",
            Event::SystemPromptEvent(_) => "System",
            Event::Condensation(_) => "Condensed",
            Event::TokenEvent(_) => "Token",
            Event::Unknown => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_deserialization() {
        // Test with "kind" discriminator (SDK format)
        let json = r#"{
            "kind": "MessageEvent",
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "llm_message": {
                "role": "assistant",
                "content": [{"type": "text", "text": "Hello!"}]
            }
        }"#;

        let event: Event = serde_json::from_str(json).unwrap();
        assert!(matches!(event, Event::MessageEvent(_)));
    }

    #[test]
    fn test_state_event_deserialization() {
        let json = r#"{
            "kind": "ConversationStateUpdateEvent",
            "id": "test-id",
            "timestamp": "2026-03-24T09:07:51.808212",
            "source": "environment",
            "key": "full_state",
            "value": {}
        }"#;

        let event: Event = serde_json::from_str(json).unwrap();
        assert!(matches!(event, Event::ConversationStateUpdateEvent(_)));
    }

    #[test]
    fn test_unknown_event() {
        let json = r#"{"kind": "SomeNewEventType", "id": "123"}"#;
        let event: Event = serde_json::from_str(json).unwrap();
        assert!(matches!(event, Event::Unknown));
    }
}
