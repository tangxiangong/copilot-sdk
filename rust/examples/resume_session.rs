//! Example demonstrating session resumption

use anyhow::Result;
use github_copilot::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new(ClientOptions::new());
    client.start().await?;
    println!("✓ Client started");

    // Create a session with a specific ID
    let config = SessionConfig {
        session_id: Some("my-persistent-session".to_string()),
        model: Some("gpt-4".to_string()),
        ..Default::default()
    };

    let session = client.create_session(Some(config)).await?;
    println!("✓ Created session with ID: {}", session.session_id);

    // Set up event handler
    let message_received = Arc::new(tokio::sync::Mutex::new(false));
    let message_received_clone = message_received.clone();

    let _unsub = session
        .on(Arc::new(move |event| {
            if event.is_assistant_message()
                && let Some(content) = event.get_content()
            {
                println!("\nAssistant: {}", content);
                let msg_clone = message_received_clone.clone();
                tokio::spawn(async move {
                    *msg_clone.lock().await = true;
                });
            }
        }))
        .await;

    // Send first message
    println!("\nFirst conversation:");
    let _ = session
        .send(MessageOptions {
            prompt: "Remember this number: 42".to_string(),
            attachments: None,
            mode: None,
        })
        .await?;

    // Wait for response
    while !*message_received.lock().await {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Get the session ID before destroying
    let session_id = session.session_id.clone();

    // Destroy the session
    println!("\n✓ Destroying session...");
    session.destroy().await?;

    // Resume the session
    println!("\n✓ Resuming session with ID: {}", session_id);
    let resumed_session = client.resume_session(&session_id).await?;

    *message_received.lock().await = false;
    let message_received_clone = message_received.clone();

    let _unsub2 = resumed_session
        .on(Arc::new(move |event| {
            if event.is_assistant_message()
                && let Some(content) = event.get_content()
            {
                println!("\nAssistant (resumed): {}", content);
                let msg_clone = message_received_clone.clone();
                tokio::spawn(async move {
                    *msg_clone.lock().await = true;
                });
            }
        }))
        .await;

    // Send second message
    println!("\nResumed conversation:");
    let _ = resumed_session
        .send(MessageOptions {
            prompt: "What number did I ask you to remember?".to_string(),
            attachments: None,
            mode: None,
        })
        .await?;

    // Wait for response
    while !*message_received.lock().await {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("\n✓ Session context preserved!");

    resumed_session.destroy().await?;
    let errors = client.stop().await;
    if !errors.is_empty() {
        eprintln!("Client stopped with errors: {:?}", errors);
    }

    Ok(())
}
