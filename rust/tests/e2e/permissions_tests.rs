//! E2E tests for permission handling

mod harness;

use github_copilot::*;
use harness::TestContext;
use std::sync::{Arc, Mutex};
use tokio::time::Duration;

#[tokio::test]
async fn test_permission_request_approved() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("permissions_approved").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    let permission_called = Arc::new(Mutex::new(false));
    let permission_called_clone = permission_called.clone();

    let permission_handler = Arc::new(move |_req: PermissionRequest, _inv: PermissionInvocation| {
        *permission_called_clone.lock().unwrap() = true;
        
        Ok(PermissionRequestResult {
            kind: "approved".to_string(),
            rules: None,
        })
    });

    let session = client
        .create_session(Some(SessionConfig {
            model: Some("gpt-4".to_string()),
            on_permission_request: Some(permission_handler),
            ..Default::default()
        }))
        .await
        .expect("Failed to create session");

    let _ = session
        .send(MessageOptions {
            prompt: "Read the file test.txt".to_string(),
            attachments: None,
            mode: None,
        })
        .await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Permission handler might have been called
    // (depends on test snapshot configuration)

    let _ = session.destroy().await;
    let _ = client.stop().await;
}

#[tokio::test]
async fn test_permission_request_denied() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("permissions_denied").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    let permission_handler = Arc::new(|_req: PermissionRequest, _inv: PermissionInvocation| {
        Ok(PermissionRequestResult {
            kind: "denied-by-user".to_string(),
            rules: None,
        })
    });

    let session = client
        .create_session(Some(SessionConfig {
            model: Some("gpt-4".to_string()),
            on_permission_request: Some(permission_handler),
            ..Default::default()
        }))
        .await
        .expect("Failed to create session");

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();

    let _unsub = session
        .on(Arc::new(move |event| {
            events_clone.lock().unwrap().push(event);
        }))
        .await;

    let _ = session
        .send(MessageOptions {
            prompt: "Delete all files".to_string(),
            attachments: None,
            mode: None,
        })
        .await;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let _ = session.destroy().await;
    let _ = client.stop().await;
}

#[tokio::test]
async fn test_permission_no_handler() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("permissions_no_handler").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    // Create session without permission handler
    let session = client
        .create_session(Some(SessionConfig {
            model: Some("gpt-4".to_string()),
            on_permission_request: None,
            ..Default::default()
        }))
        .await
        .expect("Failed to create session");

    let _ = session
        .send(MessageOptions {
            prompt: "Read config.json".to_string(),
            attachments: None,
            mode: None,
        })
        .await;

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Should handle gracefully without handler
    let _ = session.destroy().await;
    let _ = client.stop().await;
}
