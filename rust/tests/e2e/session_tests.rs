//! E2E tests for session functionality

mod harness;

use github_copilot::*;
use harness::TestContext;
use std::sync::{Arc, Mutex};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_session_send_and_receive() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("session_send_and_receive").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    let session = client
        .create_session(Some(SessionConfig {
            model: Some("gpt-4".to_string()),
            ..Default::default()
        }))
        .await
        .expect("Failed to create session");

    // Collect events
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();

    let _unsub = session
        .on(Arc::new(move |event| {
            events_clone.lock().unwrap().push(event);
        }))
        .await;

    // Send a message
    let message_id = session
        .send(MessageOptions {
            prompt: "What is 2+2?".to_string(),
            attachments: None,
            mode: None,
        })
        .await
        .expect("Failed to send message");

    assert!(!message_id.is_empty(), "Message ID should not be empty");

    // Wait for events
    tokio::time::sleep(Duration::from_secs(2)).await;

    let collected_events = events.lock().unwrap();
    assert!(
        collected_events.len() > 0,
        "Should have received some events"
    );

    // Check for assistant message
    let has_assistant_message = collected_events
        .iter()
        .any(|e| e.is_assistant_message());

    assert!(has_assistant_message, "Should have received assistant message");

    let _ = session.destroy().await;
    let _ = client.stop().await;
}

#[tokio::test]
async fn test_session_get_messages() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("session_get_messages").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    let session = client
        .create_session(Some(SessionConfig {
            model: Some("gpt-4".to_string()),
            ..Default::default()
        }))
        .await
        .expect("Failed to create session");

    // Send a message
    let _ = session
        .send(MessageOptions {
            prompt: "Hello!".to_string(),
            attachments: None,
            mode: None,
        })
        .await
        .expect("Failed to send message");

    // Wait a bit
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Get messages
    let messages = session
        .get_messages()
        .await
        .expect("Failed to get messages");

    assert!(messages.len() > 0, "Should have some messages");

    let _ = session.destroy().await;
    let _ = client.stop().await;
}

#[tokio::test]
async fn test_session_abort() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("session_abort").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    let session = client
        .create_session(Some(SessionConfig {
            model: Some("gpt-4".to_string()),
            ..Default::default()
        }))
        .await
        .expect("Failed to create session");

    // Send a long-running message
    let session_clone = session.clone();
    tokio::spawn(async move {
        let _ = session_clone
            .send(MessageOptions {
                prompt: "Write a very long story...".to_string(),
                attachments: None,
                mode: None,
            })
            .await;
    });

    // Wait a bit and abort
    tokio::time::sleep(Duration::from_millis(500)).await;

    session.abort().await.expect("Failed to abort");

    let _ = session.destroy().await;
    let _ = client.stop().await;
}

#[tokio::test]
async fn test_session_destroy() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("session_destroy").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    let session = client
        .create_session(None)
        .await
        .expect("Failed to create session");

    let session_id = session.session_id.clone();
    assert!(!session_id.is_empty(), "Session ID should not be empty");

    session.destroy().await.expect("Failed to destroy session");

    let _ = client.stop().await;
}

#[tokio::test]
async fn test_resume_session() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("resume_session").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    // Create initial session
    let session = client
        .create_session(Some(SessionConfig {
            session_id: Some("test-session-123".to_string()),
            model: Some("gpt-4".to_string()),
            ..Default::default()
        }))
        .await
        .expect("Failed to create session");

    let session_id = session.session_id.clone();

    // Destroy it
    session.destroy().await.expect("Failed to destroy session");

    // Resume it
    let resumed_session = client
        .resume_session(&session_id)
        .await
        .expect("Failed to resume session");

    assert_eq!(
        resumed_session.session_id, session_id,
        "Resumed session should have same ID"
    );

    let _ = resumed_session.destroy().await;
    let _ = client.stop().await;
}
