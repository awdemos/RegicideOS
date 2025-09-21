//! Test helpers and utilities for PortCL testing

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::{tempdir, TempDir};
use serde::{Serialize, Deserialize};
use tokio::time::sleep;
use uuid::Uuid;
use std::sync::{Arc, RwLock};
use futures::future;

use crate::fixtures::mock_data::*;
use crate::fixtures::test_models::*;
use crate::fixtures::mock_monitor::*;
use crate::fixtures::mock_executor::*;
use crate::fixtures::mock_agent::*;

pub mod test_assertions;
pub mod mock_environment;
pub mod test_runner;
pub mod data_validator;
pub mod benchmark_helpers;

pub use test_assertions::*;
pub use mock_environment::*;
pub use test_runner::*;
pub use data_validator::*;
pub use benchmark_helpers::*;

/// Creates a temporary directory for tests
pub fn create_temp_dir() -> TempDir {
    tempdir().expect("Failed to create temporary directory")
}

/// Creates a temporary file with content
pub fn create_temp_file_with_content(content: &str) -> PathBuf {
    let temp_dir = create_temp_dir();
    let file_path = temp_dir.path().join("test_file.txt");
    fs::write(&file_path, content).expect("Failed to write temporary file");
    file_path
}

/// Creates a mock JSON configuration file
pub fn create_mock_config_file() -> PathBuf {
    let config_content = r#"
{
    "api_key": "test_api_key",
    "base_url": "https://api.test.com",
    "timeout_seconds": 30,
    "max_retries": 3,
    "log_level": "info",
    "enable_ml": false
}
"#;
    create_temp_file_with_content(config_content)
}

/// Creates a mock TOML configuration file
pub fn create_mock_toml_file() -> PathBuf {
    let toml_content = r#"
[api]
key = "test_api_key"
base_url = "https://api.test.com"

[performance]
timeout_seconds = 30
max_retries = 3

[logging]
level = "info"
enable_ml = false
"#;
    create_temp_file_with_content(toml_content)
}

/// Returns test data directory path
pub fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data")
}

/// Ensures test data directory exists
pub fn ensure_test_data_dir() -> PathBuf {
    let data_dir = test_data_dir();
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).expect("Failed to create test data directory");
    }
    data_dir
}

/// Helper for testing async functions
pub async fn async_test_wrapper<F, R>(test_fn: F) -> R
where
    F: std::future::Future<Output = R>,
{
    test_fn.await
}

/// Mock implementation for testing error conditions
pub fn mock_io_error() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, "Mock IO error")
}

/// Mock implementation for testing JSON errors
pub fn mock_json_error() -> serde_json::Error {
    serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err()
}

/// Mock implementation for testing TOML errors
pub fn mock_toml_error() -> toml::de::Error {
    toml::from_str::<toml::Value>("invalid toml").unwrap_err()
}

/// Creates a mock HTTP response for testing
pub fn mock_http_response() -> reqwest::Response {
    reqwest::Response::from(
        http::Response::builder()
            .status(200)
            .body("mock response".to_string())
            .unwrap(),
    )
}

/// Helper to measure execution time
pub fn measure_execution_time<F, R>(f: F) -> (R, std::time::Duration)
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Assertion helper for comparing floating point values with tolerance
pub fn assert_almost_equal(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "Expected {} ± {}, got {} (diff: {})",
        expected, tolerance, actual, diff
    );
}

/// Helper to create mock UUID for testing
pub fn mock_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
}

/// Helper to create mock timestamp for testing
pub fn mock_timestamp() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}

/// Helper to validate file permissions
pub fn validate_file_permissions(path: &Path, expected_mode: u32) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            return permissions.mode() & 0o777 == expected_mode;
        }
    }
    false
}

/// Helper to validate file ownership (Unix only)
#[cfg(unix)]
pub fn validate_file_ownership(path: &Path, expected_uid: u32, expected_gid: u32) -> bool {
    use std::os::unix::fs::MetadataExt;
    if let Ok(metadata) = fs::metadata(path) {
        return metadata.uid() == expected_uid && metadata.gid() == expected_gid;
    }
    false
}

/// Helper to check if a path is secure (no symlinks, proper permissions)
pub fn is_secure_path(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    // Check for symlinks
    if path.is_symlink() {
        return false;
    }

    // Check parent directories
    if let Some(parent) = path.parent() {
        if !is_secure_path(parent) {
            return false;
        }
    }

    // Basic permission check (world-writable is insecure)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            if mode & 0o002 != 0 { // world-writable
                return false;
            }
        }
    }

    true
}

/// Helper macros for common test patterns
#[macro_export]
macro_rules! assert_test_success {
    ($result:expr) => {
        assert_eq!($result.status, TestStatus::Passed,
            "Test should have passed but got status: {:?}", $result.status);
    };
}

#[macro_export]
macro_rules! assert_test_failure {
    ($result:expr) => {
        assert_eq!($result.status, TestStatus::Failed,
            "Test should have failed but got status: {:?}", $result.status);
    };
}

#[macro_export]
macro_rules! assert_metrics_improvement {
    ($before:expr, $after:expr, $metric:expr) => {
        assert!($after.$metric > $before.$metric,
            "Metric {} should have improved from {} to {}",
            stringify!($metric), $before.$metric, $after.$metric);
    };
}

#[macro_export]
macro_rules! assert_within_tolerance {
    ($value:expr, $expected:expr, $tolerance:expr) => {
        let diff = ($value - $expected).abs();
        assert!(diff <= $tolerance,
            "Value {} is not within tolerance {} of expected {}",
            $value, $tolerance, $expected);
    };
}

