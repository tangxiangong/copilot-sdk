// Session event types for GitHub Copilot CLI
//
// Ported from: @github/copilot/session-events.schema.json
// Manually maintained to match the schema
//
// To update these types:
// 1. Check the schema changes in copilot-agent-runtime
// 2. Update this file manually to reflect those changes
// 3. Keep in sync with other SDK implementations (Go, Python, Node.js, .NET)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Session event type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventType {
    Abort,
    #[serde(rename = "assistant.intent")]
    AssistantIntent,
    #[serde(rename = "assistant.message")]
    AssistantMessage,
    #[serde(rename = "assistant.message_delta")]
    AssistantMessageDelta,
    #[serde(rename = "assistant.reasoning")]
    AssistantReasoning,
    #[serde(rename = "assistant.reasoning_delta")]
    AssistantReasoningDelta,
    #[serde(rename = "assistant.turn_end")]
    AssistantTurnEnd,
    #[serde(rename = "assistant.turn_start")]
    AssistantTurnStart,
    #[serde(rename = "assistant.usage")]
    AssistantUsage,
    #[serde(rename = "custom_agent.completed")]
    CustomAgentCompleted,
    #[serde(rename = "custom_agent.failed")]
    CustomAgentFailed,
    #[serde(rename = "custom_agent.selected")]
    CustomAgentSelected,
    #[serde(rename = "custom_agent.started")]
    CustomAgentStarted,
    #[serde(rename = "hook.end")]
    HookEnd,
    #[serde(rename = "hook.start")]
    HookStart,
    #[serde(rename = "pending_messages.modified")]
    PendingMessagesModified,
    #[serde(rename = "session.error")]
    SessionError,
    #[serde(rename = "session.handoff")]
    SessionHandoff,
    #[serde(rename = "session.idle")]
    SessionIdle,
    #[serde(rename = "session.info")]
    SessionInfo,
    #[serde(rename = "session.model_change")]
    SessionModelChange,
    #[serde(rename = "session.resume")]
    SessionResume,
    #[serde(rename = "session.start")]
    SessionStart,
    #[serde(rename = "session.truncation")]
    SessionTruncation,
    #[serde(rename = "system.message")]
    SystemMessage,
    #[serde(rename = "tool.execution_complete")]
    ToolExecutionComplete,
    #[serde(rename = "tool.execution_partial_result")]
    ToolExecutionPartialResult,
    #[serde(rename = "tool.execution_start")]
    ToolExecutionStart,
    #[serde(rename = "tool.user_requested")]
    ToolUserRequested,
    #[serde(rename = "user.message")]
    UserMessage,
}

/// Attachment type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentType {
    Directory,
    File,
}

/// Attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub attachment_type: AttachmentType,
}

/// Role
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Developer,
    System,
}

/// Source type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Local,
    Remote,
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub name: String,
    pub owner: String,
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

/// Tool request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub arguments: serde_json::Value,
    pub name: String,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
}

/// Result content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    pub content: String,
}

/// Quota snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    #[serde(rename = "entitlementRequests")]
    pub entitlement_requests: f64,
    #[serde(rename = "isUnlimitedEntitlement")]
    pub is_unlimited_entitlement: bool,
    pub overage: f64,
    #[serde(rename = "overageAllowedWithExhaustedQuota")]
    pub overage_allowed_with_exhausted_quota: bool,
    #[serde(rename = "remainingPercentage")]
    pub remaining_percentage: f64,
    #[serde(rename = "resetDate", skip_serializing_if = "Option::is_none")]
    pub reset_date: Option<DateTime<Utc>>,
    #[serde(rename = "usageAllowedWithExhaustedQuota")]
    pub usage_allowed_with_exhausted_quota: bool,
    #[serde(rename = "usedRequests")]
    pub used_requests: f64,
}

/// Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(rename = "promptVersion", skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, serde_json::Value>>,
}

