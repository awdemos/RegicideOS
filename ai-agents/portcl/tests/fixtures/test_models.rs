//! Test data models for comprehensive PortCL testing
//!
//! This module provides structured test result data models that mirror
//! the actual PortCL result structures but are designed for testing scenarios.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Test execution result data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub test_name: String,
    pub test_type: TestType,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub output: TestOutput,
    pub metrics: TestMetrics,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Test execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Timeout,
    Error,
}

/// Test type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestType {
    Unit,
    Integration,
    Performance,
    Contract,
    Property,
    Benchmark,
}

/// Test output data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub logs: Vec<TestLogEntry>,
    pub artifacts: Vec<TestArtifact>,
    pub coverage: Option<TestCoverage>,
}

/// Test log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestLogEntry {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub message: String,
    pub component: String,
    pub metadata: HashMap<String, String>,
}

/// Log level enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Test artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestArtifact {
    pub name: String,
    pub path: String,
    pub artifact_type: ArtifactType,
    pub size_bytes: u64,
    pub description: Option<String>,
}

/// Artifact type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArtifactType {
    Log,
    Report,
    Coverage,
    Benchmark,
    Trace,
    Custom(String),
}

/// Test coverage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCoverage {
    pub lines_covered: u32,
    pub lines_total: u32,
    pub functions_covered: u32,
    pub functions_total: u32,
    pub branches_covered: u32,
    pub branches_total: u32,
    pub percentage: f64,
    pub files: HashMap<String, FileCoverage>,
}

/// File-specific coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: String,
    pub lines_covered: u32,
    pub lines_total: u32,
    pub percentage: f64,
    pub uncovered_lines: Vec<u32>,
}

/// Test metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetrics {
    pub memory_peak_bytes: u64,
    pub cpu_time_ms: u64,
    pub allocations: u64,
    pub disk_io_bytes: u64,
    pub network_io_bytes: u64,
    pub custom_metrics: HashMap<String, f64>,
}