/// Common test timeouts and thresholds
pub const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const SHORT_TEST_TIMEOUT: Duration = Duration::from_secs(5);
pub const LONG_TEST_TIMEOUT: Duration = Duration::from_millis(120_000);
pub const DEFAULT_ASSERTION_TOLERANCE: f64 = 0.001;
pub const METRIC_IMPROVEMENT_THRESHOLD: f64 = 0.1;

/// Test assertion helpers for common validation patterns
pub struct TestAssertionHelpers;

impl TestAssertionHelpers {
    /// Assert that a test result meets basic success criteria
    pub fn assert_test_result_valid(result: &TestResult) -> Result<(), String> {
        if result.test_id.is_empty() {
            return Err("Test ID cannot be empty".to_string());
        }

        if result.start_time > result.end_time {
            return Err("Start time cannot be after end time".to_string());
        }

        if result.duration_ms == 0 && result.status != TestStatus::Skipped {
            return Err("Duration cannot be zero for non-skipped tests".to_string());
        }

        // Validate metrics
        if result.metrics.coverage_percent < 0.0 || result.metrics.coverage_percent > 100.0 {
            return Err("Coverage percentage must be between 0 and 100".to_string());
        }

        Ok(())
    }

    /// Assert that metrics show improvement over baseline
    pub fn assert_metrics_improved(
        baseline: &TestMetrics,
        current: &TestMetrics,
        min_improvement: f64,
    ) -> Result<(), String> {
        let improvements = vec![
            ("cpu_usage", baseline.cpu_usage_percent - current.cpu_usage_percent),
            ("memory_usage", baseline.memory_usage_percent - current.memory_usage_percent),
            ("disk_usage", baseline.disk_usage_percent - current.disk_usage_percent),
        ];

        let mut has_improvement = false;

        for (metric, improvement) in improvements {
            if improvement > min_improvement {
                has_improvement = true;
                break;
            }
        }

        if !has_improvement {
            return Err(format!(
                "No metrics improved by at least {}. Improvements: {:?}",
                min_improvement, improvements
            ));
        }

        Ok(())
    }

    /// Assert that test execution is within acceptable time bounds
    pub fn assert_execution_time_bounds(
        duration_ms: u64,
        min_expected: u64,
        max_expected: u64,
    ) -> Result<(), String> {
        if duration_ms < min_expected {
            return Err(format!(
                "Execution time {}ms is less than minimum expected {}ms",
                duration_ms, min_expected
            ));
        }

        if duration_ms > max_expected {
            return Err(format!(
                "Execution time {}ms exceeds maximum expected {}ms",
                duration_ms, max_expected
            ));
        }

        Ok(())
    }

    /// Assert that test output contains expected content
    pub fn assert_output_contains(
        output: &TestOutput,
        expected_content: &str,
    ) -> Result<(), String> {
        let content = match output {
            TestOutput::Success { stdout, stderr: _ } => stdout,
            TestOutput::Failure { stdout, stderr, error: _ } => {
                format!("{}: {}", stdout, stderr)
            },
            TestOutput::Timeout { message } => message,
            TestOutput::Error { message } => message,
            TestOutput::Skipped { reason } => reason,
        };

        if !content.contains(expected_content) {
            return Err(format!(
                "Expected output to contain '{}', but got: '{}'",
                expected_content, content
            ));
        }

        Ok(())
    }

    /// Assert that mock agent behavior is consistent
    pub fn assert_mock_agent_behavior(
        agent: &MockPortageAgent,
        expected_learning_mode: bool,
        min_experience_count: usize,
    ) -> Result<(), String> {
        let state = agent.state.blocking_read();

        if state.learning_mode != expected_learning_mode {
            return Err(format!(
                "Expected learning mode {}, got {}",
                expected_learning_mode, state.learning_mode
            ));
        }

        if state.experience_count < min_experience_count {
            return Err(format!(
                "Expected at least {} experiences, got {}",
                min_experience_count, state.experience_count
            ));
        }

        Ok(())
    }

    /// Assert that mock monitor metrics are valid
    pub fn assert_monitor_metrics_valid(metrics: &MockMonitorMetrics) -> Result<(), String> {
        if metrics.system_metrics.cpu_usage_percent < 0.0 || metrics.system_metrics.cpu_usage_percent > 100.0 {
            return Err("CPU usage must be between 0 and 100".to_string());
        }

        if metrics.system_metrics.memory_usage_percent < 0.0 || metrics.system_metrics.memory_usage_percent > 100.0 {
            return Err("Memory usage must be between 0 and 100".to_string());
        }

        if metrics.system_metrics.disk_usage_percent < 0.0 || metrics.system_metrics.disk_usage_percent > 100.0 {
            return Err("Disk usage must be between 0 and 100".to_string());
        }

        if metrics.system_metrics.load_average_1min < 0.0 {
            return Err("Load average cannot be negative".to_string());
        }

        Ok(())
    }

    /// Assert that action execution history is consistent
    pub fn assert_execution_history_consistent(
        history: &[MockActionResult],
        expected_min_actions: usize,
    ) -> Result<(), String> {
        if history.len() < expected_min_actions {
            return Err(format!(
                "Expected at least {} actions in history, got {}",
                expected_min_actions, history.len()
            ));
        }

        // Check for valid timestamps
        for (i, action) in history.iter().enumerate() {
            if action.start_time > action.end_time {
                return Err(format!(
                    "Action {} has invalid timestamp: start > end",
                    i
                ));
            }
        }

        // Check chronological order
        for i in 1..history.len() {
            if history[i-1].end_time > history[i].start_time {
                return Err(format!(
                    "Actions {} and {} are not in chronological order",
                    i-1, i
                ));
            }
        }

        Ok(())
    }

