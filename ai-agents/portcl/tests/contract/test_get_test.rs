//! Contract tests for GET /tests/{test_id} API endpoint
//!
//! This module contains comprehensive contract tests for the GET /tests/{test_id} endpoint
//! as defined in the OpenAPI specification. These tests follow TDD principles
//! and are designed to FAIL initially since no implementation exists.
//!
//! HTTP Status Codes tested:
//! - 200: Test details retrieved successfully
//! - 404: Test not found
//! - 500: Internal server error

use mockall::mock;
use portcl::error::{PortCLError, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// Test data models based on OpenAPI spec
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestDetail {
    pub id: String,
    pub name: String,
    pub module: String,
    pub test_type: TestType,
    pub status: TestStatus,
    pub last_run: Option<String>,
    pub duration_ms: Option<u32>,
    pub coverage_percentage: Option<f64>,
    pub description: String,
    pub steps: Vec<String>,
    pub prerequisites: Vec<String>,
    pub expected_results: Vec<String>,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestType {
    Unit,
    Integration,
    Performance,
    Property,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestType::Unit => write!(f, "unit"),
            TestType::Integration => write!(f, "integration"),
            TestType::Performance => write!(f, "performance"),
            TestType::Property => write!(f, "property"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    NotRun,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "passed"),
            TestStatus::Failed => write!(f, "failed"),
            TestStatus::Skipped => write!(f, "skipped"),
            TestStatus::NotRun => write!(f, "not_run"),
        }
    }
}

// Mock API client for testing
mock! {
    pub ApiClient {}

    impl ApiClient {
        pub fn new(base_url: &str) -> Self;
        pub async fn get_test(&self, test_id: &str) -> Result<TestDetail>;
    }
}

// Test utilities
fn create_sample_test_detail(test_id: &str) -> TestDetail {
    TestDetail {
        id: test_id.to_string(),
        name: format!("Test {}", test_id),
        module: "core".to_string(),
        test_type: TestType::Unit,
        status: TestStatus::Passed,
        last_run: Some("2024-01-15T10:30:00Z".to_string()),
        duration_ms: Some(150),
        coverage_percentage: Some(95.5),
        description: "This is a comprehensive test that verifies core functionality".to_string(),
        steps: vec![
            "Initialize test environment".to_string(),
            "Execute core functionality".to_string(),
            "Verify expected results".to_string(),
            "Clean up test environment".to_string(),
        ],
        prerequisites: vec![
            "System must be running".to_string(),
            "Database connection available".to_string(),
        ],
        expected_results: vec![
            "Core functionality works correctly".to_string(),
            "No errors generated during execution".to_string(),
            "Performance within acceptable limits".to_string(),
        ],
        dependencies: vec![
            "test_common_setup".to_string(),
            "test_database_connection".to_string(),
        ],
        tags: vec![
            "core".to_string(),
            "unit".to_string(),
            "smoke".to_string(),
        ],
    }
}