impl Default for TestResult {
    fn default() -> Self {
        let now = SystemTime::now();
        Self {
            test_id: uuid::Uuid::new_v4().to_string(),
            test_name: "unnamed_test".to_string(),
            test_type: TestType::Unit,
            status: TestStatus::Passed,
            duration_ms: 0,
            start_time: now,
            end_time: now,
            output: TestOutput::default(),
            metrics: TestMetrics::default(),
            dependencies: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

impl Default for TestOutput {
    fn default() -> Self {
        Self {
            stdout: None,
            stderr: None,
            logs: Vec::new(),
            artifacts: Vec::new(),
            coverage: None,
        }
    }
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self {
            memory_peak_bytes: 0,
            cpu_time_ms: 0,
            allocations: 0,
            disk_io_bytes: 0,
            network_io_bytes: 0,
            custom_metrics: HashMap::new(),
        }
    }
}

impl TestResult {
    /// Create a new test result with basic information
    pub fn new(test_name: String, test_type: TestType) -> Self {
        let now = SystemTime::now();
        Self {
            test_id: uuid::Uuid::new_v4().to_string(),
            test_name,
            test_type,
            status: TestStatus::Passed,
            duration_ms: 0,
            start_time: now,
            end_time: now,
            output: TestOutput::default(),
            metrics: TestMetrics::default(),
            dependencies: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Mark test as started and record start time
    pub fn start(&mut self) {
        self.start_time = SystemTime::now();
        self.status = TestStatus::Skipped; // Running state
    }

    /// Mark test as completed with final status
    pub fn complete(&mut self, status: TestStatus) {
        self.end_time = SystemTime::now();
        self.status = status;
        self.duration_ms = self.end_time
            .duration_since(self.start_time)
            .unwrap_or(Duration::from_millis(0))
            .as_millis() as u64;
    }

    /// Add a log entry to the test output
    pub fn add_log(&mut self, level: LogLevel, message: String, component: String) {
        self.output.logs.push(TestLogEntry {
            timestamp: SystemTime::now(),
            level,
            message,
            component,
            metadata: HashMap::new(),
        });
    }

    /// Set stdout output
    pub fn set_stdout(&mut self, stdout: String) {
        self.output.stdout = Some(stdout);
    }

    /// Set stderr output
    pub fn set_stderr(&mut self, stderr: String) {
        self.output.stderr = Some(stderr);
    }

    /// Add a test artifact
    pub fn add_artifact(&mut self, name: String, path: String, artifact_type: ArtifactType) {
        // In a real implementation, we'd get the file size from the filesystem
        let size_bytes = 0; // Placeholder
        self.output.artifacts.push(TestArtifact {
            name,
            path,
            artifact_type,
            size_bytes,
            description: None,
        });
    }

    /// Add a custom metric
    pub fn add_metric(&mut self, name: String, value: f64) {
        self.metrics.custom_metrics.insert(name, value);
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dependency: String) {
        self.dependencies.push(dependency);
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Check if the test passed
    pub fn is_passed(&self) -> bool {
        self.status == TestStatus::Passed
    }

    /// Check if the test failed
    pub fn is_failed(&self) -> bool {
        self.status == TestStatus::Failed
    }

    /// Check if the test was skipped
    pub fn is_skipped(&self) -> bool {
        self.status == TestStatus::Skipped
    }

    /// Get the test duration as a Duration
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }
}

/// Builder pattern for creating test results
pub struct TestResultBuilder {
    result: TestResult,
}

impl TestResultBuilder {
    pub fn new(test_name: String, test_type: TestType) -> Self {
        Self {
            result: TestResult::new(test_name, test_type),
        }
    }

    pub fn with_status(mut self, status: TestStatus) -> Self {
        self.result.status = status;
        self
    }

    pub fn with_dependency(mut self, dependency: String) -> Self {
        self.result.add_dependency(dependency);
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.result.add_tag(tag);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.result.add_metadata(key, value);
        self
    }

    pub fn build(self) -> TestResult {
        self.result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_result_creation() {
        let result = TestResult::new("test_example".to_string(), TestType::Unit);
        assert_eq!(result.test_name, "test_example");
        assert_eq!(result.test_type, TestType::Unit);
        assert_eq!(result.status, TestStatus::Passed);
    }

    #[test]
    fn test_test_result_builder() {
        let result = TestResultBuilder::new("builder_test".to_string(), TestType::Integration)
            .with_status(TestStatus::Failed)
            .with_dependency("dependency_1".to_string())
            .with_tag("integration".to_string())
            .with_metadata("version".to_string(), "1.0".to_string())
            .build();

        assert_eq!(result.test_name, "builder_test");
        assert_eq!(result.test_type, TestType::Integration);
        assert_eq!(result.status, TestStatus::Failed);
        assert!(result.dependencies.contains(&"dependency_1".to_string()));
        assert!(result.tags.contains(&"integration".to_string()));
        assert_eq!(result.metadata.get("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_test_result_completion() {
        let mut result = TestResult::new("completion_test".to_string(), TestType::Unit);
        result.start();

        // Simulate some work
        std::thread::sleep(std::time::Duration::from_millis(10));

        result.complete(TestStatus::Passed);

        assert_eq!(result.status, TestStatus::Passed);
        assert!(result.duration_ms > 0);
        assert!(result.end_time > result.start_time);
        assert!(result.is_passed());
    }

    #[test]
    fn test_log_entry_creation() {
        let mut result = TestResult::new("log_test".to_string(), TestType::Unit);
        result.add_log(LogLevel::Error, "Test error".to_string(), "test_component".to_string());

        assert_eq!(result.output.logs.len(), 1);
        let log = &result.output.logs[0];
        assert_eq!(log.level, LogLevel::Error);
        assert_eq!(log.message, "Test error");
        assert_eq!(log.component, "test_component");
    }
}

/// Test configuration data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub test_suite_name: String,
    pub test_types: Vec<TestType>,
    pub execution_config: TestExecutionConfig,
    pub reporting_config: TestReportingConfig,
    pub coverage_config: TestCoverageConfig,
    pub performance_config: TestPerformanceConfig,
    pub filters: TestFilters,
    pub environment: TestEnvironment,
}

/// Test execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExecutionConfig {
    pub max_concurrent_tests: usize,
    pub test_timeout_seconds: u64,
    pub retry_failed_tests: bool,
    pub max_retries: u32,
    pub fail_fast: bool,
    pub shuffle_tests: bool,
    pub test_order: TestOrder,
    pub include_ignored: bool,
}

/// Test reporting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReportingConfig {
    pub output_format: OutputFormat,
    pub report_path: String,
    pub include_stdout: bool,
    pub include_stderr: bool,
    pub include_logs: bool,
    pub include_metrics: bool,
    pub generate_html_report: bool,
    pub generate_json_report: bool,
    pub generate_junit_xml: bool,
}

/// Test coverage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCoverageConfig {
    pub enabled: bool,
    pub coverage_type: CoverageType,
    pub target_percentage: f64,
    pub fail_below_target: bool,
    pub output_path: String,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

/// Test performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPerformanceConfig {
    pub enabled: bool,
    pub benchmark_iterations: u32,
    pub warmup_iterations: u32,
    pub sample_size: u32,
    pub confidence_level: f64,
    pub max_execution_time_seconds: u64,
    pub memory_limit_mb: u64,
    pub cpu_limit_percent: f64,
}

/// Test filters configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFilters {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub include_tags: Vec<String>,
    pub exclude_tags: Vec<String>,
    pub include_test_types: Vec<TestType>,
    pub exclude_test_types: Vec<TestType>,
    pub min_severity: LogLevel,
}

/// Test environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEnvironment {
    pub variables: HashMap<String, String>,
    pub temp_dir: String,
    pub work_dir: String,
    pub log_dir: String,
    pub artifact_dir: String,
    pub cleanup_after: bool,
    pub preserve_temp_dirs: bool,
}

/// Test order configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestOrder {
    Sequential,
    Random,
    Parallel,
    DependencyOrder,
}