    /// Assert that test configuration is valid
    pub fn assert_test_config_valid(config: &TestConfig) -> Result<(), String> {
        if config.name.is_empty() {
            return Err("Test name cannot be empty".to_string());
        }

        if config.test_type == TestType::Unknown {
            return Err("Test type cannot be Unknown".to_string());
        }

        if config.timeout_ms == 0 {
            return Err("Timeout cannot be zero".to_string());
        }

        // Validate parallel execution constraints
        if config.max_parallel_tasks == 0 {
            return Err("Max parallel tasks cannot be zero".to_string());
        }

        // Validate retry configuration
        if config.max_retries == 0 && config.enable_retry {
            return Err("Max retries cannot be zero when retry is enabled".to_string());
        }

        Ok(())
    }
}

/// Builder pattern for creating test environments with controlled setup
pub struct MockEnvironmentBuilder {
    packages: Vec<MockPackage>,
    config: Option<MockPortageConfig>,
    actions: Vec<MockAction>,
    monitor_config: Option<MockMonitoringConfig>,
    executor_config: Option<MockActionConfig>,
    agent_config: Option<MockRLConfig>,
    error_injection: HashMap<String, bool>,
    delay_injection: HashMap<String, u64>,
}

impl MockEnvironmentBuilder {
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            config: None,
            actions: Vec::new(),
            monitor_config: None,
            executor_config: None,
            agent_config: None,
            error_injection: HashMap::new(),
            delay_injection: HashMap::new(),
        }
    }

    pub fn with_packages(mut self, packages: Vec<MockPackage>) -> Self {
        self.packages = packages;
        self
    }

    pub fn with_config(mut self, config: MockPortageConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_actions(mut self, actions: Vec<MockAction>) -> Self {
        self.actions = actions;
        self
    }

    pub fn with_monitor_config(mut self, config: MockMonitoringConfig) -> Self {
        self.monitor_config = Some(config);
        self
    }

    pub fn with_executor_config(mut self, config: MockActionConfig) -> Self {
        self.executor_config = Some(config);
        self
    }

    pub fn with_agent_config(mut self, config: MockRLConfig) -> Self {
        self.agent_config = Some(config);
        self
    }

    pub fn with_error_injection(mut self, key: String, should_error: bool) -> Self {
        self.error_injection.insert(key, should_error);
        self
    }

    pub fn with_delay_injection(mut self, key: String, delay_ms: u64) -> Self {
        self.delay_injection.insert(key, delay_ms);
        self
    }

    pub fn build(self) -> Result<MockTestEnvironment, String> {
        let config = self.config.unwrap_or_else(|| MockPortageConfig::sample_config());
        let monitor_config = self.monitor_config.unwrap_or_else(|| MockMonitoringConfig::default());
        let executor_config = self.executor_config.unwrap_or_else(|| MockActionConfig::default());
        let agent_config = self.agent_config.unwrap_or_else(|| MockRLConfig::default());

        Ok(MockTestEnvironment {
            packages: self.packages,
            config,
            actions: self.actions,
            monitor: Arc::new(RwLock::new(MockPortageMonitor::new_with_config(
                monitor_config,
                self.packages.clone(),
            ))),
            executor: Arc::new(RwLock::new(MockActionExecutor::new_with_config(
                executor_config,
            ))),
            agent: Arc::new(RwLock::new(MockPortageAgent::new_with_config(
                agent_config,
            ))),
            error_injection: Arc::new(RwLock::new(self.error_injection)),
            delay_injection: Arc::new(RwLock::new(self.delay_injection)),
            created_at: SystemTime::now(),
        })
    }
}

impl Default for MockEnvironmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete test environment with all mock components
#[derive(Debug, Clone)]
pub struct MockTestEnvironment {
    pub packages: Vec<MockPackage>,
    pub config: MockPortageConfig,
    pub actions: Vec<MockAction>,
    pub monitor: Arc<RwLock<MockPortageMonitor>>,
    pub executor: Arc<RwLock<MockActionExecutor>>,
    pub agent: Arc<RwLock<MockPortageAgent>>,
    pub error_injection: Arc<RwLock<HashMap<String, bool>>>,
    pub delay_injection: Arc<RwLock<HashMap<String, u64>>>,
    pub created_at: SystemTime,
}

impl MockTestEnvironment {
    pub async fn reset(&self) -> Result<(), String> {
        // Reset all components to initial state
        let mut monitor = self.monitor.write().await;
        monitor.reset().await;

        let mut executor = self.executor.write().await;
        executor.reset().await;

        let mut agent = self.agent.write().await;
        agent.reset().await;

        Ok(())
    }

    pub async fn inject_error(&self, component: &str, should_error: bool) -> Result<(), String> {
        let mut errors = self.error_injection.write().await;
        errors.insert(component.to_string(), should_error);
        Ok(())
    }

    pub async fn inject_delay(&self, component: &str, delay_ms: u64) -> Result<(), String> {
        let mut delays = self.delay_injection.write().await;
        delays.insert(component.to_string(), delay_ms);
        Ok(())
    }

    pub async fn get_environment_state(&self) -> MockEnvironmentState {
        let monitor = self.monitor.read().await;
        let executor = self.executor.read().await;
        let agent = self.agent.read().await;

        MockEnvironmentState {
            packages: self.packages.clone(),
            config: self.config.clone(),
            actions: self.actions.clone(),
            monitor_metrics: monitor.get_metrics().await,
            executor_state: executor.get_state().await,
            agent_state: agent.get_state().await,
            created_at: self.created_at,
            current_time: SystemTime::now(),
        }
    }
}

