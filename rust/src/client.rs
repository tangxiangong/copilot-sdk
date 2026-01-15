use crate::{
    error::{CopilotError, Result},
    jsonrpc::{JsonRpcClient, NotificationHandler, RequestHandler},
    sdk_protocol_version::get_sdk_protocol_version,
    session::Session,
    session_events::SessionEvent,
    types::{
        ClientOptions, ConnectionState, CustomAgentConfig, PingResponse, ProviderConfig,
        ResumeSessionConfig, SessionConfig, ToolInvocation, ToolResult,
    },
};
#[cfg(feature = "tcp")]
use regex::Regex;
use serde_json::{Value, json};
use std::{collections::HashMap, env, sync::Arc};
#[cfg(feature = "tcp")]
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpStream,
};
use tokio::{
    process::{Child, Command},
    sync::RwLock,
};

/// Client manages the connection to the Copilot CLI server
pub struct Client {
    options: ClientOptions,
    process: Arc<RwLock<Option<Child>>>,
    client: Arc<RwLock<Option<Arc<JsonRpcClient>>>>,
    actual_port: Arc<RwLock<u16>>,
    actual_host: Arc<RwLock<String>>,
    state: Arc<RwLock<ConnectionState>>,
    sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
    is_external_server: bool,
}

impl Client {
    /// Create a new Copilot CLI client
    ///
    /// # Example
    ///
    /// ```no_run
    /// use github_copilot::*;
    ///
    /// let client = Client::new(ClientOptions::new());
    /// ```
    pub fn new(mut options: ClientOptions) -> Self {
        let is_external_server = options.cli_url.is_some();

        // Check for CLI path environment variable
        if options.cli_path.is_none() {
            if let Ok(path) = env::var("COPILOT_CLI_PATH") {
                options.cli_path = Some(path);
            } else {
                options.cli_path = Some("copilot".to_string());
            }
        }

        // Set default log level
        if options.log_level.is_none() {
            options.log_level = Some("info".to_string());
        }

        Self {
            options,
            process: Arc::new(RwLock::new(None)),
            client: Arc::new(RwLock::new(None)),
            actual_port: Arc::new(RwLock::new(0)),
            actual_host: Arc::new(RwLock::new("localhost".to_string())),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            is_external_server,
        }
    }

    /// Start the CLI server and establish connection
    pub async fn start(&self) -> Result<()> {
        let current_state = *self.state.read().await;
        if current_state == ConnectionState::Connected {
            return Ok(());
        }

        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Connecting;
        }

        // Parse CLI URL if connecting to external server
        if let Some(url) = &self.options.cli_url {
            let (host, port) = parse_cli_url(url)?;
            *self.actual_host.write().await = host;
            *self.actual_port.write().await = port;
        }

        // Start CLI server process if not using external server
        if !self.is_external_server {
            self.start_cli_server().await?;
        }

        // Connect to server
        self.connect_to_server().await?;

