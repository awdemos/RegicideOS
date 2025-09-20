//! Unit tests for error handling module

use portcl::error::{PortCLError, Result};
use portcl::utils::error::{handle_error, is_retryable_error, error_severity, ErrorSeverity};
use std::io;

#[test]
fn test_error_creation() {
    let portage_error = PortCLError::Portage("test portage error".to_string());
    let rl_error = PortCLError::RLEngine("test rl error".to_string());
    let action_error = PortCLError::ActionExecution("test action error".to_string());
    let config_error = PortCLError::Configuration("test config error".to_string());
    let system_error = PortCLError::System("test system error".to_string());
    let validation_error = PortCLError::Validation("test validation error".to_string());
    let timeout_error = PortCLError::Timeout("test timeout error".to_string());

    assert!(matches!(portage_error, PortCLError::Portage(_)));
    assert!(matches!(rl_error, PortCLError::RLEngine(_)));
    assert!(matches!(action_error, PortCLError::ActionExecution(_)));
    assert!(matches!(config_error, PortCLError::Configuration(_)));
    assert!(matches!(system_error, PortCLError::System(_)));
    assert!(matches!(validation_error, PortCLError::Validation(_)));
    assert!(matches!(timeout_error, PortCLError::Timeout(_)));
}

#[test]
fn test_error_display() {
    let portage_error = PortCLError::Portage("test portage error".to_string());
    assert_eq!(portage_error.to_string(), "Portage API error: test portage error");

    let rl_error = PortCLError::RLEngine("test rl error".to_string());
    assert_eq!(rl_error.to_string(), "RL engine error: test rl error");

    let action_error = PortCLError::ActionExecution("test action error".to_string());
    assert_eq!(action_error.to_string(), "Action execution error: test action error");

    let config_error = PortCLError::Configuration("test config error".to_string());
    assert_eq!(config_error.to_string(), "Configuration error: test config error");

    let system_error = PortCLError::System("test system error".to_string());
    assert_eq!(system_error.to_string(), "System error: test system error");

    let validation_error = PortCLError::Validation("test validation error".to_string());
    assert_eq!(validation_error.to_string(), "Validation error: test validation error");

    let timeout_error = PortCLError::Timeout("test timeout error".to_string());
    assert_eq!(timeout_error.to_string(), "Timeout error: test timeout error");
}

#[test]
fn test_io_error_conversion() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let portcl_error: PortCLError = io_error.into();

    assert!(matches!(portcl_error, PortCLError::Io(_)));
    assert_eq!(portcl_error.to_string(), "IO error: file not found");
}

#[test]
fn test_json_error_conversion() {
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let portcl_error: PortCLError = json_error.into();

    assert!(matches!(portcl_error, PortCLError::Json(_)));
}

#[test]
fn test_toml_error_conversion() {
    let toml_error = toml::from_str::<toml::Value>("invalid toml").unwrap_err();
    let portcl_error: PortCLError = toml_error.into();

    assert!(matches!(portcl_error, PortCLError::TomlDeserialize(_)));
}

#[test]
fn test_result_type_alias() {
    let success_result: Result<i32> = Ok(42);
    assert!(success_result.is_ok());
    assert_eq!(success_result.unwrap(), 42);

    let error_result: Result<i32> = Err(PortCLError::Portage("test error".to_string()));
    assert!(error_result.is_err());
}

#[test]
fn test_error_handling() {
    let portage_error = PortCLError::Portage("test portage error".to_string());
    let result = handle_error(&portage_error);
    assert!(result.is_err());
    assert!(matches!(result, Err(PortCLError::Portage(_))));

    let config_error = PortCLError::Configuration("test config error".to_string());
    let result = handle_error(&config_error);
    assert!(result.is_err());
    assert!(matches!(result, Err(PortCLError::Configuration(_))));
}