/// Environment state snapshot for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEnvironmentState {
    pub packages: Vec<MockPackage>,
    pub config: MockPortageConfig,
    pub actions: Vec<MockAction>,
    pub monitor_metrics: PortageMetrics,
    pub executor_state: MockExecutionState,
    pub agent_state: MockModelState,
    pub created_at: SystemTime,
    pub current_time: SystemTime,
}

/// Test runner for executing tests with proper setup/teardown
pub struct TestRunner {
    pub config: TestRunnerConfig,
    pub environment: Option<MockTestEnvironment>,
    pub results: Vec<TestResult>,
    pub start_time: SystemTime,
}

#[derive(Debug, Clone)]
pub struct TestRunnerConfig {
    pub parallel_execution: bool,
    pub max_concurrent_tests: usize,
    pub default_timeout: Duration,
    pub cleanup_on_failure: bool,
    pub verbose_logging: bool,
    pub collect_metrics: bool,
}

impl Default for TestRunnerConfig {
    fn default() -> Self {
        Self {
            parallel_execution: true,
            max_concurrent_tests: 4,
            default_timeout: DEFAULT_TEST_TIMEOUT,
            cleanup_on_failure: true,
            verbose_logging: false,
            collect_metrics: true,
        }
    }
}

impl TestRunner {
    pub fn new(config: TestRunnerConfig) -> Self {
        Self {
            config,
            environment: None,
            results: Vec::new(),
            start_time: SystemTime::now(),
        }
    }

    pub fn with_environment(mut self, env: MockTestEnvironment) -> Self {
        self.environment = Some(env);
        self
    }

    pub async fn setup_environment(&mut self) -> Result<(), String> {
        if self.environment.is_none() {
            let env = MockEnvironmentBuilder::new()
                .build()
                .map_err(|e| format!("Failed to build environment: {}", e))?;
            self.environment = Some(env);
        }
        Ok(())
    }

    pub async fn run_test(&mut self, test_config: &TestConfig) -> Result<TestResult, String> {
        let start_time = SystemTime::now();
        let test_id = Uuid::new_v4().to_string();

        // Setup test environment
        if let Some(env) = &self.environment {
            env.reset().await?;
        }

        // Execute test (simplified - would be more sophisticated in practice)
        let result = self.execute_test_internal(test_config, test_id, start_time).await;

        // Collect result
        self.results.push(result.clone());

        Ok(result)
    }

    async fn execute_test_internal(
        &self,
        config: &TestConfig,
        test_id: String,
        start_time: SystemTime,
    ) -> TestResult {
        // This is a simplified test execution
        // In a real implementation, this would execute the actual test logic

        let duration_ms = start_time.elapsed()
            .unwrap_or(Duration::from_millis(100))
            .as_millis() as u64;

        let output = TestOutput::Success {
            stdout: format!("Test {} completed successfully", config.name),
            stderr: String::new(),
        };

        let metrics = TestMetrics {
            coverage_percent: 85.0,
            execution_time_ms: duration_ms,
            memory_usage_mb: 50.0,
            assertions_passed: 10,
            assertions_failed: 0,
            cpu_usage_percent: 25.0,
            disk_usage_percent: 15.0,
        };

        TestResult {
            test_id,
            test_name: config.name.clone(),
            test_type: config.test_type.clone(),
            status: TestStatus::Passed,
            duration_ms,
            start_time,
            end_time: start_time + Duration::from_millis(duration_ms),
            output,
            metrics,
            dependencies: config.dependencies.clone(),
            tags: config.tags.clone(),
            metadata: config.metadata.clone(),
        }
    }

    pub async fn run_test_suite(&mut self, test_configs: &[TestConfig]) -> Vec<TestResult> {
        let mut results = Vec::new();

        if self.config.parallel_execution {
            // Run tests in parallel with limited concurrency
            for chunk in test_configs.chunks(self.config.max_concurrent_tests) {
                let chunk_results: Vec<_> = future::join_all(
                    chunk.iter().map(|config| self.run_test(config))
                ).await;

                for result in chunk_results {
                    match result {
                        Ok(test_result) => results.push(test_result),
                        Err(e) => {
                            // Create a failure result for the error
                            let failed_result = TestResult {
                                test_id: Uuid::new_v4().to_string(),
                                test_name: "unknown".to_string(),
                                test_type: TestType::Unit,
                                status: TestStatus::Failed,
                                duration_ms: 0,
                                start_time: SystemTime::now(),
                                end_time: SystemTime::now(),
                                output: TestOutput::Error { message: e },
                                metrics: TestMetrics::default(),
                                dependencies: Vec::new(),
                                tags: Vec::new(),
                                metadata: HashMap::new(),
                            };
                            results.push(failed_result);
                        }
                    }
                }
            }
        } else {
            // Run tests sequentially
            for config in test_configs {
                if let Ok(result) = self.run_test(config).await {
                    results.push(result);
                }
            }
        }

        self.results.extend(results.clone());
        results
    }

    pub fn get_summary(&self) -> TestRunSummary {
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.status == TestStatus::Passed).count();
        let failed = self.results.iter().filter(|r| r.status == TestStatus::Failed).count();
        let skipped = self.results.iter().filter(|r| r.status == TestStatus::Skipped).count();

        let total_duration = self.results.iter()
            .map(|r| r.duration_ms)
            .sum::<u64>();

        let average_coverage = if total > 0 {
            self.results.iter()
                .map(|r| r.metrics.coverage_percent)
                .sum::<f64>() / total as f64
        } else {
            0.0
        };

