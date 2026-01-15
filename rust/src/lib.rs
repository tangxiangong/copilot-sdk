//! # GitHub Copilot SDK for Rust
//!
//! A Rust SDK for programmatic access to the GitHub Copilot CLI.
//!
//! > **Note:** This SDK is in technical preview and may change in breaking ways.
//!
//! ## Quick Start
//!
//! ```no_run
//! use github_copilot::*;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create client
//!     let client = Client::new(ClientOptions::new());
//!
//!     // Start the client
//!     client.start().await?;
//!
//!     // Create a session
//!     let session = client.create_session(Some(SessionConfig {
//!         model: Some("gpt-4".to_string()),
//!         ..Default::default()
//!     })).await?;
//!
//!     // Set up event handler
//!     let (tx, mut rx) = tokio::sync::mpsc::channel(1);
//!     let _unsubscribe = session.on(Arc::new(move |event| {
//!         if event.is_session_idle() {
//!             let _ = tx.try_send(());
//!         }
//!         if event.is_assistant_message() {
//!             if let Some(content) = event.get_content() {
//!                 println!("{}", content);
//!             }
//!         }
//!     })).await;
//!
//!     // Send a message
//!     session.send(MessageOptions {
//!         prompt: "What is 2+2?".to_string(),
//!         attachments: None,
//!         mode: None,
//!     }).await?;
//!
//!     // Wait for completion
//!     rx.recv().await;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Type-safe API**: Full Rust type safety with idiomatic patterns
//! - **Async/await**: Built on Tokio for efficient async I/O
//! - **Session management**: Create, resume, and manage conversation sessions
//! - **Custom tools**: Expose your own functionality to Copilot
//! - **Streaming**: Receive response chunks as they're generated
//! - **MCP support**: Configure MCP servers for extended functionality
//! - **Custom agents**: Define custom AI agents with specific capabilities
//!
//! ## Transport Modes
//!
//! ### stdio (Default)
//!
//! Communicates with CLI via stdin/stdout pipes:
//!
//! ```no_run
//! # use github_copilot::*;
//! let client = Client::new(ClientOptions::new());
//! ```
//!
//! ### TCP
//!
//! Communicates with CLI via TCP socket:
//!
//! ```no_run
//! # use github_copilot::*;
//! let client = Client::new(
//!     ClientOptions::new()
//!         .port(8080)
//! );
//! ```
//!
//! ### External Server
//!
//! Connect to an existing CLI server:
//!
//! ```no_run
//! # use github_copilot::*;
//! let client = Client::new(
//!     ClientOptions::new()
//!         .cli_url("localhost:8080")
//! );
//! ```

pub mod client;
pub mod error;
pub mod jsonrpc;
pub mod sdk_protocol_version;
pub mod session;
pub mod session_events;
pub mod tools;
pub mod types;

// Re-export main types
pub use client::Client;
pub use error::{CopilotError, Result};
pub use sdk_protocol_version::{SDK_PROTOCOL_VERSION, get_sdk_protocol_version};
pub use session::{Session, UnsubscribeHandle};
pub use session_events::{SessionEvent, SessionEventType};
pub use tools::{ToolResultLike, define_tool};
pub use types::*;