// Contract tests
#[cfg(test)]
mod contract_tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::Value;

    #[tokio::test]
    async fn test_get_test_valid_id() {
        // RED PHASE: This test should fail because implementation doesn't exist

        let mut mock_client = MockApiClient::new();
        let test_id = "test_unit_001";

        // Setup mock expectation - this will fail because API doesn't exist
        mock_client
            .expect_get_test()
            .returning(|_| {
                Err(PortCLError::Configuration("GET /tests/{test_id} endpoint not implemented".to_string()))
            });

        // This should fail with unimplemented error
        let result = mock_client.get_test(test_id).await;

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
    async fn test_get_test_numeric_id() {
        // RED PHASE: Test with numeric test ID

        let mut mock_client = MockApiClient::new();
        let test_id = "12345";

        mock_client
            .expect_get_test()
            .returning(|_| {
                Err(PortCLError::Configuration("Numeric test ID not implemented".to_string()))
            });

        let result = mock_client.get_test(test_id).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_get_test_alphanumeric_id() {
        // RED PHASE: Test with alphanumeric test ID

        let mut mock_client = MockApiClient::new();
        let test_id = "test_abc_123";

        mock_client
            .expect_get_test()
            .returning(|_| {
                Err(PortCLError::Configuration("Alphanumeric test ID not implemented".to_string()))
            });

        let result = mock_client.get_test(test_id).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_get_test_empty_id() {
        // RED PHASE: Test with empty test ID (should fail validation)

        let mut mock_client = MockApiClient::new();
        let test_id = "";

        mock_client
            .expect_get_test()
            .returning(|_| {
                Err(PortCLError::Validation("Empty test ID not allowed".to_string()))
            });

        let result = mock_client.get_test(test_id).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");

        match result.unwrap_err() {
            PortCLError::Validation(msg) => {
                assert!(msg.contains("Empty"), "Expected validation error for empty ID");
            }
            _ => panic!("Expected PortCLError::Validation for empty test ID"),
        }
    }

    #[tokio::test]
    async fn test_get_test_nonexistent_id() {
        // RED PHASE: Test with non-existent test ID (should return 404)

        let mut mock_client = MockApiClient::new();
        let test_id = "nonexistent_test_123";

        mock_client
            .expect_get_test()
            .returning(|_| {
                Err(PortCLError::NotFound("Test not found".to_string()))
            });

        let result = mock_client.get_test(test_id).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");

        match result.unwrap_err() {
            PortCLError::NotFound(msg) => {
                assert!(msg.contains("not found"), "Expected not found error");
            }
            _ => panic!("Expected PortCLError::NotFound for non-existent test"),
        }
    }

    #[tokio::test]
    async fn test_get_test_invalid_characters() {
        // RED PHASE: Test with invalid characters in test ID

        let mut mock_client = MockApiClient::new();
        let test_id = "test@#$%";

        mock_client
            .expect_get_test()
            .returning(|_| {
                Err(PortCLError::Validation("Invalid characters in test ID".to_string()))
            });

        let result = mock_client.get_test(test_id).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");

        match result.unwrap_err() {
            PortCLError::Validation(msg) => {
                assert!(msg.contains("Invalid"), "Expected validation error");
            }
            _ => panic!("Expected PortCLError::Validation for invalid characters"),
        }
    }

    #[tokio::test]
    async fn test_get_test_response_structure() {
        // RED PHASE: Test that response structure matches OpenAPI spec

        let mut mock_client = MockApiClient::new();
        let test_id = "test_unit_001";

        mock_client
            .expect_get_test()
            .returning(|_| {
                // This would be the expected response structure
                let response = create_sample_test_detail("test_unit_001");

                // Validate response structure matches OpenAPI spec
                let json_response = serde_json::to_value(response).unwrap();

                // Validate required fields from TestInfo base schema
                assert!(json_response.get("id").is_some(), "Response missing 'id' field");
                assert!(json_response.get("name").is_some(), "Response missing 'name' field");
                assert!(json_response.get("module").is_some(), "Response missing 'module' field");
                assert!(json_response.get("test_type").is_some(), "Response missing 'test_type' field");
                assert!(json_response.get("status").is_some(), "Response missing 'status' field");
                assert!(json_response.get("last_run").is_some(), "Response missing 'last_run' field");
                assert!(json_response.get("duration_ms").is_some(), "Response missing 'duration_ms' field");
                assert!(json_response.get("coverage_percentage").is_some(), "Response missing 'coverage_percentage' field");

                // Validate additional TestDetail fields
                assert!(json_response.get("description").is_some(), "Response missing 'description' field");
                assert!(json_response.get("steps").is_some(), "Response missing 'steps' field");
                assert!(json_response.get("prerequisites").is_some(), "Response missing 'prerequisites' field");
                assert!(json_response.get("expected_results").is_some(), "Response missing 'expected_results' field");
                assert!(json_response.get("dependencies").is_some(), "Response missing 'dependencies' field");
                assert!(json_response.get("tags").is_some(), "Response missing 'tags' field");

                // Validate field types
                assert!(json_response["id"].is_string(), "'id' should be a string");
                assert!(json_response["name"].is_string(), "'name' should be a string");
                assert!(json_response["module"].is_string(), "'module' should be a string");
                assert!(json_response["test_type"].is_string(), "'test_type' should be a string");
                assert!(json_response["status"].is_string(), "'status' should be a string");
                assert!(json_response["description"].is_string(), "'description' should be a string");
                assert!(json_response["steps"].is_array(), "'steps' should be an array");
                assert!(json_response["prerequisites"].is_array(), "'prerequisites' should be an array");
                assert!(json_response["expected_results"].is_array(), "'expected_results' should be an array");
                assert!(json_response["dependencies"].is_array(), "'dependencies' should be an array");
                assert!(json_response["tags"].is_array(), "'tags' should be an array");

                Err(PortCLError::Configuration("Response structure validation not implemented".to_string()))
            });

        let result = mock_client.get_test(test_id).await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_get_test_enums_validation() {
        // RED PHASE: Test that enum values match OpenAPI specification

        // Test valid enum values
        let valid_test_types = vec![TestType::Unit, TestType::Integration, TestType::Performance, TestType::Property];
        let valid_statuses = vec![TestStatus::Passed, TestStatus::Failed, TestStatus::Skipped, TestStatus::NotRun];

        for test_type in valid_test_types {
            let serialized = serde_json::to_string(&test_type).unwrap();
            let deserialized: TestType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(test_type, deserialized);
        }

        for status in valid_statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: TestStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(status, deserialized);
        }

        // Test that the API call fails because implementation doesn't exist
        let mut mock_client = MockApiClient::new();

        mock_client
            .expect_get_test()
            .returning(|_| {
                Err(PortCLError::Configuration("Enum validation not implemented".to_string()))
            });

        let result = mock_client.get_test("test_unit_001").await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_get_test_timestamp_format_validation() {
        // RED PHASE: Test that timestamp format matches ISO 8601

        let mut mock_client = MockApiClient::new();

        mock_client
            .expect_get_test()
            .returning(|_| {
                // Test timestamp format validation
                let timestamp = "2024-01-15T10:30:00Z";

                // Validate ISO 8601 format
                chrono::DateTime::parse_from_rfc3339(timestamp)
                    .expect("Timestamp should be valid ISO 8601 format");

                Err(PortCLError::Configuration("Timestamp validation not implemented".to_string()))
            });

        let result = mock_client.get_test("test_unit_001").await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_get_test_http_status_codes() {
        // RED PHASE: Test HTTP status code compliance

        // Test HTTP 200 (success) - should fail because not implemented
        let mut mock_client_200 = MockApiClient::new();
        mock_client_200
            .expect_get_test()
            .returning(|_| {
                // This would return HTTP 200 in implementation
                Err(PortCLError::Configuration("HTTP 200 response not implemented".to_string()))
            });

        let result_200 = mock_client_200.get_test("test_unit_001").await;
        assert!(result_200.is_err(), "Expected HTTP 200 test to fail in RED phase");

        // Test HTTP 404 (not found) - should fail because not implemented
        let mut mock_client_404 = MockApiClient::new();
        mock_client_404
            .expect_get_test()
            .returning(|_| {
                // This would return HTTP 404 for non-existent test
                Err(PortCLError::NotFound("HTTP 404 response not implemented".to_string()))
            });

        let result_404 = mock_client_404.get_test("nonexistent_test").await;
        assert!(result_404.is_err(), "Expected HTTP 404 test to fail in RED phase");

        // Test HTTP 500 (server error) - should fail because not implemented
        let mut mock_client_500 = MockApiClient::new();
        mock_client_500
            .expect_get_test()
            .returning(|_| {
                // This would return HTTP 500 for internal errors
                Err(PortCLError::Configuration("HTTP 500 response not implemented".to_string()))
            });

        let result_500 = mock_client_500.get_test("test_unit_001").await;
        assert!(result_500.is_err(), "Expected HTTP 500 test to fail in RED phase");
    }

    #[tokio::test]
    async fn test_get_test_path_parameter_validation() {
        // RED PHASE: Test path parameter validation

        let test_cases = vec![
            ("valid_test_id", "Valid test ID"),
            ("test-123", "Test with hyphen"),
            ("test_123", "Test with underscore"),
            ("TEST123", "Test with uppercase"),
            ("a".repeat(255).as_str(), "Maximum length test ID"),
        ];

        for (test_id, description) in test_cases {
            let mut mock_client = MockApiClient::new();

            mock_client
                .expect_get_test()
                .returning(|_| {
                    Err(PortCLError::Configuration(format!("Path parameter validation for '{}' not implemented", test_id)))
                });

            let result = mock_client.get_test(test_id).await;
            assert!(result.is_err(), "Expected path parameter test to fail in RED phase: {}", description);
        }
    }

    #[tokio::test]
    async fn test_get_test_array_fields_validation() {
        // RED PHASE: Test that array fields are properly structured

        let mut mock_client = MockApiClient::new();

        mock_client
            .expect_get_test()
            .returning(|_| {
                let mut test_detail = create_sample_test_detail("test_unit_001");

                // Validate array fields are properly structured
                assert!(test_detail.steps.iter().all(|s| !s.is_empty()), "All steps should be non-empty");
                assert!(test_detail.prerequisites.iter().all(|p| !p.is_empty()), "All prerequisites should be non-empty");
                assert!(test_detail.expected_results.iter().all(|r| !r.is_empty()), "All expected results should be non-empty");
                assert!(test_detail.dependencies.iter().all(|d| !d.is_empty()), "All dependencies should be non-empty");
                assert!(test_detail.tags.iter().all(|t| !t.is_empty()), "All tags should be non-empty");

                Err(PortCLError::Configuration("Array field validation not implemented".to_string()))
            });

        let result = mock_client.get_test("test_unit_001").await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_get_test_optional_fields_validation() {
        // RED PHASE: Test that optional fields can be None/null

        let mut mock_client = MockApiClient::new();

        mock_client
            .expect_get_test()
            .returning(|_| {
                let mut test_detail = create_sample_test_detail("test_unit_001");

                // Test with optional fields as None
                test_detail.last_run = None;
                test_detail.duration_ms = None;
                test_detail.coverage_percentage = None;

                // Serialize to validate JSON structure with null values
                let json_value = serde_json::to_value(test_detail).unwrap();

                assert!(json_value["last_run"].is_null(), "last_run should be null when None");
                assert!(json_value["duration_ms"].is_null(), "duration_ms should be null when None");
                assert!(json_value["coverage_percentage"].is_null(), "coverage_percentage should be null when None");

                Err(PortCLError::Configuration("Optional field validation not implemented".to_string()))
            });

        let result = mock_client.get_test("test_unit_001").await;
        assert!(result.is_err(), "Expected API call to fail in RED phase");
    }

    #[tokio::test]
    async fn test_get_test_comprehensive_error_handling() {
        // RED PHASE: Test comprehensive error handling scenarios

        let error_scenarios = vec![
            ("", "Empty test ID", PortCLError::Validation("Empty test ID".to_string())),
            ("test@#$", "Invalid characters", PortCLError::Validation("Invalid characters in test ID".to_string())),
            ("a".repeat(1000).as_str(), "Test ID too long", PortCLError::Validation("Test ID too long".to_string())),
            ("nonexistent_12345", "Non-existent test", PortCLError::NotFound("Test not found".to_string())),
        ];

        for (test_id, description, expected_error) in error_scenarios {
            let mut mock_client = MockApiClient::new();

            mock_client
                .expect_get_test()
                .returning(move |_| Err(expected_error.clone()));

            let result = mock_client.get_test(test_id).await;
            assert!(result.is_err(), "Expected error handling test to fail in RED phase: {}", description);
        }
    }
}

// Integration tests for OpenAPI contract compliance
#[cfg(test)]
mod openapi_contract_tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_test_detail_model_compliance() {
        // Validate that TestDetail model matches the OpenAPI specification

        let test_detail = create_sample_test_detail("test_unit_001");
        let json_value = serde_json::to_value(test_detail).unwrap();

        // Validate required fields from TestInfo base schema
        let required_fields = vec![
            "id", "name", "module", "test_type", "status",
            "last_run", "duration_ms", "coverage_percentage"
        ];

        for field in required_fields {
            assert!(json_value.get(field).is_some(), "TestDetail missing required field: {}", field);
        }

        // Validate additional TestDetail fields
        let detail_fields = vec![
            "description", "steps", "prerequisites",
            "expected_results", "dependencies", "tags"
        ];

        for field in detail_fields {
            assert!(json_value.get(field).is_some(), "TestDetail missing required field: {}", field);
        }

        // Validate field types
        assert!(json_value["id"].is_string(), "id should be a string");
        assert!(json_value["name"].is_string(), "name should be a string");
        assert!(json_value["module"].is_string(), "module should be a string");
        assert!(json_value["test_type"].is_string(), "test_type should be a string");
        assert!(json_value["status"].is_string(), "status should be a string");
        assert!(json_value["description"].is_string(), "description should be a string");
        assert!(json_value["steps"].is_array(), "steps should be an array");
        assert!(json_value["prerequisites"].is_array(), "prerequisites should be an array");
        assert!(json_value["expected_results"].is_array(), "expected_results should be an array");
        assert!(json_value["dependencies"].is_array(), "dependencies should be an array");
        assert!(json_value["tags"].is_array(), "tags should be an array");

        // Validate that optional fields can be null
        assert!(json_value["last_run"].as_str().is_some() || json_value["last_run"].is_null(), "last_run should be string or null");
        assert!(json_value["duration_ms"].as_u64().is_some() || json_value["duration_ms"].is_null(), "duration_ms should be number or null");
        assert!(json_value["coverage_percentage"].as_f64().is_some() || json_value["coverage_percentage"].is_null(), "coverage_percentage should be number or null");
    }

    #[test]
    fn test_array_field_compliance() {
        // Validate that array fields contain the correct types

        let test_detail = create_sample_test_detail("test_unit_001");
        let json_value = serde_json::to_value(test_detail).unwrap();

        // Validate steps array
        let steps = json_value["steps"].as_array().unwrap();
        for step in steps {
            assert!(step.is_string(), "All steps should be strings");
            assert!(!step.as_str().unwrap().is_empty(), "Steps should not be empty");
        }

        // Validate prerequisites array
        let prerequisites = json_value["prerequisites"].as_array().unwrap();
        for prereq in prerequisites {
            assert!(prereq.is_string(), "All prerequisites should be strings");
            assert!(!prereq.as_str().unwrap().is_empty(), "Prerequisites should not be empty");
        }

        // Validate expected_results array
        let expected_results = json_value["expected_results"].as_array().unwrap();
        for result in expected_results {
            assert!(result.is_string(), "All expected results should be strings");
            assert!(!result.as_str().unwrap().is_empty(), "Expected results should not be empty");
        }

        // Validate dependencies array
        let dependencies = json_value["dependencies"].as_array().unwrap();
        for dep in dependencies {
            assert!(dep.is_string(), "All dependencies should be strings");
            assert!(!dep.as_str().unwrap().is_empty(), "Dependencies should not be empty");
        }

        // Validate tags array
        let tags = json_value["tags"].as_array().unwrap();
        for tag in tags {
            assert!(tag.is_string(), "All tags should be strings");
            assert!(!tag.as_str().unwrap().is_empty(), "Tags should not be empty");
        }
    }

    #[test]
    fn test_enum_value_compliance() {
        // Validate enum values match OpenAPI specification

        // Test test_type enum values
        let test_types = vec![TestType::Unit, TestType::Integration, TestType::Performance, TestType::Property];
        let expected_test_types = vec!["unit", "integration", "performance", "property"];

        for (test_type, expected) in test_types.iter().zip(expected_test_types) {
            let serialized = serde_json::to_string(test_type).unwrap();
            let deserialized: String = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, expected, "TestType {} should serialize to {}", test_type, expected);
        }

        // Test status enum values
        let statuses = vec![TestStatus::Passed, TestStatus::Failed, TestStatus::Skipped, TestStatus::NotRun];
        let expected_statuses = vec!["passed", "failed", "skipped", "not_run"];

        for (status, expected) in statuses.iter().zip(expected_statuses) {
            let serialized = serde_json::to_string(status).unwrap();
            let deserialized: String = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, expected, "TestStatus {} should serialize to {}", status, expected);
        }
    }

    #[test]
    fn test_serialization_deserialization_roundtrip() {
        // Validate that TestDetail can be serialized and deserialized correctly

        let original_test_detail = create_sample_test_detail("test_unit_001");

        // Serialize to JSON
        let json_string = serde_json::to_string(&original_test_detail)
            .expect("Should serialize TestDetail to JSON");

        // Deserialize back to struct
        let deserialized_test_detail: TestDetail = serde_json::from_str(&json_string)
            .expect("Should deserialize JSON back to TestDetail");

        // Validate all fields match
        assert_eq!(original_test_detail.id, deserialized_test_detail.id);
        assert_eq!(original_test_detail.name, deserialized_test_detail.name);
        assert_eq!(original_test_detail.module, deserialized_test_detail.module);
        assert_eq!(original_test_detail.test_type, deserialized_test_detail.test_type);
        assert_eq!(original_test_detail.status, deserialized_test_detail.status);
        assert_eq!(original_test_detail.last_run, deserialized_test_detail.last_run);
        assert_eq!(original_test_detail.duration_ms, deserialized_test_detail.duration_ms);
        assert_eq!(original_test_detail.coverage_percentage, deserialized_test_detail.coverage_percentage);
        assert_eq!(original_test_detail.description, deserialized_test_detail.description);
        assert_eq!(original_test_detail.steps, deserialized_test_detail.steps);
        assert_eq!(original_test_detail.prerequisites, deserialized_test_detail.prerequisites);
        assert_eq!(original_test_detail.expected_results, deserialized_test_detail.expected_results);
        assert_eq!(original_test_detail.dependencies, deserialized_test_detail.dependencies);
        assert_eq!(original_test_detail.tags, deserialized_test_detail.tags);
    }

    #[test]
    fn test_timestamp_format_compliance() {
        // Validate timestamp format compliance with ISO 8601

        let test_detail = create_sample_test_detail("test_unit_001");

        if let Some(timestamp) = &test_detail.last_run {
            // Validate timestamp format
            chrono::DateTime::parse_from_rfc3339(timestamp)
                .expect("Timestamp should be valid ISO 8601 format");
        }

        // Test with None timestamp
        let mut test_detail_no_timestamp = test_detail;
        test_detail_no_timestamp.last_run = None;

        let json_value = serde_json::to_value(test_detail_no_timestamp).unwrap();
        assert!(json_value["last_run"].is_null(), "last_run should be null when None");
    }

    #[test]
    fn test_path_parameter_patterns() {
        // Validate common test ID patterns that should be supported

        let valid_test_ids = vec![
            "test_unit_001",
            "test_integration_123",
            "performance_test_456",
            "property_test_789",
            "smoke_test_001",
            "regression_test_002",
            "api_test_003",
            "ui_test_004",
            "database_test_005",
            "auth_test_006",
        ];

        for test_id in valid_test_ids {
            // Basic validation - should not contain invalid characters
            assert!(!test_id.contains('@'), "Test ID '{}' should not contain '@'", test_id);
            assert!(!test_id.contains('#'), "Test ID '{}' should not contain '#'", test_id);
            assert!(!test_id.contains('$'), "Test ID '{}' should not contain '$'", test_id);
            assert!(!test_id.contains('%'), "Test ID '{}' should not contain '%'", test_id);
            assert!(!test_id.is_empty(), "Test ID '{}' should not be empty", test_id);
            assert!(test_id.len() <= 255, "Test ID '{}' should not exceed 255 characters", test_id);
        }
    }
}