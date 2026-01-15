use crate::{error::Result, session_events::SessionEvent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

/// Connection state of the Copilot client
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Client configuration options
#[derive(Debug, Clone, Default)]
pub struct ClientOptions {
    /// Path to the Copilot CLI executable (default: "copilot")
    pub cli_path: Option<String>,

    /// Working directory for the CLI process
    pub cwd: Option<String>,

    /// Server port for TCP transport (default: 0 = random port)
    pub port: Option<u16>,

    /// Use stdio transport instead of TCP (default: true)
    pub use_stdio: bool,

    /// URL of an existing Copilot CLI server to connect to over TCP
    /// Format: "host:port", "http://host:port", or just "port" (defaults to localhost)
    /// Examples: "localhost:8080", "http://127.0.0.1:9000", "8080"
    /// Mutually exclusive with cli_path and use_stdio
    pub cli_url: Option<String>,

    /// Log level for the CLI server (default: "info")
    pub log_level: Option<String>,

    /// Automatically start the CLI server on first use (default: true)
    pub auto_start: bool,

    /// Automatically restart the CLI server if it crashes (default: true)
    pub auto_restart: bool,

    /// Environment variables for the CLI process
    pub env: Option<Vec<(String, String)>>,
}

impl ClientOptions {
    pub fn new() -> Self {
        Self {
            cli_path: None,
            cwd: None,
            port: None,
            use_stdio: true,
            cli_url: None,
            log_level: None,
            auto_start: true,
            auto_restart: true,
            env: None,
        }
    }

    pub fn cli_path(mut self, path: impl Into<String>) -> Self {
        self.cli_path = Some(path.into());
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn use_stdio(mut self, use_stdio: bool) -> Self {
        self.use_stdio = use_stdio;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self.use_stdio = false;
        self
    }

    pub fn cli_url(mut self, url: impl Into<String>) -> Self {
        self.cli_url = Some(url.into());
        self.use_stdio = false;
        self
    }

    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = Some(level.into());
        self
    }

    pub fn auto_start(mut self, enabled: bool) -> Self {
        self.auto_start = enabled;
        self
    }

    pub fn auto_restart(mut self, enabled: bool) -> Self {
        self.auto_restart = enabled;
        self
    }

    pub fn env(mut self, env: Vec<(String, String)>) -> Self {
        self.env = Some(env);
        self
    }
}

/// System message configuration mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum SystemMessageConfig {
    /// Append additional content to the default system message
    #[serde(rename = "append")]
    Append {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
    },
    /// Replace the entire system message with custom content
    #[serde(rename = "replace")]
    Replace { content: String },
}

/// Permission request from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Result of a permission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestResult {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<serde_json::Value>>,
}

/// Context for a permission request invocation
#[derive(Debug, Clone)]
pub struct PermissionInvocation {
    pub session_id: String,
}

/// Handler for permission requests
pub type PermissionHandler = Arc<
    dyn Fn(PermissionRequest, PermissionInvocation) -> Result<PermissionRequestResult>
        + Send
        + Sync,
>;

/// MCP local/stdio server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpLocalServerConfig {
    pub tools: Vec<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// MCP remote server configuration (HTTP or SSE)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRemoteServerConfig {
    pub tools: Vec<String>,
    #[serde(rename = "type")]
    pub server_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// MCP server configuration (can be local or remote)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpServerConfig {
    Local(McpLocalServerConfig),
    Remote(McpRemoteServerConfig),
}

/// Custom agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAgentConfig {
    /// Unique name of the custom agent
    pub name: String,

    /// Display name for UI purposes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Description of what the agent does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// List of tool names the agent can use (None for all tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    /// Prompt content for the agent
    pub prompt: String,

    /// MCP servers specific to this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,

    /// Whether the agent should be available for model inference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infer: Option<bool>,
}

/// Session configuration for creating a new session
#[derive(Clone, Default)]
pub struct SessionConfig {
    /// Optional custom session ID
    pub session_id: Option<String>,

    /// Model to use for this session
    pub model: Option<String>,

    /// Tools to expose to the CLI
    pub tools: Vec<Tool>,

    /// System message customization
    pub system_message: Option<SystemMessageConfig>,

    /// List of tool names to allow (takes precedence over excluded_tools)
    pub available_tools: Option<Vec<String>>,