/// Output format configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputFormat {
    Pretty,
    Terse,
    Json,
    JUnit,
    Html,
}

/// Coverage type configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoverageType {
    Line,
    Branch,
    Function,
    Condition,
    Full,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_suite_name: "PortCL Test Suite".to_string(),
            test_types: vec![TestType::Unit, TestType::Integration],
            execution_config: TestExecutionConfig::default(),
            reporting_config: TestReportingConfig::default(),
            coverage_config: TestCoverageConfig::default(),
            performance_config: TestPerformanceConfig::default(),
            filters: TestFilters::default(),
            environment: TestEnvironment::default(),
        }
    }
}

impl Default for TestExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tests: 4,
            test_timeout_seconds: 300,
            retry_failed_tests: true,
            max_retries: 3,
            fail_fast: false,
            shuffle_tests: false,
            test_order: TestOrder::Sequential,
            include_ignored: false,
        }
    }
}

impl Default for TestReportingConfig {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Pretty,
            report_path: "./test-reports".to_string(),
            include_stdout: true,
            include_stderr: true,
            include_logs: true,
            include_metrics: true,
            generate_html_report: true,
            generate_json_report: true,
            generate_junit_xml: true,
        }
    }
}

impl Default for TestCoverageConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            coverage_type: CoverageType::Full,
            target_percentage: 80.0,
            fail_below_target: true,
            output_path: "./coverage".to_string(),
            include_patterns: vec![
                "src/**/*.rs".to_string(),
                "tests/**/*.rs".to_string(),
            ],
            exclude_patterns: vec![
                "tests/fixtures/**/*".to_string(),
                "target/**/*".to_string(),
            ],
        }
    }
}

impl Default for TestPerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            benchmark_iterations: 100,
            warmup_iterations: 10,
            sample_size: 50,
            confidence_level: 0.95,
            max_execution_time_seconds: 600,
            memory_limit_mb: 1024,
            cpu_limit_percent: 80.0,
        }
    }
}

impl Default for TestFilters {
    fn default() -> Self {
        Self {
            include_patterns: vec!["**/*test*.rs".to_string()],
            exclude_patterns: vec![],
            include_tags: vec![],
            exclude_tags: vec!["slow".to_string(), "integration".to_string()],
            include_test_types: vec![TestType::Unit, TestType::Integration],
            exclude_test_types: vec![],
            min_severity: LogLevel::Info,
        }
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self {
            variables: {
                let mut vars = HashMap::new();
                vars.insert("RUST_LOG".to_string(), "info".to_string());
                vars.insert("RUST_BACKTRACE".to_string(), "1".to_string());
                vars
            },
            temp_dir: "/tmp/portcl-tests".to_string(),
            work_dir: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
            log_dir: "./test-logs".to_string(),
            artifact_dir: "./test-artifacts".to_string(),
            cleanup_after: true,
            preserve_temp_dirs: false,
        }
    }
}

impl TestConfig {
    /// Create a new test configuration with minimal settings
    pub fn new(test_suite_name: String) -> Self {
        Self {
            test_suite_name,
            ..Default::default()
        }
    }

    /// Enable specific test types
    pub fn with_test_types(mut self, test_types: Vec<TestType>) -> Self {
        self.test_types = test_types;
        self
    }

    /// Set coverage target percentage
    pub fn with_coverage_target(mut self, target_percentage: f64) -> Self {
        self.coverage_config.target_percentage = target_percentage;
        self
    }

    /// Enable performance testing
    pub fn with_performance_testing(mut self, enabled: bool) -> Self {
        self.performance_config.enabled = enabled;
        self
    }

