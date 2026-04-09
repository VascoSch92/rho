//! Agent Server HTTP and WebSocket client.
//!
//! Handles communication with the OpenHands Agent Server, including:
//! - HTTP API for conversation management
//! - WebSocket for real-time event streaming

pub mod api;
mod websocket;

pub use api::{
    AgentConfig, AgentServerClient, ExecutionStatus, LLMConfig, LocalWorkspace, SecurityAnalyzer,
    SendMessageRequest, ServerConfirmationPolicy, SkillInfo, SkillsRequest,
    StartConversationRequest,
};
pub use websocket::EventStream;

use thiserror::Error;
use tracing::{info, warn};

/// Attempt to connect a WebSocket event stream for a conversation.
///
/// Centralizes URL building, connect, and structured logging for the three
/// connect call sites (resume, reconnect, lazy). Each call site decides what
/// to do with the result — this helper only connects and logs.
///
/// `context` is a short label included in log messages so users can tell
/// which call site the log came from.
pub async fn try_connect_event_stream(
    client: &AgentServerClient,
    conv_id: uuid::Uuid,
    context: &str,
) -> Option<EventStream> {
    let ws_url = client.conversation_websocket_url(conv_id);
    match EventStream::connect(&ws_url).await {
        Ok(stream) => {
            info!("WebSocket connected ({})", context);
            Some(stream)
        }
        Err(e) => {
            warn!("Failed to connect WebSocket ({}): {}", context, e);
            None
        }
    }
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, ClientError>;
