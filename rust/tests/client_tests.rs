use github_copilot::*;

#[test]
fn test_client_options_builder() {
    let options = ClientOptions::new()
        .cli_path("/usr/local/bin/copilot")
        .log_level("debug")
        .port(8080)
        .auto_start(false);

    assert_eq!(options.cli_path, Some("/usr/local/bin/copilot".to_string()));
    assert_eq!(options.log_level, Some("debug".to_string()));
    assert_eq!(options.port, Some(8080));
    assert!(!options.use_stdio);
    assert!(!options.auto_start);
}

#[test]
fn test_client_options_cli_url() {
    let options = ClientOptions::new().cli_url("localhost:3000");

    assert_eq!(options.cli_url, Some("localhost:3000".to_string()));
    assert!(!options.use_stdio);
}

#[test]
fn test_connection_states() {
    assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
    assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);
}
