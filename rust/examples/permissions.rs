//! Example demonstrating permission handling

use anyhow::Result;
use github_copilot::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new(ClientOptions::new());
    client.start().await?;
    println!("✓ Client started");

    // Create a permission handler
    let permission_handler = Arc::new(|req: PermissionRequest, inv: PermissionInvocation| {
        println!("\n📋 Permission request:");
        println!("  Kind: {}", req.kind);
        println!("  Session: {}", inv.session_id);

        // In a real application, you might show a UI prompt or check against
        // a policy. For this example, we'll approve read operations and deny writes.

        let approved = req.kind.contains("read") || req.kind.contains("search");

        if approved {
            println!("  ✓ APPROVED");
            Ok(PermissionRequestResult {
                kind: "approved".to_string(),
                rules: None,
            })
        } else {
            println!("  ✗ DENIED");
            Ok(PermissionRequestResult {
                kind: "denied-by-user".to_string(),
                rules: None,
            })
        }
    });

    let config = SessionConfig {
        model: Some("gpt-4".to_string()),
        on_permission_request: Some(permission_handler),
        ..Default::default()
    };

    let session = client.create_session(Some(config)).await?;
    println!("✓ Session created with permission handler");

    // Set up event handler
    let done = Arc::new(tokio::sync::Mutex::new(false));
    let done_clone = done.clone();

    let _unsub = session
        .on(Arc::new(move |event| match event.event_type {
            SessionEventType::AssistantMessage => {
                if let Some(content) = event.get_content() {
                    println!("\nAssistant: {}", content);
                }
            }
            SessionEventType::SessionIdle => {
                let done = done_clone.clone();
                tokio::spawn(async move {
                    *done.lock().await = true;
                });
            }
            _ => {}
        }))
        .await;

    println!("\nAsking for file operations...");
    let _ = session
        .send(MessageOptions {
            prompt: "Read the contents of README.md and then delete it".to_string(),
            attachments: None,
            mode: None,
        })
        .await?;

    // Wait for completion
    while !*done.lock().await {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("\n✓ Done");

    session.destroy().await?;
    let errors = client.stop().await;
    if !errors.is_empty() {
        eprintln!("Client stopped with errors: {:?}", errors);
    }

    Ok(())
}