    /// Set test timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.execution_config.test_timeout_seconds = timeout_seconds;
        self
    }

    /// Add environment variable
    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.environment.variables.insert(key, value);
        self
    }

    /// Add include pattern
    pub fn with_include_pattern(mut self, pattern: String) -> Self {
        self.filters.include_patterns.push(pattern);
        self
    }

    /// Set output format
    pub fn with_output_format(mut self, format: OutputFormat) -> Self {
        self.reporting_config.output_format = format;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.test_suite_name.is_empty() {
            return Err("Test suite name cannot be empty".to_string());
        }

        if self.execution_config.max_concurrent_tests == 0 {
            return Err("Max concurrent tests must be greater than 0".to_string());
        }

        if self.execution_config.test_timeout_seconds == 0 {
            return Err("Test timeout must be greater than 0".to_string());
        }

        if !(0.0..=100.0).contains(&self.coverage_config.target_percentage) {
            return Err("Coverage target percentage must be between 0 and 100".to_string());
        }

        if self.performance_config.benchmark_iterations == 0 {
            return Err("Benchmark iterations must be greater than 0".to_string());
        }

        if !(0.0..=1.0).contains(&self.performance_config.confidence_level) {
            return Err("Confidence level must be between 0 and 1".to_string());
        }

        Ok(())
    }

    /// Check if a test should be included based on filters
    pub fn should_include_test(&self, test_name: &str, test_type: &TestType, tags: &[String]) -> bool {
        // Check test type filters
        if !self.filters.include_test_types.is_empty() && !self.filters.include_test_types.contains(test_type) {
            return false;
        }

        if self.filters.exclude_test_types.contains(test_type) {
            return false;
        }

        // Check pattern filters
        let include_match = self.filters.include_patterns.is_empty() ||
            self.filters.include_patterns.iter().any(|pattern| {
                glob::Pattern::new(pattern)
                    .map(|p| p.matches(test_name))
                    .unwrap_or(false)
            });

        let exclude_match = self.filters.exclude_patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(test_name))
                .unwrap_or(false)
        });

        // Check tag filters
        let tag_match = tags.iter().any(|tag| {
            (self.filters.include_tags.is_empty() || self.filters.include_tags.contains(tag)) &&
            !self.filters.exclude_tags.contains(tag)
        });

        include_match && !exclude_match && tag_match
    }
}

/// Builder pattern for creating test configurations
pub struct TestConfigBuilder {
    config: TestConfig,
}

impl TestConfigBuilder {
    pub fn new(test_suite_name: String) -> Self {
        Self {
            config: TestConfig::new(test_suite_name),
        }
    }

    pub fn with_test_types(mut self, test_types: Vec<TestType>) -> Self {
        self.config.test_types = test_types;
        self
    }

    pub fn with_coverage_target(mut self, target_percentage: f64) -> Self {
        self.config.coverage_config.target_percentage = target_percentage;
        self
    }

    pub fn with_performance_testing(mut self, enabled: bool) -> Self {
        self.config.performance_config.enabled = enabled;
        self
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.execution_config.test_timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.config.environment.variables.insert(key, value);
        self
    }

    pub fn build(self) -> Result<TestConfig, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_test_config_creation() {
        let config = TestConfig::new("Test Suite".to_string());
        assert_eq!(config.test_suite_name, "Test Suite");
        assert!(config.test_types.contains(&TestType::Unit));
        assert!(config.test_types.contains(&TestType::Integration));
    }

    #[test]
    fn test_test_config_builder() {
        let config = TestConfigBuilder::new("Builder Test".to_string())
            .with_test_types(vec![TestType::Unit])
            .with_coverage_target(90.0)
            .with_performance_testing(true)
            .with_timeout(600)
            .with_env_var("TEST_VAR".to_string(), "test_value".to_string())
            .build()
            .unwrap();

        assert_eq!(config.test_suite_name, "Builder Test");
        assert_eq!(config.test_types, vec![TestType::Unit]);
        assert_eq!(config.coverage_config.target_percentage, 90.0);
        assert!(config.performance_config.enabled);
        assert_eq!(config.execution_config.test_timeout_seconds, 600);
        assert_eq!(config.environment.variables.get("TEST_VAR"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_config_validation() {
        let mut config = TestConfig::default();
        assert!(config.validate().is_ok());

        config.test_suite_name = "".to_string();
        assert!(config.validate().is_err());

        config.test_suite_name = "Valid".to_string();
        config.execution_config.max_concurrent_tests = 0;
        assert!(config.validate().is_err());
    }
}