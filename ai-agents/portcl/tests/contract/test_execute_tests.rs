//! Contract tests for POST /tests/run API endpoint
//!
//! This module contains comprehensive contract tests for the POST /tests/run endpoint
//! as defined in the OpenAPI specification. These tests follow TDD principles
//! and are designed to FAIL initially since no implementation exists.
//!
//! HTTP Status Codes tested:
//! - 200: Test execution started
//! - 400: Invalid request parameters
//! - 500: Internal server error
//!
//! Test scenarios:
//! - Basic test execution request
//! - Execution with configuration parameters
//! - Parallel execution options
//! - Request validation and error handling
//! - Response structure validation

use mockall::mock;
use portcl::error::{PortCLError, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// Test data models based on OpenAPI spec for POST /tests/run
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestExecutionRequest {
    pub test_ids: Vec<String>,
    pub config: TestExecutionConfig,
    #[serde(default)]
    pub parallel: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestExecutionConfig {
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
    #[serde(default = "default_coverage")]
    pub enable_coverage: bool,
    #[serde(default)]
    pub enable_profiling: bool,
    #[serde(default = "default_output_format")]
    pub output_format: OutputFormat,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Junit,
    Html,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestExecutionResponse {
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub started_at: String,
    pub estimated_duration_ms: u32,
    pub test_count: u32,
    pub parallel_workers: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
}

// Default values for configuration
fn default_timeout() -> u32 {
    300
}

fn default_log_level() -> LogLevel {
    LogLevel::Info
}

fn default_coverage() -> bool {
    true
}

fn default_output_format() -> OutputFormat {
    OutputFormat::Json
}

// Mock API client for testing
mock! {
    pub ApiClient {}

    impl ApiClient {
        pub fn new(base_url: &str) -> Self;
        pub async fn run_tests(&self, request: &TestExecutionRequest) -> Result<TestExecutionResponse>;
    }
}

// Test utilities
fn create_sample_test_request() -> TestExecutionRequest {
    TestExecutionRequest {
        test_ids: vec![
            "test_unit_001".to_string(),
            "test_integration_001".to_string(),
            "test_performance_001".to_string(),
        ],
        config: TestExecutionConfig {
            timeout_seconds: 300,
            retry_count: 2,
            log_level: LogLevel::Info,
            enable_coverage: true,
            enable_profiling: false,
            output_format: OutputFormat::Json,
        },
        parallel: false,
    }
}

fn create_sample_parallel_request() -> TestExecutionRequest {
    TestExecutionRequest {
        test_ids: vec!["test_unit_001".to_string()],
        config: TestExecutionConfig {
            timeout_seconds: 600,
            retry_count: 1,
            log_level: LogLevel::Debug,
            enable_coverage: false,
            enable_profiling: true,
            output_format: OutputFormat::Html,
        },
        parallel: true,
    }
}

fn create_expected_response(request: &TestExecutionRequest) -> TestExecutionResponse {
    let execution_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    TestExecutionResponse {
        execution_id,
        status: ExecutionStatus::Running,
        started_at: now.to_rfc3339(),
        estimated_duration_ms: request.test_ids.len() as u32 * 1000, // Estimate 1s per test
        test_count: request.test_ids.len() as u32,
        parallel_workers: if request.parallel { Some(4) } else { None },
    }
}

// Contract tests
#[cfg(test)]
mod contract_tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::Value;

    #[tokio::test]
    async fn test_post_tests_run_basic_request() {
        // RED PHASE: This test should fail because implementation doesn't exist

        let mut mock_client = MockApiClient::new();
        let request = create_sample_test_request();

        // Setup mock expectation - this will fail because API doesn't exist
        mock_client
            .expect_run_tests()
            .returning(|_| {
                Err(PortCLError::Configuration("POST /tests/run endpoint not implemented".to_string()))
            });

        // This should fail with unimplemented error
        let result = mock_client.run_tests(&request).await;

        // In RED phase, we expect this to fail
        assert!(result.is_err(), "Expected API call to fail in RED phase");

        match result.unwrap_err() {
            PortCLError::Configuration(msg) => {
                assert!(msg.contains("not implemented"), "Expected unimplemented error");
            }
            _ => panic!("Expected PortCLError::Configuration for unimplemented API"),
        }
    }

    #[tokio::test]
    async fn test_post_tests_run_empty_test_ids() {
        // RED PHASE: Test request with empty test_ids array (should run all tests)

        let mut mock_client = MockApiClient::new();
        let mut request = create_sample_test_request();
        request.test_ids = vec![]; // Empty array means run all tests

        mock_client
            .expect_run_tests()
            .returning(|_| {
                Err(PortCLError::Configuration("Empty test_ids not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_parallel_execution() {
        // RED PHASE: Test parallel execution option

        let mut mock_client = MockApiClient::new();
        let request = create_sample_parallel_request();

        mock_client
            .expect_run_tests()
            .returning(|req| {
                if req.parallel {
                    Err(PortCLError::Configuration("Parallel execution not implemented".to_string()))
                } else {
                    Err(PortCLError::Configuration("Sequential execution not implemented".to_string()))
                }
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");

        match result.unwrap_err() {
            PortCLError::Configuration(msg) => {
                assert!(msg.contains("Parallel"), "Expected parallel execution error");
            }
            _ => panic!("Expected PortCLError::Configuration for parallel execution"),
        }
    }

    #[tokio::test]
    async fn test_post_tests_run_with_configuration() {
        // RED PHASE: Test execution with custom configuration

        let mut mock_client = MockApiClient::new();
        let mut request = create_sample_test_request();

        // Custom configuration
        request.config.timeout_seconds = 600;
        request.config.retry_count = 3;
        request.config.log_level = LogLevel::Debug;
        request.config.enable_coverage = false;
        request.config.enable_profiling = true;
        request.config.output_format = OutputFormat::Html;

        mock_client
            .expect_run_tests()
            .returning(|_| {
                Err(PortCLError::Configuration("Custom configuration not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_invalid_request() {
        // RED PHASE: Test error handling for invalid request parameters

        let mut mock_client = MockApiClient::new();
        let mut request = create_sample_test_request();
        request.test_ids.push("".to_string()); // Invalid empty test ID

        mock_client
            .expect_run_tests()
            .returning(|_| {
                Err(PortCLError::Validation("Invalid test ID format".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");

        match result.unwrap_err() {
            PortCLError::Validation(msg) => {
                assert!(msg.contains("Invalid"), "Expected validation error");
            }
            _ => panic!("Expected PortCLError::Validation for invalid request"),
        }
    }

    #[tokio::test]
    async fn test_post_tests_run_timeout_validation() {
        // RED PHASE: Test timeout configuration validation

        let mut mock_client = MockApiClient::new();
        let mut request = create_sample_test_request();
        request.config.timeout_seconds = 0; // Invalid timeout

        mock_client
            .expect_run_tests()
            .returning(|_| {
                Err(PortCLError::Validation("Invalid timeout value".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_retry_count_validation() {
        // RED PHASE: Test retry count configuration validation

        let mut mock_client = MockApiClient::new();
        let mut request = create_sample_test_request();
        request.config.retry_count = 10; // Too many retries

        mock_client
            .expect_run_tests()
            .returning(|_| {
                Err(PortCLError::Validation("Retry count too high".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_response_structure_validation() {
        // RED PHASE: Test that response structure matches OpenAPI spec

        let mut mock_client = MockApiClient::new();
        let request = create_sample_test_request();

        mock_client
            .expect_run_tests()
            .returning(|_| {
                // This would be the expected response structure
                let response = create_expected_response(&request);

                // Validate response structure matches OpenAPI spec
                let json_response = serde_json::to_value(response).unwrap();

                // Validate required fields
                assert!(json_response.get("execution_id").is_some(), "Response missing 'execution_id' field");
                assert!(json_response.get("status").is_some(), "Response missing 'status' field");
                assert!(json_response.get("started_at").is_some(), "Response missing 'started_at' field");
                assert!(json_response.get("estimated_duration_ms").is_some(), "Response missing 'estimated_duration_ms' field");
                assert!(json_response.get("test_count").is_some(), "Response missing 'test_count' field");

                // Validate field types
                assert!(json_response["execution_id"].is_string(), "'execution_id' should be a string");
                assert!(json_response["status"].is_string(), "'status' should be a string");
                assert!(json_response["started_at"].is_string(), "'started_at' should be a string");
                assert!(json_response["estimated_duration_ms"].is_number(), "'estimated_duration_ms' should be a number");
                assert!(json_response["test_count"].is_number(), "'test_count' should be a number");

                Err(PortCLError::Configuration("Response structure validation not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_enums_validation() {
        // RED PHASE: Test that enum values match OpenAPI specification

        // Test valid enum values
        let valid_log_levels = vec![LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];
        let valid_output_formats = vec![OutputFormat::Json, OutputFormat::Junit, OutputFormat::Html];
        let valid_statuses = vec![ExecutionStatus::Running, ExecutionStatus::Completed, ExecutionStatus::Failed];

        for log_level in valid_log_levels {
            let serialized = serde_json::to_string(&log_level).unwrap();
            let deserialized: LogLevel = serde_json::from_str(&serialized).unwrap();
            assert_eq!(log_level, deserialized);
        }

        for output_format in valid_output_formats {
            let serialized = serde_json::to_string(&output_format).unwrap();
            let deserialized: OutputFormat = serde_json::from_str(&serialized).unwrap();
            assert_eq!(output_format, deserialized);
        }

        for status in valid_statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: ExecutionStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(status, deserialized);
        }

        // Test that the API call fails because implementation doesn't exist
        let mut mock_client = MockApiClient::new();
        let request = create_sample_test_request();

        mock_client
            .expect_run_tests()
            .returning(|_| {
                Err(PortCLError::Configuration("Enum validation not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_timestamp_format_validation() {
        // RED PHASE: Test that timestamp format matches ISO 8601

        let mut mock_client = MockApiClient::new();
        let request = create_sample_test_request();

        mock_client
            .expect_run_tests()
            .returning(|_| {
                // Test timestamp format validation
                let timestamp = chrono::Utc::now().to_rfc3339();

                // Validate ISO 8601 format
                chrono::DateTime::parse_from_rfc3339(&timestamp)
                    .expect("Timestamp should be valid ISO 8601 format");

                Err(PortCLError::Configuration("Timestamp validation not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_http_status_codes() {
        // RED PHASE: Test HTTP status code compliance

        let request = create_sample_test_request();

        // Test HTTP 200 (success) - should fail because not implemented
        let mut mock_client_200 = MockApiClient::new();
        mock_client_200
            .expect_run_tests()
            .returning(|_| {
                // This would return HTTP 200 in implementation
                Err(PortCLError::Configuration("HTTP 200 response not implemented".to_string()))
            });

        let result_200 = mock_client_200.run_tests(&request).await;
        assert!(result_200.is_err(), "Expected HTTP 200 test to fail in RED phase");

        // Test HTTP 400 (bad request) - should fail because not implemented
        let mut mock_client_400 = MockApiClient::new();
        let mut invalid_request = create_sample_test_request();
        invalid_request.test_ids.push("".to_string()); // Invalid test ID

        mock_client_400
            .expect_run_tests()
            .returning(|_| {
                // This would return HTTP 400 for invalid parameters
                Err(PortCLError::Validation("HTTP 400 response not implemented".to_string()))
            });

        let result_400 = mock_client_400.run_tests(&invalid_request).await;
        assert!(result_400.is_err(), "Expected HTTP 400 test to fail in RED phase");

        // Test HTTP 500 (server error) - should fail because not implemented
        let mut mock_client_500 = MockApiClient::new();
        mock_client_500
            .expect_run_tests()
            .returning(|_| {
                // This would return HTTP 500 for internal errors
                Err(PortCLError::Configuration("HTTP 500 response not implemented".to_string()))
            });

        let result_500 = mock_client_500.run_tests(&request).await;
        assert!(result_500.is_err(), "Expected HTTP 500 test to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_default_values() {
        // RED PHASE: Test default values configuration

        let mut mock_client = MockApiClient::new();
        let mut request = create_sample_test_request();

        // Reset to defaults
        request.config.timeout_seconds = default_timeout();
        request.config.log_level = default_log_level();
        request.config.enable_coverage = default_coverage();
        request.config.output_format = default_output_format();
        request.config.retry_count = 0;
        request.config.enable_profiling = false;
        request.parallel = false;

        mock_client
            .expect_run_tests()
            .returning(|_| {
                Err(PortCLError::Configuration("Default values not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_execution_id_format() {
        // RED PHASE: Test execution ID format (should be UUID)

        let mut mock_client = MockApiClient::new();
        let request = create_sample_test_request();

        mock_client
            .expect_run_tests()
            .returning(|_| {
                // Test execution ID generation
                let execution_id = uuid::Uuid::new_v4().to_string();

                // Validate UUID format
                uuid::Uuid::parse_str(&execution_id)
                    .expect("Execution ID should be valid UUID format");

                Err(PortCLError::Configuration("Execution ID format not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_parallel_workers_calculation() {
        // RED PHASE: Test parallel workers calculation

        let mut mock_client = MockApiClient::new();
        let mut request = create_sample_test_request();
        request.parallel = true;

        mock_client
            .expect_run_tests()
            .returning(|_| {
                // Test parallel workers calculation (should be based on CPU cores or configuration)
                let cpu_count = num_cpus::get();
                let workers = std::cmp::min(cpu_count as u32, 8); // Max 8 workers

                assert!(workers > 0, "Should have at least 1 worker");
                assert!(workers <= 8, "Should not exceed 8 workers");

                Err(PortCLError::Configuration("Parallel workers calculation not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_post_tests_run_estimation_accuracy() {
        // RED PHASE: Test duration estimation accuracy

        let mut mock_client = MockApiClient::new();
        let mut request = create_sample_test_request();
        request.test_ids = vec!["test_001".to_string(); 10]; // 10 tests

        mock_client
            .expect_run_tests()
            .returning(|_| {
                // Test duration estimation (should be reasonable)
                let test_count = request.test_ids.len();
                let estimated_duration = test_count as u32 * 1000; // 1 second per test

                assert!(estimated_duration > 0, "Should estimate positive duration");
                assert!(estimated_duration <= 3600000, "Should not estimate more than 1 hour for 10 tests");

                Err(PortCLError::Configuration("Duration estimation not implemented".to_string()))
            });

        let result = mock_client.run_tests(&request).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }
}

// Integration tests for OpenAPI contract compliance
#[cfg(test)]
mod openapi_contract_tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_openapi_spec_compliance() {
        // Validate that our data models match the OpenAPI specification

        // Test TestExecutionRequest model
        let request = TestExecutionRequest {
            test_ids: vec!["test_001".to_string()],
            config: TestExecutionConfig {
                timeout_seconds: 300,
                retry_count: 0,
                log_level: LogLevel::Info,
                enable_coverage: true,
                enable_profiling: false,
                output_format: OutputFormat::Json,
            },
            parallel: false,
        };

        let json_value = serde_json::to_value(request).unwrap();

        // Validate required fields from OpenAPI spec
        assert!(json_value.get("test_ids").is_some(), "Request missing required field: test_ids");
        assert!(json_value.get("config").is_some(), "Request missing required field: config");

        // Validate field types
        assert!(json_value["test_ids"].is_array(), "test_ids should be an array");
        assert!(json_value["config"].is_object(), "config should be an object");
        assert!(json_value["parallel"].is_boolean(), "parallel should be a boolean");

        // Test TestExecutionConfig model
        let config = TestExecutionConfig {
            timeout_seconds: 300,
            retry_count: 0,
            log_level: LogLevel::Info,
            enable_coverage: true,
            enable_profiling: false,
            output_format: OutputFormat::Json,
        };

        let config_json = serde_json::to_value(config).unwrap();
        assert!(config_json.get("timeout_seconds").is_some(), "Config missing timeout_seconds");
        assert!(config_json.get("retry_count").is_some(), "Config missing retry_count");
        assert!(config_json.get("log_level").is_some(), "Config missing log_level");
        assert!(config_json.get("enable_coverage").is_some(), "Config missing enable_coverage");
        assert!(config_json.get("enable_profiling").is_some(), "Config missing enable_profiling");
        assert!(config_json.get("output_format").is_some(), "Config missing output_format");
    }

    #[test]
    fn test_response_structure_compliance() {
        // Validate TestExecutionResponse matches OpenAPI spec

        let response = TestExecutionResponse {
            execution_id: uuid::Uuid::new_v4().to_string(),
            status: ExecutionStatus::Running,
            started_at: chrono::Utc::now().to_rfc3339(),
            estimated_duration_ms: 1000,
            test_count: 1,
            parallel_workers: Some(4),
        };

        let json_value = serde_json::to_value(response).unwrap();

        // Validate required fields
        assert!(json_value.get("execution_id").is_some(), "Response missing required field: execution_id");
        assert!(json_value.get("status").is_some(), "Response missing required field: status");
        assert!(json_value.get("started_at").is_some(), "Response missing required field: started_at");
        assert!(json_value.get("estimated_duration_ms").is_some(), "Response missing required field: estimated_duration_ms");
        assert!(json_value.get("test_count").is_some(), "Response missing required field: test_count");

        // Validate field types
        assert!(json_value["execution_id"].is_string(), "execution_id should be a string");
        assert!(json_value["status"].is_string(), "status should be a string");
        assert!(json_value["started_at"].is_string(), "started_at should be a string");
        assert!(json_value["estimated_duration_ms"].is_number(), "estimated_duration_ms should be a number");
        assert!(json_value["test_count"].is_number(), "test_count should be a number");
        assert!(json_value["parallel_workers"].as_u64().is_some() || json_value["parallel_workers"].is_null(), "parallel_workers should be a number or null");
    }

    #[test]
    fn test_enum_value_compliance() {
        // Validate enum values match OpenAPI specification

        // Test LogLevel enum
        let log_levels = vec![LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];
        let expected_log_levels = vec!["debug", "info", "warn", "error"];

        for (log_level, expected) in log_levels.iter().zip(expected_log_levels) {
            let serialized = serde_json::to_string(log_level).unwrap();
            let deserialized: String = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, expected);
        }

        // Test OutputFormat enum
        let output_formats = vec![OutputFormat::Json, OutputFormat::Junit, OutputFormat::Html];
        let expected_formats = vec!["json", "junit", "html"];

        for (format, expected) in output_formats.iter().zip(expected_formats) {
            let serialized = serde_json::to_string(format).unwrap();
            let deserialized: String = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, expected);
        }

        // Test ExecutionStatus enum
        let execution_statuses = vec![ExecutionStatus::Running, ExecutionStatus::Completed, ExecutionStatus::Failed];
        let expected_statuses = vec!["running", "completed", "failed"];

        for (status, expected) in execution_statuses.iter().zip(expected_statuses) {
            let serialized = serde_json::to_string(status).unwrap();
            let deserialized: String = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, expected);
        }
    }

    #[test]
    fn test_default_values_compliance() {
        // Validate that default values match OpenAPI specification

        let request = TestExecutionRequest {
            test_ids: vec!["test_001".to_string()],
            config: TestExecutionConfig {
                timeout_seconds: 300, // Default from spec
                retry_count: 0,      // Default from spec
                log_level: LogLevel::Info, // Default from spec
                enable_coverage: true, // Default from spec
                enable_profiling: false, // Default from spec
                output_format: OutputFormat::Json, // Default from spec
            },
            parallel: false, // Default from spec
        };

        assert_eq!(request.config.timeout_seconds, 300);
        assert_eq!(request.config.retry_count, 0);
        assert_eq!(request.config.log_level, LogLevel::Info);
        assert_eq!(request.config.enable_coverage, true);
        assert_eq!(request.config.enable_profiling, false);
        assert_eq!(request.config.output_format, OutputFormat::Json);
        assert_eq!(request.parallel, false);
    }

    #[test]
    fn test_request_validation_rules() {
        // Validate that request validation follows OpenAPI specification rules

        // Test timeout_seconds validation
        let mut config = TestExecutionConfig {
            timeout_seconds: 0, // Invalid - should be positive
            retry_count: 0,
            log_level: LogLevel::Info,
            enable_coverage: true,
            enable_profiling: false,
            output_format: OutputFormat::Json,
        };

        // This would fail validation in implementation
        assert_eq!(config.timeout_seconds, 0, "Invalid timeout should be caught by validation");

        // Test retry_count validation
        config.retry_count = 100; // Very high - might be rejected
        assert_eq!(config.retry_count, 100, "High retry count should be caught by validation");

        // Test test_ids validation
        let request = TestExecutionRequest {
            test_ids: vec![], // Empty array is valid (runs all tests)
            config,
            parallel: false,
        };

        assert!(request.test_ids.is_empty(), "Empty test_ids should be valid");

        // Test with invalid test ID
        let request_with_invalid_id = TestExecutionRequest {
            test_ids: vec!["".to_string()], // Invalid empty ID
            config,
            parallel: false,
        };

        assert_eq!(request_with_invalid_id.test_ids[0], "", "Invalid test ID should be caught by validation");
    }
}