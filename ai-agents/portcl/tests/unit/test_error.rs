use portcl::error::{PortCLError, Result};
use portcl::utils::error::{handle_error, log_result, is_retryable_error, error_severity, ErrorSeverity};
use portcl::prelude::*;

use std::io;
use std::path::PathBuf;
use serde_json;
use toml;

// Test utilities
fn create_test_errors() -> Vec<PortCLError> {
    vec![
        PortCLError::Portage("Failed to execute emerge command".to_string()),
        PortCLError::RLEngine("Model training failed".to_string()),
        PortCLError::ActionExecution("Package installation failed".to_string()),
        PortCLError::Configuration("Invalid configuration file".to_string()),
        PortCLError::Io(io::Error::new(io::ErrorKind::NotFound, "File not found")),
        PortCLError::Json(serde_json::Error::custom("Invalid JSON format")),
        PortCLError::TomlDeserialize(toml::de::Error::custom("Invalid TOML format")),
        PortCLError::TomlSerialize(toml::ser::Error::custom("Failed to serialize TOML")),
        PortCLError::System("Out of memory".to_string()),
        PortCLError::Network(reqwest::Error::from(io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"))),
        PortCLError::Timeout("Operation timed out".to_string()),
        PortCLError::Validation("Invalid parameter value".to_string()),
        PortCLError::NotFound("Resource not found".to_string()),
        PortCLError::Service("Service unavailable".to_string()),
    ]
}

#[tokio::test]
async fn test_portcl_error_display() {
    let test_cases = vec![
        (PortCLError::Portage("Test error".to_string()), "Portage API error: Test error"),
        (PortCLError::RLEngine("RL error".to_string()), "RL engine error: RL error"),
        (PortCLError::ActionExecution("Action failed".to_string()), "Action execution error: Action failed"),
        (PortCLError::Configuration("Config error".to_string()), "Configuration error: Config error"),
        (PortCLError::System("System error".to_string()), "System error: System error"),
        (PortCLError::Network(reqwest::Error::from(io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"))), "Network error: error sending request"),
        (PortCLError::Timeout("Timeout error".to_string()), "Timeout error: Timeout error"),
        (PortCLError::Validation("Validation error".to_string()), "Validation error: Validation error"),
        (PortCLError::NotFound("Not found".to_string()), "Not found: Not found"),
        (PortCLError::Service("Service error".to_string()), "Service error: Service error"),
    ];

    for (error, expected_message) in test_cases {
        let display_message = format!("{}", error);
        assert!(display_message.contains(expected_message), "Error display should contain expected message: {}", expected_message);
    }
}

#[tokio::test]
async fn test_portcl_error_debug() {
    let error = PortCLError::Portage("Test error".to_string());
    let debug_string = format!("{:?}", error);

    assert!(debug_string.contains("PortCLError::Portage"));
    assert!(debug_string.contains("Test error"));
}

#[tokio::test]
async fn test_portcl_error_clone() {
    let original = PortCLError::Portage("Original error".to_string());
    let cloned = original.clone();

    // Both should be equal
    match (&original, &cloned) {
        (PortCLError::Portage(msg1), PortCLError::Portage(msg2)) => {
            assert_eq!(msg1, msg2);
        },
        _ => panic!("Cloned error should be same type and content"),
    }
}

#[tokio::test]
async fn test_portcl_error_from_conversions() {
    // Test IO error conversion
    let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");
    let portcl_error: PortCLError = io_error.into();
    assert!(matches!(portcl_error, PortCLError::Io(_)));

    // Test JSON error conversion
    let json_error = serde_json::Error::custom("JSON error");
    let portcl_error: PortCLError = json_error.into();
    assert!(matches!(portcl_error, PortCLError::Json(_)));

    // Test TOML deserialization error conversion
    let toml_error = toml::de::Error::custom("TOML error");
    let portcl_error: PortCLError = toml_error.into();
    assert!(matches!(portcl_error, PortCLError::TomlDeserialize(_)));

    // Test TOML serialization error conversion
    let toml_error = toml::ser::Error::custom("TOML serialization error");
    let portcl_error: PortCLError = toml_error.into();
    assert!(matches!(portcl_error, PortCLError::TomlSerialize(_)));

    // Test reqwest error conversion
    let reqwest_error = reqwest::Error::from(io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"));
    let portcl_error: PortCLError = reqwest_error.into();
    assert!(matches!(portcl_error, PortCLError::Network(_)));
}

#[tokio::test]
async fn test_result_type_alias() {
    // Test that Result type alias works correctly
    let success_result: Result<i32> = Ok(42);
    assert!(success_result.is_ok());
    assert_eq!(success_result.unwrap(), 42);

    let error_result: Result<i32> = Err(PortCLError::Validation("Invalid value".to_string()));
    assert!(error_result.is_err());
    match error_result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert_eq!(msg, "Invalid value");
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_handle_error_function() {
    let test_errors = create_test_errors();

    for error in test_errors {
        let result = handle_error(&error);

        // Should always return an error
        assert!(result.is_err());

        // The returned error should be of the same type (except for From conversions)
        match (&error, result.unwrap_err()) {
            (PortCLError::Portage(msg1), PortCLError::Portage(msg2)) => assert_eq!(msg1, msg2),
            (PortCLError::RLEngine(msg1), PortCLError::RLEngine(msg2)) => assert_eq!(msg1, msg2),
            (PortCLError::ActionExecution(msg1), PortCLError::ActionExecution(msg2)) => assert_eq!(msg1, msg2),
            (PortCLError::Configuration(msg1), PortCLError::Configuration(msg2)) => assert_eq!(msg1, msg2),
            (PortCLError::System(msg1), PortCLError::System(msg2)) => assert_eq!(msg1, msg2),
            (PortCLError::Timeout(msg1), PortCLError::Timeout(msg2)) => assert_eq!(msg1, msg2),
            (PortCLError::Validation(msg1), PortCLError::Validation(msg2)) => assert_eq!(msg1, msg2),
            (PortCLError::NotFound(msg1), PortCLError::NotFound(msg2)) => assert_eq!(msg1, msg2),
            (PortCLError::Service(msg1), PortCLError::Service(msg2)) => assert_eq!(msg1, msg2),
            // From conversions may have different types
            (PortCLError::Io(_), PortCLError::Io(_)) => (),
            (PortCLError::Json(_), PortCLError::Json(_)) => (),
            (PortCLError::TomlDeserialize(_), PortCLError::TomlDeserialize(_)) => (),
            (PortCLError::TomlSerialize(_), PortCLError::TomlSerialize(_)) => (),
            (PortCLError::Network(_), PortCLError::Network(_)) => (),
            _ => panic!("Unexpected error type conversion"),
        }
    }
}

#[tokio::test]
async fn test_log_result_function_success() {
    let success_result: Result<i32> = Ok(42);
    let operation = "test operation";

    let result = log_result(success_result, operation);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_log_result_function_error() {
    let error_result: Result<i32> = Err(PortCLError::Validation("Invalid value".to_string()));
    let operation = "test operation";

    let result = log_result(error_result, operation);

    assert!(result.is_err());
    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert_eq!(msg, "Invalid value");
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_is_retryable_error_function() {
    let retryable_errors = vec![
        PortCLError::Network(reqwest::Error::from(io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"))),
        PortCLError::Timeout("Operation timed out".to_string()),
        PortCLError::Io(io::Error::new(io::ErrorKind::ConnectionReset, "Connection reset")),
        PortCLError::Portage("Portage timeout occurred".to_string()),
        PortCLError::Portage("Network issue with Portage".to_string()),
        PortCLError::Portage("Temporary Portage failure".to_string()),
        PortCLError::System("Resource temporarily unavailable".to_string()),
        PortCLError::System("System busy, please try again".to_string()),
    ];

    let non_retryable_errors = vec![
        PortCLError::RLEngine("Model training failed".to_string()),
        PortCLError::ActionExecution("Package installation failed".to_string()),
        PortCLError::Configuration("Invalid configuration file".to_string()),
        PortCLError::Validation("Invalid parameter value".to_string()),
        PortCLError::NotFound("Resource not found".to_string()),
        PortCLError::Service("Service unavailable".to_string()),
        PortCLError::Json(serde_json::Error::custom("Invalid JSON format")),
        PortCLError::TomlDeserialize(toml::de::Error::custom("Invalid TOML format")),
        PortCLError::Portage("Package not found".to_string()), // Not retryable
        PortCLError::System("Out of memory".to_string()), // Not retryable
    ];

    for error in retryable_errors {
        assert!(is_retryable_error(&error), "Error should be retryable: {:?}", error);
    }

    for error in non_retryable_errors {
        assert!(!is_retryable_error(&error), "Error should not be retryable: {:?}", error);
    }
}

#[tokio::test]
async fn test_error_severity_function() {
    let test_cases = vec![
        (PortCLError::Portage("Portage error".to_string()), ErrorSeverity::High),
        (PortCLError::RLEngine("RL engine error".to_string()), ErrorSeverity::Medium),
        (PortCLError::ActionExecution("Action error".to_string()), ErrorSeverity::High),
        (PortCLError::Configuration("Config error".to_string()), ErrorSeverity::Critical),
        (PortCLError::System("System error".to_string()), ErrorSeverity::High),
        (PortCLError::Network(reqwest::Error::from(io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"))), ErrorSeverity::Low),
        (PortCLError::Timeout("Timeout error".to_string()), ErrorSeverity::Low),
        (PortCLError::Validation("Validation error".to_string()), ErrorSeverity::Medium),
        (PortCLError::Io(io::Error::new(io::ErrorKind::NotFound, "File not found")), ErrorSeverity::Medium),
        (PortCLError::Json(serde_json::Error::custom("JSON error")), ErrorSeverity::Low),
        (PortCLError::TomlDeserialize(toml::de::Error::custom("TOML error")), ErrorSeverity::Low),
        (PortCLError::TomlSerialize(toml::ser::Error::custom("TOML serialization error")), ErrorSeverity::Low),
        (PortCLError::NotFound("Not found".to_string()), ErrorSeverity::Medium),
        (PortCLError::Service("Service error".to_string()), ErrorSeverity::High),
    ];

    for (error, expected_severity) in test_cases {
        let severity = error_severity(&error);
        assert_eq!(severity, expected_severity, "Error severity mismatch for {:?}: expected {:?}, got {:?}", error, expected_severity, severity);
    }
}

#[tokio::test]
async fn test_error_severity_as_str() {
    assert_eq!(ErrorSeverity::Critical.as_str(), "critical");
    assert_eq!(ErrorSeverity::High.as_str(), "high");
    assert_eq!(ErrorSeverity::Medium.as_str(), "medium");
    assert_eq!(ErrorSeverity::Low.as_str(), "low");
}

#[tokio::test]
async fn test_error_severity_debug() {
    let severities = vec![
        ErrorSeverity::Critical,
        ErrorSeverity::High,
        ErrorSeverity::Medium,
        ErrorSeverity::Low,
    ];

    for severity in severities {
        let debug_string = format!("{:?}", severity);
        assert!(!debug_string.is_empty());
    }
}

#[tokio::test]
async fn test_error_severity_clone() {
    let original = ErrorSeverity::High;
    let cloned = original.clone();

    assert_eq!(original, cloned);
}

#[tokio::test]
async fn test_error_severity_partial_eq() {
    assert_eq!(ErrorSeverity::Critical, ErrorSeverity::Critical);
    assert_eq!(ErrorSeverity::High, ErrorSeverity::High);
    assert_eq!(ErrorSeverity::Medium, ErrorSeverity::Medium);
    assert_eq!(ErrorSeverity::Low, ErrorSeverity::Low);

    assert_ne!(ErrorSeverity::Critical, ErrorSeverity::High);
    assert_ne!(ErrorSeverity::High, ErrorSeverity::Medium);
    assert_ne!(ErrorSeverity::Medium, ErrorSeverity::Low);
}

#[tokio::test]
async fn test_error_chaining() {
    // Test error chaining where one error causes another
    let io_error = io::Error::new(io::ErrorKind::NotFound, "Config file not found");
    let config_error = PortCLError::Configuration(format!("Failed to load config: {}", io_error));

    // The config error should wrap the IO error context
    assert!(matches!(config_error, PortCLError::Configuration(_)));
    let error_string = format!("{}", config_error);
    assert!(error_string.contains("Failed to load config"));
    assert!(error_string.contains("Config file not found"));
}

#[tokio::test]
async fn test_error_context_propagation() {
    fn function_that_fails() -> Result<()> {
        Err(PortCLError::Validation("Invalid input".to_string()))
    }

    fn function_that_calls_other() -> Result<()> {
        function_that_fails().map_err(|e| PortCLError::ActionExecution(format!("Action failed: {}", e)))
    }

    let result = function_that_calls_other();
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::ActionExecution(msg) => {
            assert!(msg.contains("Action failed"));
            assert!(msg.contains("Invalid input"));
        },
        _ => panic!("Expected ActionExecution error with context"),
    }
}

#[tokio::test]
async fn test_error_recovery_scenarios() {
    // Test scenarios where errors can be recovered from
    fn attempt_operation(attempt: u32) -> Result<String> {
        if attempt < 3 {
            Err(PortCLError::Network(format!("Network timeout on attempt {}", attempt)))
        } else {
            Ok("Operation succeeded".to_string())
        }
    }

    // Simulate retry logic
    let mut attempts = 0;
    let max_attempts = 5;
    let mut result = Err(PortCLError::Timeout("Initial timeout".to_string()));

    while attempts < max_attempts && result.is_err() {
        attempts += 1;
        result = attempt_operation(attempts);

        if result.is_err() {
            let error = result.as_ref().unwrap_err();
            if is_retryable_error(error) {
                continue; // Retry
            } else {
                break; // Don't retry non-retryable errors
            }
        }
    }

    // Should succeed on attempt 3
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Operation succeeded");
}

#[tokio::test]
async fn test_error_severity_classification() {
    // Test that errors are classified with appropriate severity
    let critical_errors = vec![
        PortCLError::Configuration("Invalid system configuration".to_string()),
    ];

    let high_severity_errors = vec![
        PortCLError::Portage("Portage installation failed".to_string()),
        PortCLError::ActionExecution("Package installation failed".to_string()),
        PortCLError::System("System out of memory".to_string()),
        PortCLError::Service("Critical service down".to_string()),
    ];

    let medium_severity_errors = vec![
        PortCLError::RLEngine("Model training convergence issue".to_string()),
        PortCLError::Validation("Input validation failed".to_string()),
        PortCLError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied")),
        PortCLError::NotFound("Resource temporarily unavailable".to_string()),
    ];

    let low_severity_errors = vec![
        PortCLError::Network(reqwest::Error::from(io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"))),
        PortCLError::Timeout("Operation timed out".to_string()),
        PortCLError::Json(serde_json::Error::custom("JSON parsing failed")),
        PortCLError::TomlDeserialize(toml::de::Error::custom("TOML parsing failed")),
        PortCLError::TomlSerialize(toml::ser::Error::custom("TOML serialization failed")),
    ];

    for error in critical_errors {
        assert_eq!(error_severity(&error), ErrorSeverity::Critical);
    }

    for error in high_severity_errors {
        assert_eq!(error_severity(&error), ErrorSeverity::High);
    }

    for error in medium_severity_errors {
        assert_eq!(error_severity(&error), ErrorSeverity::Medium);
    }

    for error in low_severity_errors {
        assert_eq!(error_severity(&error), ErrorSeverity::Low);
    }
}

#[tokio::test]
async fn test_error_message_consistency() {
    // Test that error messages are consistent and informative
    let error = PortCLError::Portage("Emerge command failed".to_string());
    let message = format!("{}", error);

    assert!(!message.is_empty());
    assert!(message.len() > 10); // Should be reasonably descriptive
    assert!(!message.contains("Portage API error: Portage API error:")); // No double prefix
}

#[tokio::test]
async fn test_error_serialization() {
    // Test that errors can be serialized for logging/remote handling
    let error = PortCLError::Validation("Invalid configuration value".to_string());

    // Test JSON serialization
    let json_result = serde_json::to_string(&error.to_string());
    assert!(json_result.is_ok());

    let json_str = json_result.unwrap();
    let deserialized: String = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized, error.to_string());
}

#[tokio::test]
async fn test_error_in_async_context() {
    // Test error handling in async functions
    async fn async_operation(success: bool) -> Result<String> {
        if success {
            Ok("Async operation completed".to_string())
        } else {
            Err(PortCLError::Timeout("Async operation timed out".to_string()))
        }
    }

    // Test success case
    let result = async_operation(true).await;
    assert!(result.is_ok());

    // Test error case
    let result = async_operation(false).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Timeout(msg) => {
            assert_eq!(msg, "Async operation timed out");
        },
        _ => panic!("Expected Timeout error"),
    }
}

#[tokio::test]
async fn test_error_aggregation() {
    // Test scenarios where multiple errors might occur
    let operations = vec![
        || Err(PortCLError::Validation("Invalid input".to_string())),
        || Err(PortCLError::Io(io::Error::new(io::ErrorKind::NotFound, "File not found"))),
        || Ok::<(), PortCLError>(()),
        || Err(PortCLError::Network(reqwest::Error::from(io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused")))),
    ];

    let mut errors = Vec::new();
    let mut successes = 0;

    for operation in operations {
        match operation() {
            Ok(_) => successes += 1,
            Err(e) => errors.push(e),
        }
    }

    assert_eq!(successes, 1);
    assert_eq!(errors.len(), 3);

    // Verify error types
    assert!(errors.iter().any(|e| matches!(e, PortCLError::Validation(_))));
    assert!(errors.iter().any(|e| matches!(e, PortCLError::Io(_))));
    assert!(errors.iter().any(|e| matches!(e, PortCLError::Network(_))));
}

#[tokio::test]
async fn test_custom_error_creation() {
    // Test creating custom errors with specific messages
    fn create_portage_error(operation: &str) -> PortCLError {
        PortCLError::Portage(format!("Failed to execute {}: command not found", operation))
    }

    fn create_validation_error(field: &str, value: &str) -> PortCLError {
        PortCLError::Validation(format!("Invalid value '{}' for field '{}'", value, field))
    }

    let portage_error = create_portage_error("emerge");
    let validation_error = create_validation_error("parallelism", "0");

    assert!(matches!(portage_error, PortCLError::Portage(_)));
    assert!(matches!(validation_error, PortCLError::Validation(_)));

    let portage_msg = format!("{}", portage_error);
    let validation_msg = format!("{}", validation_error);

    assert!(portage_msg.contains("emerge"));
    assert!(portage_msg.contains("command not found"));
    assert!(validation_msg.contains("parallelism"));
    assert!(validation_msg.contains("0"));
}

#[tokio::test]
async fn test_error_from_standard_types() {
    // Test conversion from standard error types
    let std_io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
    let portcl_io_error: PortCLError = std_io_error.into();
    assert!(matches!(portcl_io_error, PortCLError::Io(_)));

    let std_json_error = serde_json::Error::custom("JSON parsing error");
    let portcl_json_error: PortCLError = std_json_error.into();
    assert!(matches!(portcl_json_error, PortCLError::Json(_)));
}

#[tokio::test]
async fn test_error_handling_patterns() {
    // Test common error handling patterns
    fn divide(a: f64, b: f64) -> Result<f64> {
        if b == 0.0 {
            Err(PortCLError::Validation("Division by zero".to_string()))
        } else {
            Ok(a / b)
        }
    }

    fn calculate(values: &[f64]) -> Result<f64> {
        if values.is_empty() {
            return Err(PortCLError::Validation("Empty input array".to_string()));
        }

        let sum: f64 = values.iter().sum();
        let count = values.len() as f64;

        divide(sum, count)
    }

    // Test successful calculation
    let result = calculate(&[1.0, 2.0, 3.0, 4.0]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2.5);

    // Test empty input
    let result = calculate(&[]);
    assert!(result.is_err());
    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Empty input array"));
        },
        _ => panic!("Expected Validation error"),
    }

    // Test division by zero (if all values are zero)
    let result = calculate(&[0.0, 0.0, 0.0]);
    assert!(result.is_err());
    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Division by zero"));
        },
        _ => panic!("Expected Validation error"),
    }
}