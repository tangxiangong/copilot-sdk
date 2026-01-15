/// Errors that can occur in the Copilot SDK
#[derive(thiserror::Error, Debug)]
pub enum CopilotError {
    /// JSON-RPC error
    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc {
        code: i32,
        message: String,
        data: Option<serde_json::Value>,
    },
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    /// Client not connected
    #[error("Client not connected. Call start() first")]
    NotConnected,
    /// Client already connected
    #[error("Client already connected")]
    AlreadyConnected,
    /// Failed to start CLI server
    #[error("Failed to start CLI server: {0}")]
    ServerStartFailed(String),
    /// Failed to connect to server
    #[error("Failed to connect to server: {0}")]
    ConnectionFailed(String),
    /// Protocol version mismatch
    #[error(
        "SDK protocol version mismatch: SDK expects version {expected}, but server reports version {actual}"
    )]
    ProtocolVersionMismatch { expected: i32, actual: i32 },
    /// Server did not report protocol version
    #[error(
        "SDK protocol version mismatch: SDK expects version {expected}, but server does not report a protocol version"
    )]
    NoProtocolVersion { expected: i32 },
    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    /// Invalid response from server
    #[error("Invalid response from server: {0}")]
    InvalidResponse(String),
    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    /// Tool handler failed
    #[error("Tool handler failed: {0}")]
    ToolHandlerFailed(String),
    /// Permission handler failed
    #[error("Permission handler failed: {0}")]
    PermissionHandlerFailed(String),
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    /// Invalid URL format
    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),
    /// Timeout
    #[error("Operation timed out")]
    Timeout,
    /// Client stopped
    #[error("Client stopped")]
    ClientStopped,
    /// Channel closed
    #[error("Channel closed")]
    ChannelClosed,
    /// Other error
    #[error("{0}")]
    Other(String),
}

impl CopilotError {
    /// Create a JSON-RPC error
    pub fn jsonrpc(code: i32, message: impl Into<String>) -> Self {
        Self::JsonRpc {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a JSON-RPC error with data
    pub fn jsonrpc_with_data(
        code: i32,
        message: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self::JsonRpc {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Result type for Copilot SDK operations
pub type Result<T> = std::result::Result<T, CopilotError>;