        TestRunSummary {
            total_tests: total,
            passed_tests: passed,
            failed_tests: failed,
            skipped_tests: skipped,
            total_duration_ms: total_duration,
            average_coverage_percent: average_coverage,
            success_rate: if total > 0 { passed as f64 / total as f64 } else { 0.0 },
            start_time: self.start_time,
            end_time: SystemTime::now(),
        }
    }
}

/// Summary of test run results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSummary {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub total_duration_ms: u64,
    pub average_coverage_percent: f64,
    pub success_rate: f64,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
}

/// Comprehensive data validation for test fixtures
pub struct TestDataValidator;

impl TestDataValidator {
    /// Validate a complete test environment
    pub fn validate_environment(env: &MockTestEnvironment) -> Result<(), String> {
        // Validate packages
        for (i, package) in env.packages.iter().enumerate() {
            Self::validate_package(package)
                .map_err(|e| format!("Package {} validation failed: {}", i, e))?;
        }

        // Validate configuration
        Self::validate_config(&env.config)?;

        // Validate actions
        for (i, action) in env.actions.iter().enumerate() {
            Self::validate_action(action)
                .map_err(|e| format!("Action {} validation failed: {}", i, e))?;
        }

        Ok(())
    }

    /// Validate package data structure
    pub fn validate_package(package: &MockPackage) -> Result<(), String> {
        if package.name.is_empty() {
            return Err("Package name cannot be empty".to_string());
        }

        if package.version.is_empty() {
            return Err("Package version cannot be empty".to_string());
        }

        // Validate semantic version format
        if !Self::is_valid_version(&package.version) {
            return Err(format!("Invalid version format: {}", package.version));
        }

        // Validate category
        if package.category.is_empty() {
            return Err("Package category cannot be empty".to_string());
        }

        // Validate repository
        if !package.repository.starts_with("https://") && !package.repository.starts_with("git://") {
            return Err("Repository must be a valid URL".to_string());
        }

        // Validate dependencies
        for dep in &package.dependencies {
            if dep.is_empty() {
                return Err("Dependency cannot be empty".to_string());
            }
        }

        // Validate size
        if package.size_bytes == 0 {
            return Err("Package size cannot be zero".to_string());
        }

        Ok(())
    }

    /// Validate configuration structure
    pub fn validate_config(config: &MockPortageConfig) -> Result<(), String> {
        if config.profile.is_empty() {
            return Err("Profile cannot be empty".to_string());
        }

        if config.accept_keywords.is_empty() {
            return Err("Accept keywords cannot be empty".to_string());
        }

        // Validate makeopts
        if config.makeopts.jobs == 0 {
            return Err("Makeopts jobs cannot be zero".to_string());
        }

        // Validate features
        if config.features.is_empty() {
            return Err("Features cannot be empty".to_string());
        }

        // Validate use flags
        for flag in &config.use_flags {
            if flag.is_empty() {
                return Err("Use flag cannot be empty".to_string());
            }
        }

        Ok(())
    }

    /// Validate action structure
    pub fn validate_action(action: &MockAction) -> Result<(), String> {
        match action {
            MockAction::NoOp => Ok(()),
            MockAction::AdjustParallelism { jobs } => {
                if *jobs == 0 {
                    return Err("Parallel jobs cannot be zero".to_string());
                }
                if *jobs > 64 {
                    return Err("Parallel jobs exceeds reasonable limit".to_string());
                }
                Ok(())
            },
            MockAction::OptimizeBuildOrder { package_list } => {
                if package_list.is_empty() {
                    return Err("Package list cannot be empty".to_string());
                }
                for pkg in package_list {
                    if pkg.is_empty() {
                        return Err("Package name in list cannot be empty".to_string());
                    }
                }
                Ok(())
            },
            MockAction::ScheduleOperation { delay_seconds } => {
                if *delay_seconds == 0 {
                    return Err("Delay seconds cannot be zero".to_string());
                }
                if *delay_seconds > 86400 { // 24 hours
                    return Err("Delay seconds exceeds 24 hours".to_string());
                }
                Ok(())
            },
            MockAction::PreFetchDependencies { packages } => {
                if packages.is_empty() {
                    return Err("Packages for prefetch cannot be empty".to_string());
                }
                for pkg in packages {
                    if pkg.is_empty() {
                        return Err("Package name for prefetch cannot be empty".to_string());
                    }
                }
                Ok(())
            },
            MockAction::CleanObsoletePackages { force } => {
                // No specific validation needed for force flag
                Ok(())
            },
        }
    }

    /// Validate test results
    pub fn validate_test_result(result: &TestResult) -> Result<(), String> {
        if result.test_id.is_empty() {
            return Err("Test ID cannot be empty".to_string());
        }

        if result.test_name.is_empty() {
            return Err("Test name cannot be empty".to_string());
        }

        if result.start_time > result.end_time {
            return Err("Start time cannot be after end time".to_string());
        }

        if result.duration_ms == 0 && result.status != TestStatus::Skipped {
            return Err("Duration cannot be zero for non-skipped tests".to_string());
        }

        // Validate metrics
        Self::validate_metrics(&result.metrics)?;

        // Validate output
        Self::validate_test_output(&result.output)?;

        // Validate dependencies
        for dep in &result.dependencies {
            if dep.is_empty() {
                return Err("Dependency cannot be empty".to_string());
            }
        }

        // Validate tags
        for tag in &result.tags {
            if tag.is_empty() {
                return Err("Tag cannot be empty".to_string());
            }
        }

        Ok(())
    }

