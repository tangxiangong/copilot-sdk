//! E2E tests for tool functionality

mod harness;

use github_copilot::*;
use harness::TestContext;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::time::Duration;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CalculatorParams {
    operation: String,
    a: f64,
    b: f64,
}

#[tokio::test]
async fn test_custom_tool_execution() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("tools_custom_tool").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    // Define a calculator tool
    let calculator = define_tool(
        "calculator",
        "Perform basic arithmetic operations",
        |params: CalculatorParams, _inv| {
            let result = match params.operation.as_str() {
                "add" => params.a + params.b,
                "subtract" => params.a - params.b,
                "multiply" => params.a * params.b,
                "divide" => {
                    if params.b != 0.0 {
                        params.a / params.b
                    } else {
                        return Ok(ToolResult::failure("Division by zero"));
                    }
                }
                _ => return Ok(ToolResult::failure("Unknown operation")),
            };
            Ok(ToolResult::success(format!("Result: {}", result)))
        },
    );

    let config = SessionConfig {
        model: Some("gpt-4".to_string()),
        tools: vec![calculator],
        ..Default::default()
    };

    let session = client
        .create_session(Some(config))
        .await
        .expect("Failed to create session");

    let tool_called = Arc::new(Mutex::new(false));
    let tool_called_clone = tool_called.clone();

    let _unsub = session
        .on(Arc::new(move |event| {
            if matches!(event.event_type, SessionEventType::ToolExecutionStart) {
                *tool_called_clone.lock().unwrap() = true;
            }
        }))
        .await;

    // Send a message that should trigger the tool
    let _ = session
        .send(MessageOptions {
            prompt: "Calculate 15 + 27".to_string(),
            attachments: None,
            mode: None,
        })
        .await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check if tool was called
    assert!(
        *tool_called.lock().unwrap(),
        "Calculator tool should have been called"
    );

    let _ = session.destroy().await;
    let _ = client.stop().await;
}

#[tokio::test]
async fn test_tool_with_json_schema() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("tools_json_schema").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct SearchParams {
        query: String,
        #[serde(default)]
        max_results: Option<u32>,
    }

    let search_tool = define_tool(
        "search",
        "Search for information",
        |params: SearchParams, _inv| {
            Ok(ToolResult::success(format!(
                "Found {} results for '{}'",
                params.max_results.unwrap_or(10),
                params.query
            )))
        },
    );

    let config = SessionConfig {
        model: Some("gpt-4".to_string()),
        tools: vec![search_tool],
        ..Default::default()
    };

    let session = client
        .create_session(Some(config))
        .await
        .expect("Failed to create session");

    assert!(!session.session_id.is_empty());

    let _ = session.destroy().await;
    let _ = client.stop().await;
}

#[tokio::test]
async fn test_multiple_tools() {
    let ctx = TestContext::new().expect("Failed to create test context");
    ctx.configure_for_test("tools_multiple").ok();

    let client = ctx.new_client();
    client.start().await.expect("Failed to start client");

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct EmptyParams {}

    let tool1 = define_tool("get_time", "Get current time", |_: EmptyParams, _inv| {
        Ok(ToolResult::success("12:00 PM"))
    });

    let tool2 = define_tool("get_weather", "Get weather", |_: EmptyParams, _inv| {
        Ok(ToolResult::success("Sunny, 72°F"))
    });

    let config = SessionConfig {
        model: Some("gpt-4".to_string()),
        tools: vec![tool1, tool2],
        ..Default::default()
    };

    let session = client
        .create_session(Some(config))
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
            prompt: "What time is it and what's the weather?".to_string(),
            attachments: None,
            mode: None,
        })
        .await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let collected_events = events.lock().unwrap();
    assert!(
        collected_events.len() > 0,
        "Should have received events"
    );

    let _ = session.destroy().await;
    let _ = client.stop().await;
}
