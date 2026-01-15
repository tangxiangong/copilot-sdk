# Copilot CLI SDK for Rust

A Rust SDK for programmatic access to the GitHub Copilot CLI.

> **Note:** This SDK is in technical preview and may change in breaking ways.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
github-copilot = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Features

The SDK supports two transport modes via Cargo features:

- **`stdio`** - Communication via standard input/output (default)
- **`tcp`** - Communication via TCP sockets (default)

By default, both features are enabled. You can customize which transports to include to reduce binary size and compile times:

```toml
# Default: Both stdio and TCP support
[dependencies]
github-copilot = "0.1"

# Only stdio support (saves ~50KB, removes regex dependency)
[dependencies]
github-copilot = { version = "0.1", default-features = false, features = ["stdio"] }

# Only TCP support
[dependencies]
github-copilot = { version = "0.1", default-features = false, features = ["tcp"] }
```

#### Dependency Optimization

The SDK carefully manages dependencies based on enabled features:

| Dependency | Always | stdio | tcp | Purpose       |
| ---------- | ------ | ----- | --- | ------------- |
| `tokio`    | ✓      |       |     | Async runtime |
| `serde`    | ✓      |       |     | Serialization |
| `chrono`   | ✓      |       |     | Timestamps    |
| `uuid`     | ✓      |       |     | Request IDs   |
| `schemars` | ✓      |       |     | Tool schemas  |
| `regex`    |        |       | ✓   | Port parsing  |

## Quick Start

```rust
use github_copilot::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Create client
    let client = Client::new(ClientOptions::new());

    // Start the client
    client.start().await?;

    // Create a session
    let session = client.create_session(Some(SessionConfig {
        model: Some("gpt-4".to_string()),
        ..Default::default()
    })).await?;

    // Set up event handler
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let _unsubscribe = session.on(Arc::new(move |event| {
        if event.is_session_idle() {
            let _ = tx.try_send(());
        }
        if event.is_assistant_message() {
            if let Some(content) = event.get_content() {
                println!("{}", content);
            }
        }
    })).await;

    // Send a message
    session.send(MessageOptions {
        prompt: "What is 2+2?".to_string(),
        attachments: None,
        mode: None,
    }).await?;

    // Wait for completion
    rx.recv().await;

    Ok(())
}
```

## Examples

The SDK includes several comprehensive examples:

- **[stdio_example.rs](examples/stdio_example.rs)** - Basic usage with stdio transport
- **[tcp_example.rs](examples/tcp_example.rs)** - Basic usage with TCP transport
- **[tools_example.rs](examples/tools_example.rs)** - Custom tool definition and execution
- **[permissions_example.rs](examples/permissions_example.rs)** - Permission request handling
- **[resume_session_example.rs](examples/resume_session_example.rs)** - Session persistence and resumption

Run an example:

```bash
cargo run --example tools_example
```

## API Reference

### Client

- `Client::new(options: ClientOptions) -> Client` - Create a new client
- `client.start() -> Result<()>` - Start the CLI server
- `client.stop() -> Vec<CopilotError>` - Stop the CLI server (returns errors if any)
- `client.force_stop()` - Forcefully stop without graceful cleanup
- `client.create_session(config: Option<SessionConfig>) -> Result<Arc<Session>>` - Create a new session
- `client.resume_session(session_id: impl Into<String>) -> Result<Arc<Session>>` - Resume an existing session
- `client.resume_session_with_options(session_id, config) -> Result<Arc<Session>>` - Resume with additional configuration
- `client.get_state() -> ConnectionState` - Get connection state
- `client.ping(message: Option<String>) -> Result<PingResponse>` - Ping the server

**ClientOptions Builder:**

```rust
let options = ClientOptions::new()
    .cli_path("/path/to/copilot")
    .log_level("debug")
    .auto_start(true);
```

Available options:

- `cli_path(path)` - Path to CLI executable (default: "copilot" or `COPILOT_CLI_PATH` env var)
- `cli_url(url)` - URL of existing CLI server (e.g., `"localhost:8080"`, `"8080"`)
- `cwd(path)` - Working directory for CLI process
- `port(port)` - Server port for TCP mode (default: 0 for random)
- `log_level(level)` - Log level (default: "info")
- `auto_start(enabled)` - Auto-start server on first use (default: true)
- `auto_restart(enabled)` - Auto-restart on crash (default: true)
- `env(vars)` - Environment variables for CLI process

### Session