    /// Validate test metrics
    pub fn validate_metrics(metrics: &TestMetrics) -> Result<(), String> {
        if metrics.coverage_percent < 0.0 || metrics.coverage_percent > 100.0 {
            return Err("Coverage percentage must be between 0 and 100".to_string());
        }

        if metrics.execution_time_ms == 0 {
            return Err("Execution time cannot be zero".to_string());
        }

        if metrics.memory_usage_mb < 0.0 {
            return Err("Memory usage cannot be negative".to_string());
        }

        if metrics.assertions_passed + metrics.assertions_failed == 0 {
            return Err("Total assertions cannot be zero".to_string());
        }

        if metrics.cpu_usage_percent < 0.0 || metrics.cpu_usage_percent > 100.0 {
            return Err("CPU usage must be between 0 and 100".to_string());
        }

        if metrics.disk_usage_percent < 0.0 || metrics.disk_usage_percent > 100.0 {
            return Err("Disk usage must be between 0 and 100".to_string());
        }

        Ok(())
    }

    /// Validate test output
    pub fn validate_test_output(output: &TestOutput) -> Result<(), String> {
        match output {
            TestOutput::Success { stdout, stderr } => {
                // No specific validation required for success output
                Ok(())
            },
            TestOutput::Failure { stdout, stderr, error } => {
                if error.is_empty() {
                    return Err("Error message in failure output cannot be empty".to_string());
                }
                Ok(())
            },
            TestOutput::Timeout { message } => {
                if message.is_empty() {
                    return Err("Timeout message cannot be empty".to_string());
                }
                Ok(())
            },
            TestOutput::Error { message } => {
                if message.is_empty() {
                    return Err("Error message cannot be empty".to_string());
                }
                Ok(())
            },
            TestOutput::Skipped { reason } => {
                if reason.is_empty() {
                    return Err("Skip reason cannot be empty".to_string());
                }
                Ok(())
            },
        }
    }

    /// Validate mock monitor metrics
    pub fn validate_monitor_metrics(metrics: &MockMonitorMetrics) -> Result<(), String> {
        Self::validate_system_metrics(&metrics.system_metrics)?;

        if metrics.package_count == 0 {
            return Err("Package count cannot be zero".to_string());
        }

        if metrics.update_count < 0 {
            return Err("Update count cannot be negative".to_string());
        }

        Ok(())
    }

    /// Validate system metrics
    pub fn validate_system_metrics(metrics: &SystemMetrics) -> Result<(), String> {
        if metrics.cpu_usage_percent < 0.0 || metrics.cpu_usage_percent > 100.0 {
            return Err("CPU usage must be between 0 and 100".to_string());
        }

        if metrics.memory_usage_percent < 0.0 || metrics.memory_usage_percent > 100.0 {
            return Err("Memory usage must be between 0 and 100".to_string());
        }

        if metrics.disk_usage_percent < 0.0 || metrics.disk_usage_percent > 100.0 {
            return Err("Disk usage must be between 0 and 100".to_string());
        }

        if metrics.load_average_1min < 0.0 {
            return Err("Load average cannot be negative".to_string());
        }

        if metrics.load_average_5min < 0.0 {
            return Err("5-minute load average cannot be negative".to_string());
        }

        if metrics.load_average_15min < 0.0 {
            return Err("15-minute load average cannot be negative".to_string());
        }

        Ok(())
    }

    /// Validate mock agent state
    pub fn validate_agent_state(state: &MockModelState) -> Result<(), String> {
        if state.episode_count < 0 {
            return Err("Episode count cannot be negative".to_string());
        }

        if state.training_step < 0 {
            return Err("Training step cannot be negative".to_string());
        }

        if state.experience_count < 0 {
            return Err("Experience count cannot be negative".to_string());
        }

        if state.epsilon < 0.0 || state.epsilon > 1.0 {
            return Err("Epsilon must be between 0 and 1".to_string());
        }

        if state.learning_rate <= 0.0 || state.learning_rate > 1.0 {
            return Err("Learning rate must be between 0 and 1".to_string());
        }

        if state.discount_factor <= 0.0 || state.discount_factor > 1.0 {
            return Err("Discount factor must be between 0 and 1".to_string());
        }

        Ok(())
    }

    /// Validate test configuration
    pub fn validate_test_config(config: &TestConfig) -> Result<(), String> {
        if config.name.is_empty() {
            return Err("Test name cannot be empty".to_string());
        }

        if config.test_type == TestType::Unknown {
            return Err("Test type cannot be Unknown".to_string());
        }

        if config.timeout_ms == 0 {
            return Err("Timeout cannot be zero".to_string());
        }

        if config.max_parallel_tasks == 0 {
            return Err("Max parallel tasks cannot be zero".to_string());
        }

        if config.max_retries == 0 && config.enable_retry {
            return Err("Max retries cannot be zero when retry is enabled".to_string());
        }

        Ok(())
    }

    /// Helper function to validate semantic version format
    fn is_valid_version(version: &str) -> bool {
        // Basic semantic version validation: X.Y.Z
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 || parts.len() > 4 {
            return false;
        }

        for part in parts {
            if part.is_empty() {
                return false;
            }
            // Allow for versions with suffixes like 1.2.3-r1
            let numeric_part = part.split('-').next().unwrap_or(part);
            if numeric_part.chars().any(|c| !c.is_ascii_digit()) {
                return false;
            }
        }

        true
    }

