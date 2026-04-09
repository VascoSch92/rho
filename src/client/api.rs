//! HTTP API client for Agent Server.
//!
//! This client communicates with the OpenHands Agent Server REST API.
//! API endpoints are prefixed with `/api/` (e.g., `/api/conversations`).
//!
//! The SDK uses discriminated unions with a `kind` field to distinguish types.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ClientError, Result};

/// Conversation execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    #[default]
    Idle,
    Running,
    Paused,
    WaitingForConfirmation,
    Finished,
    Error,
}

/// Conversation info from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationInfo {
    pub id: Uuid,
    #[serde(default)]
    pub execution_status: ExecutionStatus,
    pub title: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// LLM configuration for the agent
/// Uses `kind: "LLM"` for discriminated union support
#[derive(Debug, Clone, Serialize)]
pub struct LLMConfig {
    /// Discriminator field for the SDK's DiscriminatedUnionMixin
    pub kind: &'static str,
    /// Usage identifier for the LLM service
    pub usage_id: String,
    pub model: String,
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl LLMConfig {
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            kind: "LLM",
            usage_id: "rho".to_string(),
            model: model.into(),
            api_key: api_key.into(),
            base_url: None,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }
}

/// Text content for messages
#[derive(Debug, Clone, Serialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: &'static str,
    pub text: String,
}

impl TextContent {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            content_type: "text",
            text: text.into(),
        }
    }
}

/// Send message request matching SDK's SendMessageRequest
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageRequest {
    pub role: &'static str,
    pub content: Vec<TextContent>,
    #[serde(default)]
    pub run: bool,
}

impl SendMessageRequest {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: "user",
            content: vec![TextContent::new(text)],
            run: false,
        }
    }

    pub fn with_run(mut self) -> Self {
        self.run = true;
        self
    }
}

/// LocalWorkspace configuration
/// Uses `kind: "LocalWorkspace"` for discriminated union support
#[derive(Debug, Clone, Serialize)]
pub struct LocalWorkspace {
    /// Discriminator field
    pub kind: &'static str,
    pub working_dir: String,
}

impl LocalWorkspace {
    pub fn new(working_dir: impl Into<String>) -> Self {
        Self {
            kind: "LocalWorkspace",
            working_dir: working_dir.into(),
        }
    }
}

/// Tool configuration with name
#[derive(Debug, Clone, Serialize)]
pub struct ToolConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl ToolConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Some(serde_json::json!({})),
        }
    }
}

/// Agent configuration
/// Uses `kind: "Agent"` for discriminated union support
#[derive(Debug, Clone, Serialize)]
pub struct AgentConfig {
    /// Discriminator field
    pub kind: &'static str,
    pub llm: LLMConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolConfig>>,
}

impl AgentConfig {
    /// Create agent config with default development tools
    /// Tool names are snake_case without _tool suffix (e.g., TerminalTool -> "terminal")
    pub fn with_default_tools(llm: LLMConfig) -> Self {
        Self {
            kind: "Agent",
            llm,
            tools: Some(vec![
                ToolConfig::new("terminal"),     // TerminalTool -> terminal
                ToolConfig::new("file_editor"),  // FileEditorTool -> file_editor
                ToolConfig::new("task_tracker"), // TaskTrackerTool -> task_tracker
            ]),
        }
    }
}

/// Confirmation policy for server-side action confirmation
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum ServerConfirmationPolicy {
    /// Never require confirmation
    NeverConfirm,
    /// Always require confirmation
    AlwaysConfirm,
    /// Only require confirmation for risky actions (medium/high security risk)
    ConfirmRisky,
}

/// Security analyzer discriminated union
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum SecurityAnalyzer {
    /// LLM-based security analyzer — reads risk from the LLM's tool call arguments
    LLMSecurityAnalyzer,
}

/// Start conversation request (matches SDK's StartConversationRequest)
#[derive(Debug, Serialize)]
pub struct StartConversationRequest {
    pub agent: AgentConfig,
    pub workspace: LocalWorkspace,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_message: Option<SendMessageRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_policy: Option<ServerConfirmationPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_analyzer: Option<SecurityAnalyzer>,
}

/// Health check response - server returns plain "OK" string
pub struct HealthResponse {}

// ── Skills ──────────────────────────────────────────────────────────────────

/// Request to list loaded skills.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SkillsRequest {
    pub load_public: bool,
    pub load_user: bool,
    pub load_project: bool,
    pub load_org: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_dir: Option<String>,
}

/// Skill metadata returned by `POST /skills`.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SkillInfo {
    pub name: String,
    #[serde(default, rename = "type")]
    pub skill_type: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_agentskills_format: Option<bool>,
}

/// Response from `POST /skills`.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillsResponse {
    #[serde(default)]
    pub skills: Vec<SkillInfo>,
    #[serde(default)]
    pub sources: std::collections::HashMap<String, u64>,
}

