use crate::{
    error::{CopilotError, Result},
    jsonrpc::JsonRpcClient,
    session_events::SessionEvent,
    types::{
        MessageOptions, PermissionHandler, PermissionInvocation, PermissionRequest,
        SessionEventHandler, ToolHandler,
    },
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// Session represents a single conversation session with the Copilot CLI
pub struct Session {
    /// Unique session identifier
    pub session_id: String,

    /// JSON-RPC client (shared with Client)
    client: Arc<JsonRpcClient>,

    /// Event handlers
    handlers: Arc<RwLock<Vec<SessionEventHandler>>>,

    /// Tool handlers registry
    tool_handlers: Arc<RwLock<HashMap<String, ToolHandler>>>,

    /// Permission handler
    permission_handler: Arc<RwLock<Option<PermissionHandler>>>,
}

impl Session {
    /// Create a new session (internal use)
    pub(crate) fn new(session_id: String, client: Arc<JsonRpcClient>) -> Self {
        Self {
            session_id,
            client,
            handlers: Arc::new(RwLock::new(Vec::new())),
            tool_handlers: Arc::new(RwLock::new(HashMap::new())),
            permission_handler: Arc::new(RwLock::new(None)),
        }
    }

    /// Send a message to this session
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use github_copilot::*;
    /// # async fn example(session: &Session) -> Result<()> {
    /// let message_id = session.send(MessageOptions {
    ///     prompt: "Hello!".to_string(),
    ///     attachments: None,
    ///     mode: None,
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(&self, options: MessageOptions) -> Result<String> {
        let mut params = json!({
            "sessionId": self.session_id,
            "prompt": options.prompt,
        });

        if let Some(attachments) = options.attachments {
            params["attachments"] = json!(attachments);
        }

        if let Some(mode) = options.mode {
            params["mode"] = json!(mode);
        }

        let result = self.client.request("session.send", params).await?;

        let message_id = result
            .get("messageId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CopilotError::InvalidResponse("Missing messageId".to_string()))?;

        Ok(message_id.to_string())
    }

    /// Subscribe to session events
    ///
    /// Returns a handle that will unsubscribe when dropped
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use github_copilot::*;
    /// # use std::sync::Arc;
    /// # async fn example(session: &Session) {
    /// let _unsubscribe = session.on(Arc::new(|event| {
    ///     if event.is_assistant_message() {
    ///         if let Some(content) = event.get_content() {
    ///             println!("Assistant: {}", content);
    ///         }
    ///     }
    /// })).await;
    /// # }
    /// ```
    pub async fn on(&self, handler: SessionEventHandler) -> UnsubscribeHandle {
        let mut handlers = self.handlers.write().await;
        let index = handlers.len();
        handlers.push(handler);

        UnsubscribeHandle {
            handlers: self.handlers.clone(),
            index,
        }
    }

    /// Get all messages from this session's history
    pub async fn get_messages(&self) -> Result<Vec<SessionEvent>> {
        let params = json!({
            "sessionId": self.session_id,
        });

        let result = self.client.request("session.getMessages", params).await?;

        let events_value = result
            .get("events")
            .ok_or_else(|| CopilotError::InvalidResponse("Missing events".to_string()))?;

        let events: Vec<SessionEvent> = serde_json::from_value(events_value.clone())?;

        Ok(events)
    }

    /// Destroy this session
    pub async fn destroy(&self) -> Result<()> {
        let params = json!({
            "sessionId": self.session_id,
        });

        self.client.request("session.destroy", params).await?;

        // Clear handlers
        {
            let mut handlers = self.handlers.write().await;
            handlers.clear();
        }

        {
            let mut tool_handlers = self.tool_handlers.write().await;
            tool_handlers.clear();
        }

        {
            let mut permission_handler = self.permission_handler.write().await;
            *permission_handler = None;
        }

        Ok(())
    }

    /// Abort the currently processing message
    pub async fn abort(&self) -> Result<()> {
        let params = json!({
            "sessionId": self.session_id,
        });

        self.client.request("session.abort", params).await?;

        Ok(())
    }

    /// Register tool handlers (internal)
    pub(crate) async fn register_tools(&self, tools: &[(String, ToolHandler)]) {
        let mut handlers = self.tool_handlers.write().await;
        for (name, handler) in tools {
            handlers.insert(name.clone(), handler.clone());
        }
    }

    /// Get a tool handler (internal)
    pub(crate) async fn get_tool_handler(&self, name: &str) -> Option<ToolHandler> {
        let handlers = self.tool_handlers.read().await;
        handlers.get(name).cloned()
    }

    /// Register permission handler (internal)
    pub(crate) async fn register_permission_handler(&self, handler: PermissionHandler) {
        let mut h = self.permission_handler.write().await;
        *h = Some(handler);
    }

    /// Get permission handler (internal)
    pub(crate) async fn get_permission_handler(&self) -> Option<PermissionHandler> {
        let h = self.permission_handler.read().await;
        h.clone()
    }

    /// Handle permission request (internal)
    pub(crate) async fn handle_permission_request(&self, request_data: Value) -> Result<Value> {
        let handler = self.get_permission_handler().await;

        if handler.is_none() {
            return Ok(json!({
                "kind": "denied-no-approval-rule-and-could-not-request-from-user"
            }));
        }

        let handler = handler.unwrap();

        // Parse request
        let kind = request_data
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tool_call_id = request_data
            .get("toolCallId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut extra = HashMap::new();
        if let Value::Object(map) = &request_data {
            for (k, v) in map {
                if k != "kind" && k != "toolCallId" {
                    extra.insert(k.clone(), v.clone());
                }
            }
        }

        let request = PermissionRequest {
            kind,
            tool_call_id,
            extra,
        };

        let invocation = PermissionInvocation {
            session_id: self.session_id.clone(),
        };

        match handler(request, invocation) {
            Ok(result) => Ok(serde_json::to_value(result)?),
            Err(_) => Ok(json!({
                "kind": "denied-no-approval-rule-and-could-not-request-from-user"
            })),
        }
    }

    /// Dispatch event to handlers (internal)
    pub(crate) async fn dispatch_event(&self, event: SessionEvent) {
        let handlers = {
            let h = self.handlers.read().await;
            h.clone()
        };

        for handler in handlers {
            // Call handler - catch panics to prevent crashing
            let event_clone = event.clone();
            tokio::task::spawn(async move {
                handler(event_clone);
            });
        }
    }
}

/// Handle to unsubscribe from session events
pub struct UnsubscribeHandle {
    handlers: Arc<RwLock<Vec<SessionEventHandler>>>,
    index: usize,
}

impl Drop for UnsubscribeHandle {
    fn drop(&mut self) {
        // Note: This is a simplified unsubscribe that doesn't actually remove
        // the handler immediately. A production implementation would use a
        // more sophisticated approach with unique IDs.
        let handlers = self.handlers.clone();
        let index = self.index;
        tokio::spawn(async move {
            let mut h = handlers.write().await;
            if index < h.len() {
                h.remove(index);
            }
        });
    }
}