    /// Validate consistency between related test data
    pub fn validate_consistency(
        results: &[TestResult],
        configs: &[TestConfig],
    ) -> Result<(), String> {
        // Check that all test results have corresponding configs
        for result in results {
            let matching_config = configs.iter().find(|config| config.name == result.test_name);
            if matching_config.is_none() {
                return Err(format!("Test result '{}' has no corresponding configuration", result.test_name));
            }
        }

        // Check for consistent timestamps
        if !results.is_empty() {
            let mut timestamps: Vec<SystemTime> = results.iter().map(|r| r.start_time).collect();
            timestamps.sort();

            for i in 1..timestamps.len() {
                if timestamps[i] < timestamps[i-1] {
                    return Err("Test timestamps are not in chronological order".to_string());
                }
            }
        }

        Ok(())
    }

    /// Validate test data for performance reasonableness
    pub fn validate_performance_reasonableness(results: &[TestResult]) -> Result<(), String> {
        for (i, result) in results.iter().enumerate() {
            // Check for unusually long execution times
            if result.duration_ms > 300_000 { // 5 minutes
                return Err(format!(
                    "Test '{}' execution time {}ms is unusually long",
                    result.test_name, result.duration_ms
                ));
            }

            // Check for unusually high memory usage
            if result.metrics.memory_usage_mb > 1000.0 { // 1GB
                return Err(format!(
                    "Test '{}' memory usage {}MB is unusually high",
                    result.test_name, result.metrics.memory_usage_mb
                ));
            }

            // Check for zero coverage in non-skipped tests
            if result.status != TestStatus::Skipped && result.metrics.coverage_percent == 0.0 {
                return Err(format!(
                    "Test '{}' has zero coverage but was not skipped",
                    result.test_name
                ));
            }
        }

        Ok(())
    }
}

/// Performance benchmarking helpers for PortCL testing
pub struct BenchmarkHelpers;

impl BenchmarkHelpers {
    /// Execute a function and measure its performance
    pub fn benchmark<F, R>(name: &str, f: F) -> BenchmarkResult
    where
        F: FnOnce() -> R,
    {
        let start_time = std::time::Instant::now();
        let start_memory = Self::get_memory_usage();

        let result = f();

        let end_time = std::time::Instant::now();
        let end_memory = Self::get_memory_usage();

        let duration = end_time - start_time;
        let memory_diff = end_memory.saturating_sub(start_memory);

        BenchmarkResult {
            name: name.to_string(),
            duration_ms: duration.as_millis() as u64,
            memory_usage_bytes: memory_diff,
            success: true,
            error_message: None,
            timestamp: SystemTime::now(),
        }
    }

    /// Execute an async function and measure its performance
    pub async fn benchmark_async<F, R>(name: &str, f: F) -> BenchmarkResult
    where
        F: std::future::Future<Output = R>,
    {
        let start_time = std::time::Instant::now();
        let start_memory = Self::get_memory_usage();

        let result = f.await;

        let end_time = std::time::Instant::now();
        let end_memory = Self::get_memory_usage();

        let duration = end_time - start_time;
        let memory_diff = end_memory.saturating_sub(start_memory);

        BenchmarkResult {
            name: name.to_string(),
            duration_ms: duration.as_millis() as u64,
            memory_usage_bytes: memory_diff,
            success: true,
            error_message: None,
            timestamp: SystemTime::now(),
        }
    }

    /// Compare performance between two functions
    pub fn compare_benchmarks<F1, F2, R1, R2>(
        name1: &str,
        f1: F1,
        name2: &str,
        f2: F2,
    ) -> BenchmarkComparison
    where
        F1: FnOnce() -> R1,
        F2: FnOnce() -> R2,
    {
        let benchmark1 = Self::benchmark(name1, f1);
        let benchmark2 = Self::benchmark(name2, f2);

        BenchmarkComparison {
            benchmark1,
            benchmark2,
            improvement_percent: Self::calculate_improvement(&benchmark1, &benchmark2),
            winner: if benchmark1.duration_ms < benchmark2.duration_ms {
                name1.to_string()
            } else {
                name2.to_string()
            },
        }
    }

    /// Run multiple iterations of a benchmark for statistical analysis
    pub fn run_statistical_benchmark<F, R>(
        name: &str,
        iterations: usize,
        f: F,
    ) -> StatisticalBenchmarkResult
    where
        F: Fn() -> R,
    {
        let mut results = Vec::with_capacity(iterations);

        for i in 0..iterations {
            let result = Self::benchmark(&format!("{}_{}", name, i), f);
            results.push(result);
        }

        StatisticalBenchmarkResult::from_results(name.to_string(), results)
    }

    /// Benchmark a test configuration
    pub async fn benchmark_test_config(
        config: &TestConfig,
        test_runner: &mut TestRunner,
    ) -> BenchmarkResult {
        let start_time = std::time::Instant::now();
        let start_memory = Self::get_memory_usage();

        let result = test_runner.run_test(config).await;

        let end_time = std::time::Instant::now();
        let end_memory = Self::get_memory_usage();

        let duration = end_time - start_time;
        let memory_diff = end_memory.saturating_sub(start_memory);

        BenchmarkResult {
            name: config.name.clone(),
            duration_ms: duration.as_millis() as u64,
            memory_usage_bytes: memory_diff,
            success: result.is_ok(),
            error_message: result.err(),
            timestamp: SystemTime::now(),
        }
    }

