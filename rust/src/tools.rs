use crate::{
    error::Result,
    types::{Tool, ToolInvocation, ToolResult},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Define a type-safe tool with automatic schema generation
///
/// # Example
///
/// ```no_run
/// use github_copilot::*;
/// use serde::{Deserialize, Serialize};
/// use schemars::JsonSchema;
///
/// #[derive(Deserialize, JsonSchema)]
/// struct WeatherParams {
///     city: String,
///     unit: String,
/// }
///
/// let tool = define_tool(
///     "get_weather",
///     "Get weather for a city",
///     |params: WeatherParams, _inv| {
///         Ok(format!("Weather in {}: 22°{}", params.city, params.unit))
///     }
/// );
/// ```
pub fn define_tool<P, R, F>(
    name: impl Into<String>,
    description: impl Into<String>,
    handler: F,
) -> Tool
where
    P: for<'de> Deserialize<'de> + JsonSchema,
    R: ToolResultLike,
    F: Fn(P, ToolInvocation) -> Result<R> + Send + Sync + 'static,
{
    let name = name.into();
    let description = description.into();

    // Generate JSON schema for parameters
    let schema = schemars::schema_for!(P);
    let schema_value = serde_json::to_value(schema).ok();

    // Create the handler wrapper
    let handler_arc = Arc::new(move |inv: ToolInvocation| -> Result<ToolResult> {
        // Deserialize arguments into typed parameters
        let params: P = serde_json::from_value(inv.arguments.clone()).map_err(|e| {
            crate::error::CopilotError::InvalidResponse(format!(
                "Failed to deserialize tool arguments: {}",
                e
            ))
        })?;

        // Call the user's handler
        let result = handler(params, inv)?;

        // Normalize the result
        result.into_tool_result()
    });

    Tool {
        name,
        description: Some(description),
        parameters: schema_value,
        handler: handler_arc,
    }
}

/// Trait for types that can be converted to ToolResult
///
/// This trait is implemented for:
/// - `ToolResult` - passes through directly
/// - `String` and `&str` - converted to success results
pub trait ToolResultLike: Sized {
    fn into_tool_result(self) -> Result<ToolResult>;
}

impl ToolResultLike for ToolResult {
    fn into_tool_result(self) -> Result<ToolResult> {
        Ok(self)
    }
}

impl ToolResultLike for String {
    fn into_tool_result(self) -> Result<ToolResult> {
        Ok(ToolResult::success(self))
    }
}

impl ToolResultLike for &str {
    fn into_tool_result(self) -> Result<ToolResult> {
        Ok(ToolResult::success(self.to_string()))
    }
}

/// Helper to wrap serializable values as ToolResult
pub fn result_from_json<T: Serialize>(value: T) -> Result<ToolResult> {
    let json = serde_json::to_string(&value)?;
    Ok(ToolResult::success(json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, JsonSchema)]
    struct TestParams {
        message: String,
    }

    #[test]
    fn test_define_tool_with_string_result() {
        let tool = define_tool("test_tool", "A test tool", |params: TestParams, _inv| {
            Ok(format!("You said: {}", params.message))
        });

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, Some("A test tool".to_string()));
        assert!(tool.parameters.is_some());
    }

    #[test]
    fn test_define_tool_with_tool_result() {
        let tool = define_tool("test_tool", "A test tool", |params: TestParams, _inv| {
            Ok(ToolResult {
                text_result_for_llm: format!("You said: {}", params.message),
                binary_results_for_llm: None,
                result_type: "success".to_string(),
                error: None,
                session_log: Some("Tool executed".to_string()),
                tool_telemetry: None,
            })
        });

        assert_eq!(tool.name, "test_tool");
    }
}
