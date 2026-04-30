//! Test Configuration Loader
//!
//! This module provides utilities for loading and managing test configurations
//! from TOML files with environment variable support and configuration merging.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::env;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use anyhow::Result;
use config::{Config, ConfigError, Environment, File};
use tracing::{info, warn, error};

/// Test configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub test_environment: TestEnvironment,
    pub execution: ExecutionConfig,
    pub logging: LoggingConfig,
    pub coverage: CoverageConfig,
    pub performance: PerformanceConfig,
    pub output: OutputConfig,
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
    pub test_type: Option<TestTypeConfig>,
    pub test_template: Option<TestTemplateConfig>,
    pub filtering: Option<FilteringConfig>,
    pub isolation: Option<IsolationConfig>,
    pub database: Option<DatabaseConfig>,
    pub fixtures: Option<FixturesConfig>,
    pub mocks: Option<MocksConfig>,
    pub assertions: Option<AssertionsConfig>,
    pub benchmarks: Option<BenchmarksConfig>,
    pub network: Option<NetworkConfig>,
    pub property_testing: Option<PropertyTestingConfig>,
    pub generation: Option<GenerationConfig>,
    pub shrinking: Option<ShrinkingConfig>,
    pub load_testing: Option<LoadTestingConfig>,
    pub thresholds: Option<ThresholdsConfig>,
    pub baseline: Option<BaselineConfig>,
    pub notifications: Option<NotificationsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEnvironment {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub timeout_seconds: u64,
    pub max_parallel_tests: usize,
    pub retry_count: u32,
    pub fail_fast: bool,
    pub randomize_order: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub target: String,
    pub show_skipped: bool,
    pub show_output: bool,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageConfig {
    pub enabled: bool,
    pub report_formats: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub threshold_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub enabled: bool,
    pub sample_size: u32,
    pub warmup_runs: u32,
    pub max_execution_time_ms: u64,
    pub memory_limit_mb: u64,
    pub cool_down_runs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub directory: String,
    pub formats: Vec<String>,
    pub preserve_artifacts: bool,
    pub cleanup_on_success: bool,
    pub generate_report: Option<bool>,
    pub generate_dashboard: Option<bool>,
    pub include_raw_data: Option<bool>,
    pub statistical_analysis: Option<bool>,
    pub comparison_baseline: Option<bool>,
    pub historical_tracking: Option<bool>,
    pub upload_results: Option<bool>,
    pub artifacts_path: Option<String>,
    pub generate_diff_report: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,
    pub profiling_enabled: bool,
    pub metrics_collection_rate_ms: Option<u64>,
    pub system_metrics: Option<bool>,
    pub application_metrics: Option<bool>,
    pub database_metrics: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub api_key_required: bool,
    pub rate_limiting_enabled: bool,
    pub requests_per_minute: Option<u32>,
    pub timeout_tolerance_ms: u64,
    pub ssl_verification_enabled: Option<bool>,
    pub api_key_env_var: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTypeConfig {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTemplateConfig {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteringConfig {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub test_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationConfig {
    pub enabled: bool,
    pub temp_directory: String,
    pub cleanup_after_each_test: Option<bool>,
    pub cleanup_after_suite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub migrations_enabled: bool,
    pub pool_size: u32,
    pub setup_required: Option<bool>,
    pub seed_data_enabled: Option<bool>,
    pub cleanup_after_tests: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixturesConfig {
    pub auto_generate: bool,
    pub cleanup_after_tests: bool,
    pub data_directory: String,
    pub minimal: Option<bool>,
    pub realistic: Option<bool>,
    pub auto_cleanup: Option<bool>,
    pub setup_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MocksConfig {
    pub auto_generate: bool,
    pub strict_mocking: bool,
    pub mock_verification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionsConfig {
    pub detailed_failures: bool,
    pub custom_assertions_enabled: bool,
    pub diff_on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarksConfig {
    pub iterations: u32,
    pub measure_memory: bool,
    pub measure_cpu: bool,
    pub measure_disk_io: bool,
    pub measure_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub mock_external_services: bool,
    pub timeout_ms: u64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyTestingConfig {
    pub test_cases: u32,
    pub max_shrink_steps: u32,
    pub size_range: [u32; 2],
    pub max_discard_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub strategies_enabled: bool,
    pub custom_generators: bool,
    pub arbitrary_impls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShrinkingConfig {
    pub enabled: bool,
    pub minimal_counterexamples: bool,
    pub report_shrink_path: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestingConfig {
    pub ramp_up_duration_seconds: u64,
    pub peak_duration_seconds: u64,
    pub ramp_down_duration_seconds: u64,
    pub users_per_second: u32,
    pub max_users: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdsConfig {
    pub error_rate_percentage: f64,
    pub response_time_ms: u64,
    pub memory_usage_mb: u64,
    pub cpu_usage_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineConfig {
    pub enabled: bool,
    pub baseline_file: String,
    pub compare_threshold_percentage: f64,
    pub require_baseline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub enabled: bool,
    pub on_failure: bool,
    pub on_success: bool,
    pub webhook_url_env_var: Option<String>,
}

/// Configuration loader error
#[derive(Debug, Error)]
pub enum ConfigLoaderError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Invalid configuration format: {0}")]
    InvalidFormat(String),
    #[error("Configuration merge error: {0}")]
    MergeError(String),
    #[error("Environment variable error: {0}")]
    EnvironmentError(String),
}

/// Configuration loader
pub struct ConfigLoader {
    config_dir: PathBuf,
    env_prefix: String,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new(config_dir: impl AsRef<Path>) -> Self {
        Self {
            config_dir: config_dir.as_ref().to_path_buf(),
            env_prefix: "TEST".to_string(),
        }
    }

    /// Load configuration from a file
    pub fn load(&self, config_path: impl AsRef<Path>) -> Result<TestConfig> {
        let config_path = config_path.as_ref();

        if !config_path.exists() {
            return Err(ConfigLoaderError::FileNotFound(config_path.to_path_buf()).into());
        }

        let mut config_builder = Config::builder()
            .add_source(File::from(config_path))
            .add_source(Environment::with_prefix(&self.env_prefix).separator("_"));

        // Try to load local overrides
        let local_config_path = self.config_dir.join("local.toml");
        if local_config_path.exists() {
            info!("Loading local configuration overrides");
            config_builder = config_builder.add_source(File::from(local_config_path));
        }

        // Apply environment variable overrides
        config_builder = self.apply_env_overrides(config_builder);

        let config = config_builder.build()?;

        // Convert to our test config structure
        let test_config: TestConfig = config.try_deserialize()?;

        info!("Loaded configuration from: {}", config_path.display());
        Ok(test_config)
    }

    /// Load and merge multiple configuration files
    pub fn load_merged(&self, config_paths: &[impl AsRef<Path>]) -> Result<TestConfig> {
        if config_paths.is_empty() {
            return Err(anyhow::anyhow!("No configuration files provided"));
        }

        let mut merged_config = Config::default();

        for path in config_paths {
            let path = path.as_ref();
            if !path.exists() {
                warn!("Configuration file not found: {}", path.display());
                continue;
            }

            info!("Loading configuration from: {}", path.display());
            let file_config = Config::builder()
                .add_source(File::from(path))
                .build()?;

            merged_config = merged_config.merge(file_config)?;
        }

        // Apply environment variable overrides
        merged_config = self.apply_env_overrides(merged_config);

        // Try to load local overrides
        let local_config_path = self.config_dir.join("local.toml");
        if local_config_path.exists() {
            info!("Loading local configuration overrides");
            let local_config = Config::builder()
                .add_source(File::from(local_config_path))
                .build()?;
            merged_config = merged_config.merge(local_config)?;
        }

        let test_config: TestConfig = merged_config.try_deserialize()?;

        info!("Loaded merged configuration from {} files", config_paths.len());
        Ok(test_config)
    }

    /// Load configuration for a specific environment
    pub fn load_environment(&self, environment: &str) -> Result<TestConfig> {
        let config_path = self.config_dir
            .join("environments")
            .join(format!("{}.toml", environment));

        self.load(config_path)
    }

    /// Load configuration for a specific test type
    pub fn load_test_type(&self, test_type: &str) -> Result<TestConfig> {
        let config_path = self.config_dir
            .join("test_types")
            .join(format!("{}.toml", test_type));

        self.load(config_path)
    }

    /// Load configuration template
    pub fn load_template(&self, template: &str) -> Result<TestConfig> {
        let config_path = self.config_dir
            .join("templates")
            .join(format!("{}.toml", template));

        self.load(config_path)
    }

    /// Get available environments
    pub fn available_environments(&self) -> Result<Vec<String>> {
        let env_dir = self.config_dir.join("environments");
        self.list_config_files(&env_dir)
    }

    /// Get available test types
    pub fn available_test_types(&self) -> Result<Vec<String>> {
        let test_types_dir = self.config_dir.join("test_types");
        self.list_config_files(&test_types_dir)
    }

    /// Get available templates
    pub fn available_templates(&self) -> Result<Vec<String>> {
        let templates_dir = self.config_dir.join("templates");
        self.list_config_files(&templates_dir)
    }

    /// List configuration files in a directory
    fn list_config_files(&self, dir: &Path) -> Result<Vec<String>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    files.push(stem.to_string());
                }
            }
        }

        files.sort();
        Ok(files)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&self, mut config: ConfigBuilder) -> ConfigBuilder {
        // Common overrides
        if let Ok(env_name) = env::var("TEST_ENVIRONMENT") {
            config = config.set_override("test_environment.name", env_name).unwrap();
        }

        if let Ok(output_dir) = env::var("TEST_OUTPUT_DIR") {
            config = config.set_override("output.directory", output_dir).unwrap();
        }

        if let Ok(log_level) = env::var("TEST_LOG_LEVEL") {
            config = config.set_override("logging.level", log_level).unwrap();
        }

        if let Ok(api_key) = env::var("TEST_API_KEY") {
            config = config.set_override("security.api_key_required", true).unwrap();
        }

        if let Ok(timeout) = env::var("TEST_TIMEOUT_SECONDS") {
            if let Ok(timeout_secs) = timeout.parse::<u64>() {
                config = config.set_override("execution.timeout_seconds", timeout_secs).unwrap();
            }
        }

        if let Ok(parallel) = env::var("TEST_PARALLEL_TESTS") {
            if let Ok(parallel_count) = parallel.parse::<usize>() {
                config = config.set_override("execution.max_parallel_tests", parallel_count).unwrap();
            }
        }

        config
    }

    /// Validate configuration
    pub fn validate_config(&self, config: &TestConfig) -> Result<Vec<String>> {
        let mut issues = Vec::new();

        // Validate execution settings
        if config.execution.timeout_seconds == 0 {
            issues.push("Timeout seconds must be greater than 0".to_string());
        }

        if config.execution.max_parallel_tests == 0 {
            issues.push("Max parallel tests must be greater than 0".to_string());
        }

        // Validate coverage settings
        if config.coverage.enabled {
            if config.coverage.threshold_percentage < 0.0 || config.coverage.threshold_percentage > 100.0 {
                issues.push("Coverage threshold percentage must be between 0 and 100".to_string());
            }
        }

        // Validate performance settings
        if config.performance.enabled {
            if config.performance.sample_size == 0 {
                issues.push("Performance sample size must be greater than 0".to_string());
            }
        }

        // Validate output directory
        if config.output.directory.is_empty() {
            issues.push("Output directory cannot be empty".to_string());
        }

        Ok(issues)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new("tests/config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_loader_creation() {
        let loader = ConfigLoader::new("tests/config");
        assert_eq!(loader.env_prefix, "TEST");
    }

    #[test]
    fn test_default_config_loader() {
        let loader = ConfigLoader::default();
        assert_eq!(loader.config_dir, PathBuf::from("tests/config"));
    }

    #[test]
    fn test_config_validation() {
        let config = TestConfig {
            test_environment: TestEnvironment {
                name: "test".to_string(),
                description: "Test environment".to_string(),
            },
            execution: ExecutionConfig {
                timeout_seconds: 30,
                max_parallel_tests: 4,
                retry_count: 0,
                fail_fast: false,
                randomize_order: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                target: "console".to_string(),
                show_skipped: false,
                show_output: false,
                file_path: None,
            },
            coverage: CoverageConfig {
                enabled: true,
                report_formats: vec!["json".to_string()],
                exclude_paths: vec![],
                threshold_percentage: 80.0,
            },
            performance: PerformanceConfig {
                enabled: true,
                sample_size: 10,
                warmup_runs: 3,
                max_execution_time_ms: 5000,
                memory_limit_mb: 512,
                cool_down_runs: None,
            },
            output: OutputConfig {
                directory: "target/test-results".to_string(),
                formats: vec!["json".to_string()],
                preserve_artifacts: true,
                cleanup_on_success: false,
                generate_report: None,
                generate_dashboard: None,
                include_raw_data: None,
                statistical_analysis: None,
                comparison_baseline: None,
                historical_tracking: None,
                upload_results: None,
                artifacts_path: None,
                generate_diff_report: None,
            },
            monitoring: MonitoringConfig {
                enabled: true,
                metrics_enabled: true,
                tracing_enabled: true,
                profiling_enabled: true,
                metrics_collection_rate_ms: None,
                system_metrics: None,
                application_metrics: None,
                database_metrics: None,
            },
            security: SecurityConfig {
                api_key_required: false,
                rate_limiting_enabled: false,
                requests_per_minute: None,
                timeout_tolerance_ms: 5000,
                ssl_verification_enabled: None,
                api_key_env_var: None,
            },
            test_type: None,
            test_template: None,
            filtering: None,
            isolation: None,
            database: None,
            fixtures: None,
            mocks: None,
            assertions: None,
            benchmarks: None,
            network: None,
            property_testing: None,
            generation: None,
            shrinking: None,
            load_testing: None,
            thresholds: None,
            baseline: None,
            notifications: None,
        };

        let loader = ConfigLoader::default();
        let issues = loader.validate_config(&config).unwrap();
        assert!(issues.is_empty());
    }
}