    /// List of tool names to disable
    pub excluded_tools: Option<Vec<String>>,

    /// Handler for permission requests
    pub on_permission_request: Option<PermissionHandler>,

    /// Enable streaming of assistant message and reasoning chunks
    pub streaming: bool,

    /// Custom model provider configuration
    pub provider: Option<ProviderConfig>,

    /// MCP servers configuration
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,

    /// Custom agents configuration
    pub custom_agents: Option<Vec<CustomAgentConfig>>,
}

/// Configuration for resuming an existing session
#[derive(Clone, Default)]
pub struct ResumeSessionConfig {
    /// Tools to expose to the CLI
    pub tools: Vec<Tool>,

    /// Custom model provider configuration
    pub provider: Option<ProviderConfig>,

    /// Handler for permission requests
    pub on_permission_request: Option<PermissionHandler>,

    /// Enable streaming of assistant message and reasoning chunks
    pub streaming: bool,

    /// MCP servers configuration
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,

    /// Custom agents configuration
    pub custom_agents: Option<Vec<CustomAgentConfig>>,
}

/// Tool definition that can be invoked by Copilot
#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub handler: ToolHandler,
}

impl std::fmt::Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters)
            .finish()
    }
}

/// Context for a tool invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub session_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// Handler for tool invocations
pub type ToolHandler = Arc<dyn Fn(ToolInvocation) -> Result<ToolResult> + Send + Sync>;

/// Result of a tool invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    #[serde(rename = "textResultForLlm")]
    pub text_result_for_llm: String,

    #[serde(
        rename = "binaryResultsForLlm",
        skip_serializing_if = "Option::is_none"
    )]
    pub binary_results_for_llm: Option<Vec<ToolBinaryResult>>,

    #[serde(rename = "resultType")]
    pub result_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    #[serde(rename = "sessionLog", skip_serializing_if = "Option::is_none")]
    pub session_log: Option<String>,

    #[serde(rename = "toolTelemetry", skip_serializing_if = "Option::is_none")]
    pub tool_telemetry: Option<HashMap<String, serde_json::Value>>,
}

impl ToolResult {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text_result_for_llm: text.into(),
            binary_results_for_llm: None,
            result_type: "success".to_string(),
            error: None,
            session_log: None,
            tool_telemetry: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            text_result_for_llm:
                "Invoking this tool produced an error. Detailed information is not available."
                    .to_string(),
            binary_results_for_llm: None,
            result_type: "failure".to_string(),
            error: Some(error.into()),
            session_log: None,
            tool_telemetry: None,
        }
    }
}

/// Binary payload returned by tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolBinaryResult {
    pub data: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "type")]
    pub result_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Custom model provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type: "openai", "azure", or "anthropic" (default: "openai")
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,

    /// API format: "completions" or "responses" (default: "completions")
    #[serde(rename = "wireApi", skip_serializing_if = "Option::is_none")]
    pub wire_api: Option<String>,

    /// API endpoint URL
    #[serde(rename = "baseUrl")]
    pub base_url: String,

    /// API key (optional for local providers like Ollama)
    #[serde(rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Bearer token for authentication (takes precedence over API key)
    #[serde(rename = "bearerToken", skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,

    /// Azure-specific options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure: Option<AzureProviderOptions>,
}

/// Azure provider options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureProviderOptions {
    /// Azure API version (default: "2024-10-21")
    #[serde(rename = "apiVersion", skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}

/// Message options for sending a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageOptions {
    /// The message to send
    pub prompt: String,

    /// File or directory attachments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,

    /// Message delivery mode (default: "enqueue")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// File or directory attachment
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Attachment {
    /// Attachment type: "file" or "directory"
    #[serde(rename = "type")]
    pub attachment_type: String,

    /// Path to the file or directory
    pub path: String,

    /// Optional display name
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Handler for session events
pub type SessionEventHandler = Arc<dyn Fn(SessionEvent) + Send + Sync>;

/// Response from a ping request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    pub message: String,
    pub timestamp: i64,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: Option<i32>,
}

/// Response from session.create
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateResponse {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

/// Response from session.send
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSendResponse {
    #[serde(rename = "messageId")]
    pub message_id: String,
}

/// Response from session.getMessages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGetMessagesResponse {
    pub events: Vec<SessionEvent>,
}
