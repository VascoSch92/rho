//! Agent Server HTTP and WebSocket client.
//!
//! Handles communication with the OpenHands Agent Server, including:
//! - HTTP API for conversation management
//! - WebSocket for real-time event streaming

pub mod api;
mod websocket;

pub use api::{
    AgentConfig, AgentServerClient, ExecutionStatus, LLMConfig, LocalWorkspace, SecurityAnalyzer,
    SendMessageRequest, ServerConfirmationPolicy, StartConversationRequest,
};
pub use websocket::EventStream;

use thiserror::Error;

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
