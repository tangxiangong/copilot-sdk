//! CAPI Proxy for E2E testing
//! Manages a child process that acts as a replaying proxy to AI endpoints.

use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use reqwest::blocking::Client;

/// CapiProxy manages a child process that acts as a replaying proxy to AI endpoints
pub struct CapiProxy {
    cmd: Arc<Mutex<Option<Child>>>,
    proxy_url: Arc<Mutex<String>>,
}

impl CapiProxy {
    /// Create a new proxy instance
    pub fn new() -> Self {
        Self {
            cmd: Arc::new(Mutex::new(None)),
            proxy_url: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Start launches the proxy server and returns its URL
    pub fn start(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut proxy_url = self.proxy_url.lock().unwrap();
        if !proxy_url.is_empty() {
            return Ok(proxy_url.clone());
        }

        // The harness server is in the shared test directory
        let server_path = "../../test/harness/server.ts";

        let mut cmd = Command::new("npx")
            .args(&["tsx", server_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdout = cmd.stdout.take().expect("Failed to capture stdout");
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        // Read the first line to get the listening URL
        let line = lines.next()
            .ok_or("No output from proxy")??;

        // Parse "Listening: http://..." from output
        let url = line
            .strip_prefix("Listening: ")
            .ok_or("Unexpected proxy output")?
            .trim()
            .to_string();

        *proxy_url = url.clone();
        *self.cmd.lock().unwrap() = Some(cmd);

        Ok(url)
    }

    /// Stop gracefully shuts down the proxy server
    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = self.proxy_url.lock().unwrap().clone();

        if !url.is_empty() {
            // Best effort - ignore errors
            let _ = Client::new().post(format!("{}/stop", url)).send();
        }

        if let Some(mut child) = self.cmd.lock().unwrap().take() {
            let _ = child.wait();
        }

        *self.proxy_url.lock().unwrap() = String::new();

        Ok(())
    }

    /// Configure sends configuration to the proxy
    pub fn configure(&self, file_path: &str, work_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = self.proxy_url.lock().unwrap().clone();

        if url.is_empty() {
            return Err("Proxy not started".into());
        }

        #[derive(Serialize)]
        struct Config<'a> {
            #[serde(rename = "filePath")]
            file_path: &'a str,
            #[serde(rename = "workDir")]
            work_dir: &'a str,
        }

        let config = Config { file_path, work_dir };
        let response = Client::new()
            .post(format!("{}/config", url))
            .json(&config)
            .send()?;

        if !response.status().is_success() {
            return Err(format!("Proxy config failed with status {}", response.status()).into());
        }

        Ok(())
    }

    /// GetExchanges retrieves the captured HTTP exchanges from the proxy
    pub fn get_exchanges(&self) -> Result<Vec<ParsedHttpExchange>, Box<dyn std::error::Error>> {
        let url = self.proxy_url.lock().unwrap().clone();

        if url.is_empty() {
            return Err("Proxy not started".into());
        }

        let exchanges = Client::new()
            .get(format!("{}/exchanges", url))
            .send()?
            .json()?;

        Ok(exchanges)
    }

    /// URL returns the proxy URL, or empty if not started
    pub fn url(&self) -> String {
        self.proxy_url.lock().unwrap().clone()
    }
}

impl Drop for CapiProxy {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// ParsedHttpExchange represents a captured HTTP exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedHttpExchange {
    pub request: ChatCompletionRequest,
    pub response: Option<ChatCompletionResponse>,
}

/// ChatCompletionRequest represents an OpenAI chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionTool>>,
}

/// ChatCompletionMessage represents a message in the chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "tool_call_id")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "tool_calls")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// ToolCall represents a tool call in an assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionCall,
}

/// FunctionCall represents the function details in a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// ChatCompletionTool represents a tool in the chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionTool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ChatCompletionToolFunction,
}

/// ChatCompletionToolFunction represents a function tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionToolFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// ChatCompletionResponse represents an OpenAI chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
}

/// ChatCompletionChoice represents a choice in the response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ChatCompletionChoice {
    pub index: usize,
    pub message: ChatCompletionMessage,
    pub finish_reason: String,
}