#[test]
fn test_retryable_errors() {
    // Network errors should be retryable
    let network_error = PortCLError::Network(reqwest::Error::from(
        reqwest::Response::from(
            http::Response::builder()
                .status(500)
                .body("server error".to_string())
                .unwrap(),
        ),
    ));
    assert!(is_retryable_error(&network_error));

    // Timeout errors should be retryable
    let timeout_error = PortCLError::Timeout("request timed out".to_string());
    assert!(is_retryable_error(&timeout_error));

    // IO errors should be retryable
    let io_error = PortCLError::Io(io::Error::new(io::ErrorKind::WouldBlock, "would block"));
    assert!(is_retryable_error(&io_error));

    // Portage errors with timeout keyword should be retryable
    let portage_timeout_error = PortCLError::Portage("connection timeout".to_string());
    assert!(is_retryable_error(&portage_timeout_error));

    // Non-retryable errors
    let validation_error = PortCLError::Validation("invalid input".to_string());
    assert!(!is_retryable_error(&validation_error));

    let action_error = PortCLError::ActionExecution("action failed".to_string());
    assert!(!is_retryable_error(&action_error));
}

#[test]
fn test_error_severity() {
    // Critical severity
    let config_error = PortCLError::Configuration("invalid config".to_string());
    assert_eq!(error_severity(&config_error), ErrorSeverity::Critical);

    // High severity
    let portage_error = PortCLError::Portage("portage failed".to_string());
    assert_eq!(error_severity(&portage_error), ErrorSeverity::High);

    let action_error = PortCLError::ActionExecution("action failed".to_string());
    assert_eq!(error_severity(&action_error), ErrorSeverity::High);

    let system_error = PortCLError::System("system error".to_string());
    assert_eq!(error_severity(&system_error), ErrorSeverity::High);

    // Medium severity
    let rl_error = PortCLError::RLEngine("rl engine error".to_string());
    assert_eq!(error_severity(&rl_error), ErrorSeverity::Medium);

    let validation_error = PortCLError::Validation("validation error".to_string());
    assert_eq!(error_severity(&validation_error), ErrorSeverity::Medium);

    let io_error = PortCLError::Io(io::Error::new(io::ErrorKind::NotFound, "not found"));
    assert_eq!(error_severity(&io_error), ErrorSeverity::Medium);

    // Low severity
    let network_error = PortCLError::Network(reqwest::Error::from(
        reqwest::Response::from(
            http::Response::builder()
                .status(404)
                .body("not found".to_string())
                .unwrap(),
        ),
    ));
    assert_eq!(error_severity(&network_error), ErrorSeverity::Low);

    let timeout_error = PortCLError::Timeout("timeout".to_string());
    assert_eq!(error_severity(&timeout_error), ErrorSeverity::Low);

    let json_error = PortCLError::Json(serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err());
    assert_eq!(error_severity(&json_error), ErrorSeverity::Low);

    let toml_error = PortCLError::TomlDeserialize(toml::from_str::<toml::Value>("invalid toml").unwrap_err());
    assert_eq!(error_severity(&toml_error), ErrorSeverity::Low);
}

#[test]
fn test_error_severity_display() {
    assert_eq!(ErrorSeverity::Critical.as_str(), "critical");
    assert_eq!(ErrorSeverity::High.as_str(), "high");
    assert_eq!(ErrorSeverity::Medium.as_str(), "medium");
    assert_eq!(ErrorSeverity::Low.as_str(), "low");
}

#[test]
fn test_error_debug_format() {
    let portage_error = PortCLError::Portage("test error".to_string());
    let debug_str = format!("{:?}", portage_error);
    assert!(debug_str.contains("Portage"));
    assert!(debug_str.contains("test error"));
}

#[test]
fn test_error_clone() {
    let original_error = PortCLError::Portage("test error".to_string());
    let cloned_error = original_error.clone();

    assert!(matches!(cloned_error, PortCLError::Portage(_)));
    assert_eq!(original_error.to_string(), cloned_error.to_string());
}

#[test]
fn test_error_from_string_conversions() {
    // Test that string conversions work properly
    let portage_error = PortCLError::Portage("test".to_string());
    let portage_string = portage_error.to_string();
    assert!(portage_string.contains("test"));
}