        // Verify protocol version
        self.verify_protocol_version().await?;

        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Connected;
        }

        Ok(())
    }

    /// Stop the CLI server and close all sessions
    pub async fn stop(&self) -> Vec<CopilotError> {
        let mut errors = Vec::new();

        // Destroy all sessions
        let sessions = {
            let s = self.sessions.read().await;
            s.values().cloned().collect::<Vec<_>>()
        };

        for session in sessions {
            if let Err(e) = session.destroy().await {
                errors.push(CopilotError::Other(format!(
                    "Failed to destroy session: {}",
                    e
                )));
            }
        }

        {
            let mut s = self.sessions.write().await;
            s.clear();
        }

        // Kill CLI process if we spawned it
        if !self.is_external_server {
            let mut process = self.process.write().await;
            if let Some(mut child) = process.take()
                && let Err(e) = child.kill().await
            {
                errors.push(CopilotError::Other(format!(
                    "Failed to kill CLI process: {}",
                    e
                )));
            }
        }

        // Close JSON-RPC client
        {
            let mut client = self.client.write().await;
            if let Some(_c) = client.take() {
                // Client will be dropped and cleaned up
            }
        }

        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Disconnected;
        }

        if !self.is_external_server {
            let mut port = self.actual_port.write().await;
            *port = 0;
        }

        errors
    }

    /// Force stop without graceful cleanup
    pub async fn force_stop(&self) {
        // Clear sessions immediately
        {
            let mut s = self.sessions.write().await;
            s.clear();
        }

        // Kill process
        if !self.is_external_server {
            let mut process = self.process.write().await;
            if let Some(mut child) = process.take() {
                let _ = child.kill().await;
            }
        }

        // Close client
        {
            let mut client = self.client.write().await;
            *client = None;
        }

        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Disconnected;
        }

        if !self.is_external_server {
            let mut port = self.actual_port.write().await;
            *port = 0;
        }
    }

    /// Get current connection state
    pub async fn get_state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Ping the server
    pub async fn ping(&self, message: Option<String>) -> Result<PingResponse> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or(CopilotError::NotConnected)?;

        let mut params = json!({});
        if let Some(msg) = message {
            params["message"] = json!(msg);
        }

        let result = client.request("ping", params).await?;

        let response = PingResponse {
            message: result
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            timestamp: result
                .get("timestamp")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            protocol_version: result
                .get("protocolVersion")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
        };

        Ok(response)
    }

    /// Create a new session
    pub async fn create_session(&self, config: Option<SessionConfig>) -> Result<Arc<Session>> {
        // Auto-start if needed
        if self.options.auto_start {
            let state = *self.state.read().await;
            if state != ConnectionState::Connected {
                self.start().await?;
            }
        }

        let client = self.client.read().await;
        let client = client.as_ref().ok_or(CopilotError::NotConnected)?.clone();

        let mut params = json!({});

        if let Some(cfg) = &config {
            if let Some(session_id) = &cfg.session_id {
                params["sessionId"] = json!(session_id);
            }
            if let Some(model) = &cfg.model {
                params["model"] = json!(model);
            }
            if !cfg.tools.is_empty() {
                let tool_defs: Vec<Value> = cfg
                    .tools
                    .iter()
                    .map(|tool| {
                        let mut def = json!({
                            "name": tool.name,
                            "description": tool.description,
                        });
                        if let Some(params) = &tool.parameters {
                            def["parameters"] = params.clone();
                        }
                        def
                    })
                    .collect();
                params["tools"] = json!(tool_defs);
            }
            if let Some(system_message) = &cfg.system_message {
                params["systemMessage"] = serde_json::to_value(system_message)?;
            }
            if let Some(available_tools) = &cfg.available_tools {
                params["availableTools"] = json!(available_tools);
            }
            if let Some(excluded_tools) = &cfg.excluded_tools {
                params["excludedTools"] = json!(excluded_tools);
            }
            if cfg.streaming {
                params["streaming"] = json!(true);
            }
            if let Some(provider) = &cfg.provider {
                params["provider"] = build_provider_params(provider);
            }
            if cfg.on_permission_request.is_some() {
                params["requestPermission"] = json!(true);
            }
            if let Some(mcp_servers) = &cfg.mcp_servers {
                params["mcpServers"] = serde_json::to_value(mcp_servers)?;
            }
            if let Some(custom_agents) = &cfg.custom_agents {
                params["customAgents"] = build_custom_agents_params(custom_agents)?;
            }
        }

        let result = client.request("session.create", params).await?;

        let session_id = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CopilotError::InvalidResponse("Missing sessionId".to_string()))?
            .to_string();

        let session = Arc::new(Session::new(session_id.clone(), client));

        // Register tools and permission handler
        if let Some(cfg) = config {
            let tool_handlers: Vec<(String, _)> = cfg
                .tools
                .iter()
                .map(|t| (t.name.clone(), t.handler.clone()))
                .collect();
            session.register_tools(&tool_handlers).await;

            if let Some(handler) = cfg.on_permission_request {
                session.register_permission_handler(handler).await;
            }
        }

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id, session.clone());
        }

        Ok(session)
    }

    /// Resume an existing session
    pub async fn resume_session(&self, session_id: impl Into<String>) -> Result<Arc<Session>> {
        self.resume_session_with_options(session_id, None).await
    }

    /// Resume an existing session with options
    pub async fn resume_session_with_options(
        &self,
        session_id: impl Into<String>,
        config: Option<ResumeSessionConfig>,
    ) -> Result<Arc<Session>> {
        // Auto-start if needed
        if self.options.auto_start {
            let state = *self.state.read().await;
            if state != ConnectionState::Connected {
                self.start().await?;
            }
        }

        let client = self.client.read().await;
        let client = client.as_ref().ok_or(CopilotError::NotConnected)?.clone();

        let session_id = session_id.into();
        let mut params = json!({
            "sessionId": session_id,
        });

        if let Some(cfg) = &config {
            if !cfg.tools.is_empty() {
                let tool_defs: Vec<Value> = cfg
                    .tools
                    .iter()
                    .map(|tool| {
                        let mut def = json!({
                            "name": tool.name,
                            "description": tool.description,
                        });
                        if let Some(params) = &tool.parameters {
                            def["parameters"] = params.clone();
                        }
                        def
                    })
                    .collect();
                params["tools"] = json!(tool_defs);
            }
            if let Some(provider) = &cfg.provider {
                params["provider"] = build_provider_params(provider);
            }
            if cfg.streaming {
                params["streaming"] = json!(true);
            }
            if cfg.on_permission_request.is_some() {
                params["requestPermission"] = json!(true);
            }
            if let Some(mcp_servers) = &cfg.mcp_servers {
                params["mcpServers"] = serde_json::to_value(mcp_servers)?;
            }
            if let Some(custom_agents) = &cfg.custom_agents {
                params["customAgents"] = build_custom_agents_params(custom_agents)?;
            }
        }

        let result = client.request("session.resume", params).await?;

        let resumed_session_id = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CopilotError::InvalidResponse("Missing sessionId".to_string()))?
            .to_string();

        let session = Arc::new(Session::new(resumed_session_id.clone(), client));

        // Register tools and permission handler
        if let Some(cfg) = config {
            let tool_handlers: Vec<(String, _)> = cfg
                .tools
                .iter()
                .map(|t| (t.name.clone(), t.handler.clone()))
                .collect();
            session.register_tools(&tool_handlers).await;

            if let Some(handler) = cfg.on_permission_request {
                session.register_permission_handler(handler).await;
            }
        }

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(resumed_session_id, session.clone());
        }

        Ok(session)
    }

    // Internal methods

    async fn start_cli_server(&self) -> Result<()> {
        let cli_path =
            self.options.cli_path.as_ref().ok_or_else(|| {
                CopilotError::ServerStartFailed("No CLI path specified".to_string())
            })?;

        let log_level = self.options.log_level.as_deref().unwrap_or("info");

        let mut args = vec![
            "--server".to_string(),
            "--log-level".to_string(),
            log_level.to_string(),
        ];

        #[cfg(feature = "stdio")]
        if self.options.use_stdio {
            args.push("--stdio".to_string());
        }

        #[cfg(feature = "tcp")]
        if !self.options.use_stdio
            && let Some(port) = self.options.port
        {
            args.push("--port".to_string());
            args.push(port.to_string());
        }

        // Handle .js files with node
        let (command, args) = if cli_path.ends_with(".js") {
            let mut node_args = vec![cli_path.clone()];
            node_args.extend(args);
            ("node".to_string(), node_args)
        } else {
            (cli_path.clone(), args)
        };

        let mut cmd = Command::new(&command);
        cmd.args(&args);

        if let Some(cwd) = &self.options.cwd {
            cmd.current_dir(cwd);
        }

        if let Some(env) = &self.options.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        #[cfg(all(feature = "stdio", not(feature = "tcp")))]
        {
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = cmd
                .spawn()
                .map_err(|e| CopilotError::ServerStartFailed(e.to_string()))?;

            let stdin = child.stdin.take().ok_or_else(|| {
                CopilotError::ServerStartFailed("Failed to get stdin".to_string())
            })?;
            let stdout = child.stdout.take().ok_or_else(|| {
                CopilotError::ServerStartFailed("Failed to get stdout".to_string())
            })?;

            // Create JSON-RPC client immediately
            let rpc_client = Arc::new(JsonRpcClient::new(stdout, stdin));
            self.setup_notification_handler(&rpc_client).await;

            {
                let mut client = self.client.write().await;
                *client = Some(rpc_client);
            }

            {
                let mut process = self.process.write().await;
                *process = Some(child);
            }

            return Ok(());
        }

        #[cfg(all(feature = "tcp", not(feature = "stdio")))]
        {
            cmd.stdout(std::process::Stdio::piped());

            let mut child = cmd
                .spawn()
                .map_err(|e| CopilotError::ServerStartFailed(e.to_string()))?;

            let stdout = child.stdout.take().ok_or_else(|| {
                CopilotError::ServerStartFailed("Failed to get stdout".to_string())
            })?;

            // Wait for port announcement
            let port = Self::wait_for_port(stdout).await?;

            {
                let mut actual_port = self.actual_port.write().await;
                *actual_port = port;
            }

            {
                let mut process = self.process.write().await;
                *process = Some(child);
            }

            return Ok(());
        }

        #[cfg(all(feature = "stdio", feature = "tcp"))]
        if self.options.use_stdio {
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = cmd
                .spawn()
                .map_err(|e| CopilotError::ServerStartFailed(e.to_string()))?;

            let stdin = child.stdin.take().ok_or_else(|| {
                CopilotError::ServerStartFailed("Failed to get stdin".to_string())
            })?;
            let stdout = child.stdout.take().ok_or_else(|| {
                CopilotError::ServerStartFailed("Failed to get stdout".to_string())
            })?;

            // Create JSON-RPC client immediately
            let rpc_client = Arc::new(JsonRpcClient::new(stdout, stdin));
            self.setup_notification_handler(&rpc_client).await;

            {
                let mut client = self.client.write().await;
                *client = Some(rpc_client);
            }

            {
                let mut process = self.process.write().await;
                *process = Some(child);
            }

            Ok(())
        } else {
            cmd.stdout(std::process::Stdio::piped());

            let mut child = cmd
                .spawn()
                .map_err(|e| CopilotError::ServerStartFailed(e.to_string()))?;

            let stdout = child.stdout.take().ok_or_else(|| {
                CopilotError::ServerStartFailed("Failed to get stdout".to_string())
            })?;

            // Wait for port announcement
            let port = Self::wait_for_port(stdout).await?;

            {
                let mut actual_port = self.actual_port.write().await;
                *actual_port = port;
            }

            {
                let mut process = self.process.write().await;
                *process = Some(child);
            }

            Ok(())
        }

        #[cfg(not(any(feature = "stdio", feature = "tcp")))]
        Err(CopilotError::ServerStartFailed(
            "No transport feature enabled. Enable 'stdio' or 'tcp' feature.".to_string(),
        ))
    }

    #[cfg(feature = "tcp")]
    async fn wait_for_port(stdout: tokio::process::ChildStdout) -> Result<u16> {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let port_regex = Regex::new(r"listening on port (\d+)").unwrap();

        let timeout = tokio::time::Duration::from_secs(10);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            tokio::select! {
                line = lines.next_line() => {
                    if let Ok(Some(line)) = line
                        && let Some(captures) = port_regex.captures(&line)
                            && let Some(port_str) = captures.get(1) && let Ok(port) = port_str.as_str().parse::<u16>() {
                                    return Ok(port);
                            }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
            }
        }

        Err(CopilotError::ServerStartFailed(
            "Timeout waiting for port".to_string(),
        ))
    }

    async fn connect_to_server(&self) -> Result<()> {
        #[cfg(all(feature = "stdio", not(feature = "tcp")))]
        {
            Ok(())
        }

        #[cfg(all(feature = "tcp", not(feature = "stdio")))]
        {
            // Connect via TCP
            let port = *self.actual_port.read().await;
            let host = self.actual_host.read().await.clone();

            if port == 0 {
                return Err(CopilotError::ConnectionFailed(
                    "Server port not available".to_string(),
                ));
            }

            let addr = format!("{}:{}", host, port);
            let stream = TcpStream::connect(&addr).await.map_err(|e| {
                CopilotError::ConnectionFailed(format!("Failed to connect to {}: {}", addr, e))
            })?;

            let (reader, writer) = stream.into_split();

            let rpc_client = Arc::new(JsonRpcClient::new(reader, writer));
            self.setup_notification_handler(&rpc_client).await;

            {
                let mut client = self.client.write().await;
                *client = Some(rpc_client);
            }

            Ok(())
        }

        #[cfg(all(feature = "stdio", feature = "tcp"))]
        if self.options.use_stdio {
            Ok(())
        } else {
            // Connect via TCP
            let port = *self.actual_port.read().await;
            let host = self.actual_host.read().await.clone();

            if port == 0 {
                return Err(CopilotError::ConnectionFailed(
                    "Server port not available".to_string(),
                ));
            }

            let addr = format!("{}:{}", host, port);
            let stream = TcpStream::connect(&addr).await.map_err(|e| {
                CopilotError::ConnectionFailed(format!("Failed to connect to {}: {}", addr, e))
            })?;

            let (reader, writer) = stream.into_split();

            let rpc_client = Arc::new(JsonRpcClient::new(reader, writer));
            self.setup_notification_handler(&rpc_client).await;

            {
                let mut client = self.client.write().await;
                *client = Some(rpc_client);
            }

            Ok(())
        }

        #[cfg(not(any(feature = "stdio", feature = "tcp")))]
        Err(CopilotError::ConnectionFailed(
            "No transport feature enabled. Enable 'stdio' or 'tcp' feature.".to_string(),
        ))
    }

    async fn setup_notification_handler(&self, client: &Arc<JsonRpcClient>) {
        let sessions = self.sessions.clone();

        let handler: NotificationHandler = Arc::new(move |method, params| {
            if method == "session.event" {
                let sessions = sessions.clone();
                tokio::spawn(async move {
                    let session_id = params
                        .get("sessionId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let event_value = params.get("event");

                    if let (Some(session_id), Some(event_value)) = (session_id, event_value)
                        && let Ok(event) =
                            serde_json::from_value::<SessionEvent>(event_value.clone())
                    {
                        let sessions = sessions.read().await;
                        if let Some(session) = sessions.get(&session_id) {
                            session.dispatch_event(event).await;
                        }
                    }
                });
            }
        });

        client.set_notification_handler(handler).await;

        // Set up tool.call handler
        let sessions_clone = self.sessions.clone();
        let tool_call_handler: RequestHandler = Arc::new(move |params| {
            let sessions = sessions_clone.clone();

            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { handle_tool_call(params, &sessions).await })
            })
        });

        client
            .set_request_handler("tool.call".to_string(), tool_call_handler)
            .await;

        // Set up permission.request handler
        let sessions_clone = self.sessions.clone();
        let permission_handler: RequestHandler = Arc::new(move |params| {
            let sessions = sessions_clone.clone();

            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { handle_permission_request(params, &sessions).await })
            })
        });

        client
            .set_request_handler("permission.request".to_string(), permission_handler)
            .await;
    }

    async fn verify_protocol_version(&self) -> Result<()> {
        let expected_version = get_sdk_protocol_version();
        let ping_result = self.ping(None).await?;

        match ping_result.protocol_version {
            None => Err(CopilotError::NoProtocolVersion {
                expected: expected_version,
            }),
            Some(actual) if actual != expected_version => {
                Err(CopilotError::ProtocolVersionMismatch {
                    expected: expected_version,
                    actual,
                })
            }
            _ => Ok(()),
        }
    }
}