- `session.send(options: MessageOptions) -> Result<String>` - Send a message
- `session.on(handler: SessionEventHandler) -> UnsubscribeHandle` - Subscribe to events
- `session.abort() -> Result<()>` - Abort the currently processing message
- `session.get_messages() -> Result<Vec<SessionEvent>>` - Get message history
- `session.destroy() -> Result<()>` - Destroy the session

### Tools

Expose your own functionality to Copilot by attaching tools to a session.

#### Using `define_tool` (Recommended)

Use `define_tool` for type-safe tools with automatic JSON schema generation:

```rust
use github_copilot::*;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Deserialize, JsonSchema)]
struct LookupIssueParams {
    id: String,
}

let lookup_issue = define_tool(
    "lookup_issue",
    "Fetch issue details from our tracker",
    |params: LookupIssueParams, _inv| {
        let issue = fetch_issue(&params.id)?;
        Ok(issue.summary)
    }
);

let session = client.create_session(Some(SessionConfig {
    model: Some("gpt-4".to_string()),
    tools: vec![lookup_issue],
    ..Default::default()
})).await?;
```

#### Using Tool struct directly

For more control over the JSON schema:

```rust
use std::sync::Arc;

let lookup_issue = Tool {
    name: "lookup_issue".to_string(),
    description: Some("Fetch issue details from our tracker".to_string()),
    parameters: Some(serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "string",
                "description": "Issue identifier"
            }
        },
        "required": ["id"]
    })),
    handler: Arc::new(|inv: ToolInvocation| {
        let args: serde_json::Value = inv.arguments;
        let id = args["id"].as_str().unwrap();
        let issue = fetch_issue(id)?;
        Ok(ToolResult::success(issue.summary))
    }),
};
```

## Streaming

Enable streaming to receive assistant response chunks as they're generated:

```rust
use github_copilot::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new(ClientOptions::new());
    client.start().await?;

    let session = client.create_session(Some(SessionConfig {
        model: Some("gpt-4".to_string()),
        streaming: true,
        ..Default::default()
    })).await?;

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let _unsubscribe = session.on(Arc::new(move |event| {
        match event.event_type {
            SessionEventType::AssistantMessageDelta => {
                // Streaming message chunk - print incrementally
                if let Some(delta) = event.get_delta_content() {
                    print!("{}", delta);
                }
            }
            SessionEventType::AssistantReasoningDelta => {
                // Streaming reasoning chunk (if model supports reasoning)
                if let Some(delta) = event.get_delta_content() {
                    print!("{}", delta);
                }
            }
            SessionEventType::AssistantMessage => {
                // Final message - complete content
                println!("\n--- Final message ---");
                if let Some(content) = event.get_content() {
                    println!("{}", content);
                }
            }
            SessionEventType::SessionIdle => {
                let _ = tx.try_send(());
            }
            _ => {}
        }
    })).await;

    session.send(MessageOptions {
        prompt: "Tell me a short story".to_string(),
        attachments: None,
        mode: None,
    }).await?;

    rx.recv().await;

    Ok(())
}
```

When `streaming: true`:

- `assistant.message_delta` events are sent with `delta_content` containing incremental text
- `assistant.reasoning_delta` events are sent with `delta_content` for reasoning/chain-of-thought
- Accumulate `delta_content` values to build the full response progressively
- The final `assistant.message` and `assistant.reasoning` events contain the complete content

## Transport Modes

### stdio (Default)

Communicates with CLI via stdin/stdout pipes. Recommended for most use cases.

```rust
let client = Client::new(ClientOptions::new()); // Uses stdio by default
```

### TCP

Communicates with CLI via TCP socket. Useful for distributed scenarios.

```rust
let client = Client::new(
    ClientOptions::new()
        .port(8080)
);
```

### External Server

Connect to an existing CLI server:

```rust
let client = Client::new(
    ClientOptions::new()
        .cli_url("localhost:8080")
);
```

## Environment Variables

- `COPILOT_CLI_PATH` - Path to the Copilot CLI executable

## Examples

See the `examples/` directory for more examples:

```bash
cargo run --example basic
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run integration tests
cargo test --tests

# Run with output
cargo test -- --nocapture
```

### Session Event Types

The session event types in `src/session_events.rs` are manually maintained and should be kept in sync with the JSON schema in `@github/copilot/session-events.schema.json`.

When the schema changes:

1. Review the changes in copilot-agent-runtime
2. Update `src/generated/session_events.rs` manually
3. Ensure consistency with other SDK implementations

## Testing

The SDK includes comprehensive E2E tests:

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run E2E tests (requires test harness)
cargo test --test e2e_session_tests
cargo test --test e2e_tools_tests
cargo test --test e2e_permissions_tests
```

## License

MIT