    /// Create a performance baseline from existing results
    pub fn create_baseline(results: &[BenchmarkResult]) -> PerformanceBaseline {
        if results.is_empty() {
            return PerformanceBaseline::default();
        }

        let avg_duration = results.iter()
            .map(|r| r.duration_ms)
            .sum::<u64>() as f64 / results.len() as f64;

        let avg_memory = results.iter()
            .map(|r| r.memory_usage_bytes)
            .sum::<u64>() as f64 / results.len() as f64;

        let max_duration = results.iter()
            .map(|r| r.duration_ms)
            .max()
            .unwrap_or(0);

        let max_memory = results.iter()
            .map(|r| r.memory_usage_bytes)
            .max()
            .unwrap_or(0);

        PerformanceBaseline {
            average_duration_ms: avg_duration,
            average_memory_bytes: avg_memory,
            max_duration_ms: max_duration,
            max_memory_bytes: max_memory,
            created_at: SystemTime::now(),
        }
    }

    /// Check if current performance is within acceptable range of baseline
    pub fn check_performance_regression(
        current: &BenchmarkResult,
        baseline: &PerformanceBaseline,
        tolerance_percent: f64,
    ) -> PerformanceCheckResult {
        let duration_ratio = current.duration_ms as f64 / baseline.average_duration_ms;
        let memory_ratio = current.memory_usage_bytes as f64 / baseline.average_memory_bytes;

        let duration_regression = duration_ratio > (1.0 + tolerance_percent / 100.0);
        let memory_regression = memory_ratio > (1.0 + tolerance_percent / 100.0);

        PerformanceCheckResult {
            duration_regression,
            memory_regression,
            duration_ratio,
            memory_ratio,
            tolerance_percent,
            within_tolerance: !duration_regression && !memory_regression,
        }
    }

    /// Get current memory usage (platform dependent)
    fn get_memory_usage() -> u64 {
        #[cfg(target_os = "linux")]
        {
            // On Linux, read from /proc/self/statm
            if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
                let parts: Vec<&str> = content.split_whitespace().collect();
                if parts.len() > 1 {
                    if let Ok(pages) = parts[1].parse::<u64>() {
                        return pages * 4096; // Assume 4KB pages
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, use mach API
            unsafe {
                use std::mem;
                let mut info = mem::zeroed::<libc::mach_task_basic_info>();
                let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;

                if libc::task_info(
                    libc::mach_task_self(),
                    libc::MACH_TASK_BASIC_INFO,
                    &mut info as *mut _ as libc::task_info_t,
                    &mut count,
                ) == libc::KERN_SUCCESS {
                    return info.resident_size as u64;
                }
            }
        }

        // Fallback: return 0 if we can't get memory usage
        0
    }

    /// Calculate improvement percentage between two benchmarks
    fn calculate_improvement(baseline: &BenchmarkResult, current: &BenchmarkResult) -> f64 {
        if baseline.duration_ms == 0 {
            return 0.0;
        }

        let improvement = baseline.duration_ms as f64 - current.duration_ms as f64;
        (improvement / baseline.duration_ms as f64) * 100.0
    }
}

/// Individual benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration_ms: u64,
    pub memory_usage_bytes: u64,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: SystemTime,
}

/// Comparison between two benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub benchmark1: BenchmarkResult,
    pub benchmark2: BenchmarkResult,
    pub improvement_percent: f64,
    pub winner: String,
}

/// Statistical analysis of multiple benchmark runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalBenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub average_duration_ms: f64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub median_duration_ms: f64,
    pub standard_deviation_ms: f64,
    pub average_memory_bytes: f64,
    pub success_rate: f64,
    pub timestamp: SystemTime,
}

impl StatisticalBenchmarkResult {
    pub fn from_results(name: String, results: Vec<BenchmarkResult>) -> Self {
        let iterations = results.len();

        let durations: Vec<u64> = results.iter().map(|r| r.duration_ms).collect();
        let memories: Vec<u64> = results.iter().map(|r| r.memory_usage_bytes).collect();
        let successes = results.iter().filter(|r| r.success).count();

        let avg_duration = durations.iter().sum::<u64>() as f64 / iterations as f64;
        let avg_memory = memories.iter().sum::<u64>() as f64 / iterations as f64;

        let min_duration = durations.iter().min().copied().unwrap_or(0);
        let max_duration = durations.iter().max().copied().unwrap_or(0);

        let median_duration = Self::calculate_median(&durations);
        let std_dev = Self::calculate_standard_deviation(&durations, avg_duration);

        let success_rate = successes as f64 / iterations as f64;

        Self {
            name,
            iterations,
            average_duration_ms: avg_duration,
            min_duration_ms: min_duration,
            max_duration_ms: max_duration,
            median_duration_ms: median_duration,
            standard_deviation_ms: std_dev,
            average_memory_bytes: avg_memory,
            success_rate,
            timestamp: SystemTime::now(),
        }
    }

    fn calculate_median(data: &[u64]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut sorted = data.to_vec();
        sorted.sort();

        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) as f64 / 2.0
        } else {
            sorted[mid] as f64
        }
    }

    fn calculate_standard_deviation(data: &[u64], mean: f64) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }

        let variance = data.iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / (data.len() - 1) as f64;

        variance.sqrt()
    }
}

/// Performance baseline for regression testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub average_duration_ms: f64,
    pub average_memory_bytes: f64,
    pub max_duration_ms: u64,
    pub max_memory_bytes: u64,
    pub created_at: SystemTime,
}

impl Default for PerformanceBaseline {
    fn default() -> Self {
        Self {
            average_duration_ms: 0.0,
            average_memory_bytes: 0.0,
            max_duration_ms: 0,
            max_memory_bytes: 0,
            created_at: SystemTime::now(),
        }
    }
}

/// Result of performance regression check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCheckResult {
    pub duration_regression: bool,
    pub memory_regression: bool,
    pub duration_ratio: f64,
    pub memory_ratio: f64,
    pub tolerance_percent: f64,
    pub within_tolerance: bool,
}