fn parse_cli_url(url: &str) -> Result<(String, u16)> {
    let url = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");

    // Check if it's just a port number
    if let Ok(port) = url.parse::<u16>() {
        return Ok(("localhost".to_string(), port));
    }

    // Parse host:port
    let parts: Vec<&str> = url.split(':').collect();
    if parts.len() != 2 {
        return Err(CopilotError::InvalidUrl(format!(
            "Invalid CLI URL format: {}. Expected 'host:port' or 'port'",
            url
        )));
    }

    let host = if parts[0].is_empty() {
        "localhost".to_string()
    } else {
        parts[0].to_string()
    };

    let port = parts[1]
        .parse::<u16>()
        .map_err(|_| CopilotError::InvalidUrl(format!("Invalid port in CLI URL: {}", url)))?;

    Ok((host, port))
}

fn build_provider_params(provider: &ProviderConfig) -> Value {
    let mut params = json!({
        "baseUrl": provider.base_url,
    });

    if let Some(provider_type) = &provider.provider_type {
        params["type"] = json!(provider_type);
    }
    if let Some(wire_api) = &provider.wire_api {
        params["wireApi"] = json!(wire_api);
    }
    if let Some(api_key) = &provider.api_key {
        params["apiKey"] = json!(api_key);
    }
    if let Some(bearer_token) = &provider.bearer_token {
        params["bearerToken"] = json!(bearer_token);
    }
    if let Some(azure) = &provider.azure {
        let mut azure_params = json!({});
        if let Some(api_version) = &azure.api_version {
            azure_params["apiVersion"] = json!(api_version);
        }
        params["azure"] = azure_params;
    }

    params
}