/// Agent Server HTTP client
#[derive(Clone)]
pub struct AgentServerClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl AgentServerClient {
    /// Create a new client
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
        }
    }

    /// Build request with optional API key header
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);
        if let Some(ref key) = self.api_key {
            req = req.header("X-Session-API-Key", key);
        }
        req
    }

    /// Check server health
    /// The server returns plain text "OK" on success
    pub async fn health(&self) -> Result<HealthResponse> {
        let resp = self.request(reqwest::Method::GET, "/health").send().await?;

        if !resp.status().is_success() {
            return Err(ClientError::Server {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        // Server returns plain "OK" string, not JSON
        let _text = resp.text().await?;
        Ok(HealthResponse {})
    }

    /// Start a new conversation with agent configuration.
    ///
    /// POSTs to `/api/conversations` with the full agent config (LLM, tools,
    /// workspace, initial message). Returns a `ConversationInfo` with the new
    /// conversation ID and optional title. The caller should then connect a
    /// WebSocket via `conversation_websocket_url()` to receive events.
    pub async fn start_conversation(
        &self,
        request: StartConversationRequest,
    ) -> Result<ConversationInfo> {
        // Debug: log the JSON being sent
        if let Ok(json) = serde_json::to_string_pretty(&request) {
            tracing::debug!("Sending conversation request:\n{}", json);
        }

        let resp = self
            .request(reqwest::Method::POST, "/api/conversations")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            tracing::error!("Failed to start conversation: {} - {}", status, message);
            return Err(ClientError::Server { status, message });
        }

        Ok(resp.json().await?)
    }

    /// Get full conversation state including stats/metrics
    /// Returns raw JSON to allow flexible parsing of nested stats
    pub async fn get_conversation_state(&self, id: Uuid) -> Result<serde_json::Value> {
        // Try the main conversation endpoint which should return full state
        let resp = self
            .request(reqwest::Method::GET, &format!("/api/conversations/{}", id))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ClientError::Server {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(resp.json().await?)
    }

    /// Send a message to the conversation
    /// Endpoint: POST /api/conversations/{id}/events
    pub async fn send_message(
        &self,
        conversation_id: Uuid,
        message: &str,
        run: bool,
    ) -> Result<()> {
        let msg = if run {
            SendMessageRequest::user(message).with_run()
        } else {
            SendMessageRequest::user(message)
        };

        tracing::debug!(
            "Sending message to conversation {}: run={}",
            conversation_id,
            run
        );

        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/api/conversations/{}/events", conversation_id),
            )
            .json(&msg)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            tracing::error!("Failed to send message: {} - {}", status, message);
            return Err(ClientError::Server { status, message });
        }

        Ok(())
    }

    /// Pause the conversation
    pub async fn pause_conversation(&self, conversation_id: Uuid) -> Result<()> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/api/conversations/{}/pause", conversation_id),
            )
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ClientError::Server {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }

    /// Respond to confirmation request (accept or reject pending actions)
    /// Uses the /events/respond_to_confirmation endpoint
    pub async fn respond_to_confirmation(
        &self,
        conversation_id: Uuid,
        accept: bool,
        reason: Option<&str>,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct ConfirmationResponse {
            accept: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            reason: Option<String>,
        }

        let req = ConfirmationResponse {
            accept,
            reason: reason.map(|s| s.to_string()),
        };

        tracing::debug!(
            "Responding to confirmation: accept={}, reason={:?}",
            accept,
            req.reason
        );

        let resp = self
            .request(
                reqwest::Method::POST,
                &format!(
                    "/api/conversations/{}/events/respond_to_confirmation",
                    conversation_id
                ),
            )
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ClientError::Server {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }

    /// Reject pending actions with an optional reason
    /// Convenience wrapper around respond_to_confirmation
    pub async fn reject_pending_actions(
        &self,
        conversation_id: Uuid,
        reason: Option<&str>,
    ) -> Result<()> {
        self.respond_to_confirmation(conversation_id, false, reason)
            .await
    }

    /// Accept pending actions
    /// Convenience wrapper around respond_to_confirmation
    pub async fn accept_pending_actions(&self, conversation_id: Uuid) -> Result<()> {
        self.respond_to_confirmation(conversation_id, true, None)
            .await
    }

    /// Get the WebSocket URL for a specific conversation's events
    /// The endpoint is at /sockets/events/{conversation_id}
    pub fn conversation_websocket_url(&self, conversation_id: Uuid) -> String {
        let ws_base = self
            .base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        format!("{}/sockets/events/{}", ws_base, conversation_id)
    }

    /// List loaded skills from the server.
    /// POSTs to `/api/skills` with load flags and an optional project directory.
    pub async fn list_skills(&self, request: SkillsRequest) -> Result<SkillsResponse> {
        let resp = self
            .request(reqwest::Method::POST, "/api/skills")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ClientError::Server {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(resp.json().await?)
    }

    /// Sync the public skills marketplace (git-pulls `OpenHands/extensions`).
    /// POSTs to `/api/skills/sync`.
    pub async fn sync_skills(&self) -> Result<()> {
        let resp = self
            .request(reqwest::Method::POST, "/api/skills/sync")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ClientError::Server {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }
}