/// Session event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    // session.start fields
    #[serde(rename = "copilotVersion", skip_serializing_if = "Option::is_none")]
    pub copilot_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
    #[serde(rename = "selectedModel", skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<String>,
    #[serde(rename = "sessionId", skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(rename = "startTime", skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<f64>,

    // session.resume fields
    #[serde(rename = "eventCount", skip_serializing_if = "Option::is_none")]
    pub event_count: Option<f64>,
    #[serde(rename = "resumeTime", skip_serializing_if = "Option::is_none")]
    pub resume_time: Option<DateTime<Utc>>,

    // session.error fields
    #[serde(rename = "errorType", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,

    // session.info fields
    #[serde(rename = "infoType", skip_serializing_if = "Option::is_none")]
    pub info_type: Option<String>,

    // session.model_change fields
    #[serde(rename = "newModel", skip_serializing_if = "Option::is_none")]
    pub new_model: Option<String>,
    #[serde(rename = "previousModel", skip_serializing_if = "Option::is_none")]
    pub previous_model: Option<String>,

    // session.handoff fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(rename = "handoffTime", skip_serializing_if = "Option::is_none")]
    pub handoff_time: Option<DateTime<Utc>>,
    #[serde(rename = "remoteSessionId", skip_serializing_if = "Option::is_none")]
    pub remote_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Repository>,
    #[serde(rename = "sourceType", skip_serializing_if = "Option::is_none")]
    pub source_type: Option<SourceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    // session.truncation fields
    #[serde(
        rename = "messagesRemovedDuringTruncation",
        skip_serializing_if = "Option::is_none"
    )]
    pub messages_removed_during_truncation: Option<f64>,
    #[serde(rename = "performedBy", skip_serializing_if = "Option::is_none")]
    pub performed_by: Option<String>,
    #[serde(
        rename = "postTruncationMessagesLength",
        skip_serializing_if = "Option::is_none"
    )]
    pub post_truncation_messages_length: Option<f64>,
    #[serde(
        rename = "postTruncationTokensInMessages",
        skip_serializing_if = "Option::is_none"
    )]
    pub post_truncation_tokens_in_messages: Option<f64>,
    #[serde(
        rename = "preTruncationMessagesLength",
        skip_serializing_if = "Option::is_none"
    )]
    pub pre_truncation_messages_length: Option<f64>,
    #[serde(
        rename = "preTruncationTokensInMessages",
        skip_serializing_if = "Option::is_none"
    )]
    pub pre_truncation_tokens_in_messages: Option<f64>,
    #[serde(rename = "tokenLimit", skip_serializing_if = "Option::is_none")]
    pub token_limit: Option<f64>,
    #[serde(
        rename = "tokensRemovedDuringTruncation",
        skip_serializing_if = "Option::is_none"
    )]
    pub tokens_removed_during_truncation: Option<f64>,

    // user.message fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(rename = "transformedContent", skip_serializing_if = "Option::is_none")]
    pub transformed_content: Option<String>,
    #[serde(rename = "turnId", skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,

    // assistant.intent fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,

    // assistant.reasoning fields
    #[serde(rename = "chunkContent", skip_serializing_if = "Option::is_none")]
    pub chunk_content: Option<String>,
    #[serde(rename = "reasoningId", skip_serializing_if = "Option::is_none")]
    pub reasoning_id: Option<String>,

    // assistant.message_delta / reasoning_delta fields
    #[serde(rename = "deltaContent", skip_serializing_if = "Option::is_none")]
    pub delta_content: Option<String>,

    // assistant.message fields
    #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(rename = "parentToolCallId", skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
    #[serde(rename = "toolRequests", skip_serializing_if = "Option::is_none")]
    pub tool_requests: Option<Vec<ToolRequest>>,
    #[serde(
        rename = "totalResponseSizeBytes",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_response_size_bytes: Option<f64>,

    // assistant.usage fields
    #[serde(rename = "apiCallId", skip_serializing_if = "Option::is_none")]
    pub api_call_id: Option<String>,
    #[serde(rename = "cacheReadTokens", skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<f64>,
    #[serde(rename = "cacheWriteTokens", skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initiator: Option<String>,
    #[serde(rename = "inputTokens", skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(rename = "outputTokens", skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<f64>,
    #[serde(rename = "providerCallId", skip_serializing_if = "Option::is_none")]
    pub provider_call_id: Option<String>,
    #[serde(rename = "quotaSnapshots", skip_serializing_if = "Option::is_none")]
    pub quota_snapshots: Option<HashMap<String, QuotaSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    // tool.execution_start fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    #[serde(rename = "toolCallId", skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(rename = "toolName", skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,

    // tool.execution_partial_result fields
    #[serde(rename = "partialOutput", skip_serializing_if = "Option::is_none")]
    pub partial_output: Option<String>,

    // tool.execution_complete fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
    #[serde(rename = "isUserRequested", skip_serializing_if = "Option::is_none")]
    pub is_user_requested: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Result>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(rename = "toolTelemetry", skip_serializing_if = "Option::is_none")]
    pub tool_telemetry: Option<HashMap<String, serde_json::Value>>,

    // custom_agent fields
    #[serde(rename = "agentDescription", skip_serializing_if = "Option::is_none")]
    pub agent_description: Option<String>,
    #[serde(rename = "agentDisplayName", skip_serializing_if = "Option::is_none")]
    pub agent_display_name: Option<String>,
    #[serde(rename = "agentName", skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    // hook fields
    #[serde(rename = "hookInvocationId", skip_serializing_if = "Option::is_none")]
    pub hook_invocation_id: Option<String>,
    #[serde(rename = "hookType", skip_serializing_if = "Option::is_none")]
    pub hook_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,

    // system.message fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
}

/// Session event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub data: Data,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<bool>,
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "type")]
    pub event_type: SessionEventType,
}

impl SessionEvent {
    /// Check if this is an assistant message event
    pub fn is_assistant_message(&self) -> bool {
        self.event_type == SessionEventType::AssistantMessage
    }

    /// Check if this is a session idle event
    pub fn is_session_idle(&self) -> bool {
        self.event_type == SessionEventType::SessionIdle
    }

    /// Check if this is a session error event
    pub fn is_session_error(&self) -> bool {
        self.event_type == SessionEventType::SessionError
    }

    /// Get the content of the event if available
    pub fn get_content(&self) -> Option<&str> {
        self.data.content.as_deref()
    }

    /// Get the delta content if this is a streaming event
    pub fn get_delta_content(&self) -> Option<&str> {
        self.data.delta_content.as_deref()
    }
}