fn build_custom_agents_params(agents: &[CustomAgentConfig]) -> Result<Value> {
    let agents_value: Vec<Value> = agents
        .iter()
        .map(|agent| {
            let mut agent_map = json!({
                "name": agent.name,
                "prompt": agent.prompt,
            });
            if let Some(display_name) = &agent.display_name {
                agent_map["displayName"] = json!(display_name);
            }
            if let Some(description) = &agent.description {
                agent_map["description"] = json!(description);
            }
            if let Some(tools) = &agent.tools {
                agent_map["tools"] = json!(tools);
            }
            if let Some(mcp_servers) = &agent.mcp_servers {
                agent_map["mcpServers"] = serde_json::to_value(mcp_servers).unwrap_or(Value::Null);
            }
            if let Some(infer) = agent.infer {
                agent_map["infer"] = json!(infer);
            }
            agent_map
        })
        .collect();

    Ok(json!(agents_value))
}

async fn handle_tool_call(
    params: Value,
    sessions: &Arc<RwLock<HashMap<String, Arc<Session>>>>,
) -> Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidResponse("Missing sessionId".to_string()))?;

    let tool_call_id = params
        .get("toolCallId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidResponse("Missing toolCallId".to_string()))?;

    let tool_name = params
        .get("toolName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidResponse("Missing toolName".to_string()))?;

    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

    let sessions = sessions.read().await;
    let session = sessions
        .get(session_id)
        .ok_or_else(|| CopilotError::SessionNotFound(session_id.to_string()))?;

    let handler = session
        .get_tool_handler(tool_name)
        .await
        .ok_or_else(|| CopilotError::ToolNotFound(tool_name.to_string()))?;

    let invocation = ToolInvocation {
        session_id: session_id.to_string(),
        tool_call_id: tool_call_id.to_string(),
        tool_name: tool_name.to_string(),
        arguments,
    };

    let result = match handler(invocation) {
        Ok(result) => result,
        Err(e) => ToolResult::failure(e.to_string()),
    };

    Ok(json!({
        "result": result
    }))
}

async fn handle_permission_request(
    params: Value,
    sessions: &Arc<RwLock<HashMap<String, Arc<Session>>>>,
) -> Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidResponse("Missing sessionId".to_string()))?;

    let permission_request = params
        .get("permissionRequest")
        .ok_or_else(|| CopilotError::InvalidResponse("Missing permissionRequest".to_string()))?;

    let sessions = sessions.read().await;
    let session = sessions
        .get(session_id)
        .ok_or_else(|| CopilotError::SessionNotFound(session_id.to_string()))?;

    let result = session
        .handle_permission_request(permission_request.clone())
        .await?;

    Ok(json!({
        "result": result
    }))
}
