use crate::error::{CopilotError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    sync::{RwLock, mpsc, oneshot},
};

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: Value,
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// JSON-RPC notification
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: Value,
}

/// Handler for incoming notifications
pub type NotificationHandler = Arc<dyn Fn(String, Value) + Send + Sync>;

/// Handler for incoming requests from server
pub type RequestHandler = Arc<dyn Fn(Value) -> Result<Value> + Send + Sync>;

/// JSON-RPC client
pub struct JsonRpcClient {
    write_tx: mpsc::UnboundedSender<Vec<u8>>,
    pending_requests: Arc<RwLock<HashMap<String, oneshot::Sender<Result<Value>>>>>,
    notification_handler: Arc<RwLock<Option<NotificationHandler>>>,
    request_handlers: Arc<RwLock<HashMap<String, RequestHandler>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl JsonRpcClient {
    /// Create a new JSON-RPC client with the given read/write streams
    pub fn new<R, W>(reader: R, writer: W) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (write_tx, write_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let pending_requests = Arc::new(RwLock::new(HashMap::new()));
        let notification_handler = Arc::new(RwLock::new(None));
        let request_handlers = Arc::new(RwLock::new(HashMap::new()));

        // Spawn read loop
        let pending_requests_clone = pending_requests.clone();
        let notification_handler_clone = notification_handler.clone();
        let request_handlers_clone = request_handlers.clone();
        let write_tx_clone = write_tx.clone();

        tokio::spawn(async move {
            Self::read_loop(
                reader,
                pending_requests_clone,
                notification_handler_clone,
                request_handlers_clone,
                write_tx_clone,
            )
            .await;
        });

        // Spawn write loop
        tokio::spawn(async move {
            Self::write_loop(writer, write_rx, shutdown_rx).await;
        });

        Self {
            write_tx,
            pending_requests,
            notification_handler,
            request_handlers,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Start the client (already started in new())
    pub fn start(&self) {
        // Client starts automatically in new()
    }

    /// Stop the client
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    /// Set the notification handler
    pub async fn set_notification_handler(&self, handler: NotificationHandler) {
        let mut h = self.notification_handler.write().await;
        *h = Some(handler);
    }

    /// Set a request handler for a specific method
    pub async fn set_request_handler(&self, method: String, handler: RequestHandler) {
        let mut handlers = self.request_handlers.write().await;
        handlers.insert(method, handler);
    }

    /// Remove a request handler
    pub async fn remove_request_handler(&self, method: &str) {
        let mut handlers = self.request_handlers.write().await;
        handlers.remove(method);
    }

    /// Send a request and wait for response
    pub async fn request(&self, method: impl Into<String>, params: Value) -> Result<Value> {
        let id = uuid::Uuid::new_v4().to_string();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            method: method.into(),
            params,
        };

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id.clone(), tx);
        }

        // Send request
        self.send_message(&request).await?;

        // Wait for response
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(CopilotError::ChannelClosed),
        }
    }

    /// Send a notification (no response expected)
    pub async fn notify(&self, method: impl Into<String>, params: Value) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        };

        self.send_message(&notification).await
    }

    /// Send a message (internal)
    async fn send_message<T: Serialize>(&self, message: &T) -> Result<()> {
        let json = serde_json::to_vec(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", json.len());

        let mut data = header.into_bytes();
        data.extend_from_slice(&json);

        self.write_tx
            .send(data)
            .map_err(|_| CopilotError::ChannelClosed)?;

        Ok(())
    }

    /// Read loop to handle incoming messages
    async fn read_loop<R>(
        reader: R,
        pending_requests: Arc<RwLock<HashMap<String, oneshot::Sender<Result<Value>>>>>,
        notification_handler: Arc<RwLock<Option<NotificationHandler>>>,
        request_handlers: Arc<RwLock<HashMap<String, RequestHandler>>>,
        write_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) where
        R: AsyncRead + Unpin,
    {
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            // Read Content-Length header
            line.clear();
            let mut content_length = 0;

            loop {
                match reader.read_line(&mut line).await {
                    Ok(0) => return, // EOF
                    Ok(_) => {
                        if line == "\r\n" || line == "\n" {
                            break;
                        }

                        if let Some(len_str) = line.strip_prefix("Content-Length: ")
                            && let Ok(len) = len_str.trim().parse::<usize>()
                        {
                            content_length = len;
                        }

                        line.clear();
                    }
                    Err(_) => return,
                }
            }

            if content_length == 0 {
                continue;
            }

            // Read message body
            let mut body = vec![0u8; content_length];
            if reader.read_exact(&mut body).await.is_err() {
                return;
            }

            // Try to parse as response first
            if let Ok(response) = serde_json::from_slice::<JsonRpcResponse>(&body)
                && let Some(id) = response.id.clone()
            {
                Self::handle_response(response, &id, &pending_requests).await;
                continue;
            }

            // Try to parse as request
            if let Ok(request) = serde_json::from_slice::<JsonRpcRequest>(&body) {
                Self::handle_request(request, &request_handlers, &write_tx).await;
                continue;
            }

            // Try to parse as notification
            if let Ok(notification) = serde_json::from_slice::<JsonRpcNotification>(&body) {
                Self::handle_notification(notification, &notification_handler).await;
            }
        }
    }

    /// Write loop to handle outgoing messages
    async fn write_loop<W>(
        mut writer: W,
        mut write_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) where
        W: AsyncWrite + Unpin,
    {
        loop {
            tokio::select! {
                Some(data) = write_rx.recv() => {
                    if writer.write_all(&data).await.is_err() {
                        return;
                    }
                    if writer.flush().await.is_err() {
                        return;
                    }
                }
                _ = shutdown_rx.recv() => {
                    return;
                }
            }
        }
    }

    /// Handle a response
    async fn handle_response(
        response: JsonRpcResponse,
        id: &str,
        pending_requests: &RwLock<HashMap<String, oneshot::Sender<Result<Value>>>>,
    ) {
        let tx = {
            let mut pending = pending_requests.write().await;
            pending.remove(id)
        };

        if let Some(tx) = tx {
            let result = if let Some(error) = response.error {
                Err(CopilotError::jsonrpc_with_data(
                    error.code,
                    error.message,
                    error.data.unwrap_or(Value::Null),
                ))
            } else if let Some(result) = response.result {
                Ok(result)
            } else {
                Err(CopilotError::InvalidResponse(
                    "Missing result and error".to_string(),
                ))
            };

            let _ = tx.send(result);
        }
    }

    /// Handle a notification
    async fn handle_notification(
        notification: JsonRpcNotification,
        notification_handler: &Arc<RwLock<Option<NotificationHandler>>>,
    ) {
        let handler = notification_handler.read().await;
        if let Some(h) = handler.as_ref() {
            h(notification.method, notification.params);
        }
    }

    /// Handle a request from server
    async fn handle_request(
        request: JsonRpcRequest,
        request_handlers: &Arc<RwLock<HashMap<String, RequestHandler>>>,
        write_tx: &mpsc::UnboundedSender<Vec<u8>>,
    ) {
        let handler = {
            let handlers = request_handlers.read().await;
            handlers.get(&request.method).cloned()
        };

        let response = if let Some(handler) = handler {
            match handler(request.params) {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(request.id),
                    result: Some(result),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(request.id),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                        data: None,
                    }),
                },
            }
        } else {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(request.id),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            }
        };

        // Send response
        if let Ok(json) = serde_json::to_vec(&response) {
            let header = format!("Content-Length: {}\r\n\r\n", json.len());
            let mut data = header.into_bytes();
            data.extend_from_slice(&json);
            let _ = write_tx.send(data);
        }
    }
}
