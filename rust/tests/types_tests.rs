use github_copilot::*;
use std::sync::Arc;

#[test]
fn test_tool_result_success() {
    let result = ToolResult::success("Test output");
    assert_eq!(result.text_result_for_llm, "Test output");
    assert_eq!(result.result_type, "success");
    assert!(result.error.is_none());
}

#[test]
fn test_tool_result_failure() {
    let result = ToolResult::failure("Test error");
    assert_eq!(result.result_type, "failure");
    assert_eq!(result.error, Some("Test error".to_string()));
    assert!(result.text_result_for_llm.contains("error"));
}

#[test]
fn test_session_config_default() {
    let config = SessionConfig::default();
    assert!(config.model.is_none());
    assert!(config.tools.is_empty());
    assert!(!config.streaming);
}

#[test]
fn test_message_options() {
    let options = MessageOptions {
        prompt: "Hello".to_string(),
        attachments: Some(vec![Attachment {
            attachment_type: "file".to_string(),
            path: "/path/to/file".to_string(),
            display_name: Some("file.txt".to_string()),
        }]),
        mode: Some("enqueue".to_string()),
    };

    assert_eq!(options.prompt, "Hello");
    assert_eq!(options.attachments.as_ref().unwrap().len(), 1);
}

#[test]
fn test_provider_config() {
    let provider = ProviderConfig {
        provider_type: Some("openai".to_string()),
        wire_api: Some("completions".to_string()),
        base_url: "https://api.openai.com".to_string(),
        api_key: Some("sk-test".to_string()),
        bearer_token: None,
        azure: None,
    };

    assert_eq!(provider.base_url, "https://api.openai.com");
    assert_eq!(provider.api_key, Some("sk-test".to_string()));
}

#[test]
fn test_tool_handler_creation() {
    let handler: ToolHandler = Arc::new(|_inv| Ok(ToolResult::success("test")));

    let invocation = ToolInvocation {
        session_id: "test".to_string(),
        tool_call_id: "call-1".to_string(),
        tool_name: "test_tool".to_string(),
        arguments: serde_json::json!({}),
    };

    let result = handler(invocation).unwrap();
    assert_eq!(result.text_result_for_llm, "test");
}
