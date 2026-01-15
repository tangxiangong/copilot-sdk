// Example: Using stdio transport (requires feature = "stdio")
//
// Run with: cargo run --example stdio_example --features stdio

use github_copilot::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create client with stdio transport
    let client = Client::new(
        ClientOptions::new()
            .use_stdio(true) // Use stdio transport
            .auto_start(true),
    );

    // Start the client
    client.start().await?;

    // Create a session
    let session = client
        .create_session(Some(SessionConfig {
            model: Some("gpt-4".to_string()),
            ..Default::default()
        }))
        .await?;

    // Set up event handler
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let _unsubscribe = session
        .on(Arc::new(move |event| {
            if event.is_session_idle() {
                let _ = tx.try_send(());
            }
            if event.is_assistant_message()
                && let Some(content) = event.get_content()
            {
                println!("Assistant: {}", content);
            }
        }))
        .await;

    // Send a message
    println!("Sending message via stdio...");
    session
        .send(MessageOptions {
            prompt: "What is Rust?".to_string(),
            attachments: None,
            mode: None,
        })
        .await?;

    // Wait for completion
    rx.recv().await;

    // Clean up
    session.destroy().await?;
    let errors = client.stop().await;
    if !errors.is_empty() {
        eprintln!("Errors during cleanup: {:?}", errors);
    }

    Ok(())
}
