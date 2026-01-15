//! Example demonstrating custom tool usage

use anyhow::Result;
use github_copilot::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WeatherParams {
    location: String,
    #[serde(default)]
    unit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CalculatorParams {
    operation: String,
    a: f64,
    b: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create weather tool
    let weather_tool = define_tool(
        "get_weather",
        "Get current weather for a location",
        |params: WeatherParams, _inv| {
            let unit = params.unit.unwrap_or_else(|| "celsius".to_string());
            Ok(ToolResult::success(format!(
                "The weather in {} is 72°{}",
                params.location,
                if unit == "fahrenheit" { "F" } else { "C" }
            )))
        },
    );

    // Create calculator tool
    let calculator_tool = define_tool(
        "calculator",
        "Perform arithmetic operations (add, subtract, multiply, divide)",
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

    let client = Client::new(ClientOptions::new().use_stdio(true));
    client.start().await?;
    println!("✓ Client started");

    let config = SessionConfig {
        model: Some("gpt-4".to_string()),
        tools: vec![weather_tool, calculator_tool],
        ..Default::default()
    };

    let session = client.create_session(Some(config)).await?;
    println!("✓ Session created with custom tools");

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
            SessionEventType::ToolExecutionStart => {
                if let Some(tool_name) = &event.data.tool_name {
                    println!("\n🔧 Executing tool: {}", tool_name);
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

    println!("\nSending message...");
    let _ = session
        .send(MessageOptions {
            prompt: "What's the weather in San Francisco? Also, calculate 15 * 23.".to_string(),
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
