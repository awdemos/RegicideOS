//! Configuration Validator
//!
//! Provides utilities for validating test configurations and detecting issues.

use std::path::Path;
use crate::loader::{TestConfig, ConfigLoader, ConfigLoaderError};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub level: ValidationLevel,
    pub category: ValidationCategory,
    pub message: String,
    pub path: String,
    pub suggestion: Option<String>,
}

/// Validation level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationLevel {
    Error,
    Warning,
    Info,
}

/// Validation category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationCategory {
    Structure,
    Value,
    Compatibility,
    Performance,
    Security,
    BestPractice,
}

/// Configuration validator
pub struct ConfigValidator {
    loader: ConfigLoader,
}

impl ConfigValidator {
    /// Create a new configuration validator
    pub fn new(loader: ConfigLoader) -> Self {
        Self { loader }
    }

    /// Validate a configuration file
    pub fn validate_file(&self, config_path: impl AsRef<Path>) -> Result<ValidationResult> {
        let config_path = config_path.as_ref();
        let config = self.loader.load(config_path)?;
        self.validate_config(&config, Some(config_path))
    }

    /// Validate a configuration object
    pub fn validate_config(&self, config: &TestConfig, path: Option<&Path>) -> Result<ValidationResult> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Validate basic structure
        self.validate_structure(config, &mut issues, path);

        // Validate values
        self.validate_values(config, &mut issues, &mut warnings, path);

        // Validate compatibility
        self.validate_compatibility(config, &mut issues, &mut warnings, path);

        // Validate performance implications
        self.validate_performance(config, &mut warnings, path);

        // Validate security settings
        self.validate_security(config, &mut issues, &mut warnings, path);

        // Validate best practices
        self.validate_best_practices(config, &mut suggestions, path);

        let is_valid = !issues.iter().any(|issue| matches!(issue.level, ValidationLevel::Error));

        Ok(ValidationResult {
            is_valid,
            issues,
            warnings,
            suggestions,
        })
    }

    /// Validate configuration structure
    fn validate_structure(&self, config: &TestConfig, issues: &mut Vec<ValidationIssue>, path: Option<&Path>) {
        let path_str = path.map_or("config".to_string(), |p| p.display().to_string());

        // Check required fields
        if config.test_environment.name.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                category: ValidationCategory::Structure,
                message: "Environment name cannot be empty".to_string(),
                path: path_str.clone(),
                suggestion: Some("Set test_environment.name to a non-empty value".to_string()),
            });
        }

        if config.execution.timeout_seconds == 0 {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                category: ValidationCategory::Structure,
                message: "Timeout seconds must be greater than 0".to_string(),
                path: path_str.clone(),
                suggestion: Some("Set execution.timeout_seconds to a value > 0".to_string()),
            });
        }

        if config.execution.max_parallel_tests == 0 {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                category: ValidationCategory::Structure,
                message: "Max parallel tests must be greater than 0".to_string(),
                path: path_str.clone(),
                suggestion: Some("Set execution.max_parallel_tests to a value > 0".to_string()),
            });
        }

        if config.output.directory.is_empty() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                category: ValidationCategory::Structure,
                message: "Output directory cannot be empty".to_string(),
                path: path_str.clone(),
                suggestion: Some("Set output.directory to a valid path".to_string()),
            });
        }
    }

    /// Validate configuration values
    fn validate_values(&self, config: &TestConfig, issues: &mut Vec<ValidationIssue>, warnings: &mut Vec<String>, path: Option<&Path>) {
        let path_str = path.map_or("config".to_string(), |p| p.display().to_string());

        // Coverage threshold validation
        if config.coverage.enabled {
            if config.coverage.threshold_percentage < 0.0 || config.coverage.threshold_percentage > 100.0 {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    category: ValidationCategory::Value,
                    message: format!("Coverage threshold ({}) must be between 0 and 100", config.coverage.threshold_percentage),
                    path: path_str.clone(),
                    suggestion: Some("Set coverage.threshold_percentage to a value between 0.0 and 100.0".to_string()),
                });
            }

            if config.coverage.threshold_percentage > 95.0 {
                warnings.push(format!("Coverage threshold ({:.1}%) is very high and may be difficult to maintain", config.coverage.threshold_percentage));
            }
        }

        // Performance validation
        if config.performance.enabled {
            if config.performance.sample_size < 5 {
                warnings.push("Performance sample size is low, consider using at least 10 samples for reliable results".to_string());
            }

            if config.performance.sample_size > 1000 {
                warnings.push("Performance sample size is very high, this may significantly increase test duration".to_string());
            }
        }

        // Memory validation
        if config.performance.memory_limit_mb > 8192 {
            warnings.push("Memory limit is very high (>8GB), ensure system has sufficient resources".to_string());
        }

        // Timeout validation
        if config.execution.timeout_seconds > 3600 {
            warnings.push("Test timeout is very long (>1 hour), consider if this is necessary".to_string());
        }

        // Parallel execution validation
        let num_cpus = num_cpus::get();
        if config.execution.max_parallel_tests > num_cpus * 2 {
            warnings.push(format!("Parallel test count ({}) is much higher than CPU count ({}), this may lead to contention", config.execution.max_parallel_tests, num_cpus));
        }
    }

    /// Validate configuration compatibility
    fn validate_compatibility(&self, config: &TestConfig, issues: &mut Vec<ValidationIssue>, warnings: &mut Vec<String>, path: Option<&Path>) {
        let path_str = path.map_or("config".to_string(), |p| p.display().to_string());

        // Environment compatibility
        if let Some(env_name) = &config.test_environment.name.as_deref() {
            match env_name {
                "production" => {
                    if config.logging.level == "debug" {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Warning,
                            category: ValidationCategory::Compatibility,
                            message: "Debug logging in production environment".to_string(),
                            path: path_str.clone(),
                            suggestion: Some("Consider using 'info' or 'warn' log level in production".to_string()),
                        });
                    }

                    if config.coverage.enabled {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Warning,
                            category: ValidationCategory::Compatibility,
                            message: "Coverage collection enabled in production environment".to_string(),
                            path: path_str.clone(),
                            suggestion: Some("Consider disabling coverage in production for performance".to_string()),
                        });
                    }
                }
                "development" => {
                    if config.execution.fail_fast && config.execution.max_parallel_tests > 1 {
                        warnings.push("Fail-fast with parallel tests may make debugging difficult in development".to_string());
                    }
                }
                _ => {}
            }
        }

        // Test type compatibility
        if let Some(test_type) = &config.test_type {
            match test_type.name.as_str() {
                "unit" => {
                    if config.execution.timeout_seconds > 60 {
                        warnings.push("Unit tests typically should complete quickly, consider reducing timeout".to_string());
                    }
                }
                "integration" => {
                    if config.execution.max_parallel_tests > 8 {
                        warnings.push("High parallelism in integration tests may cause resource contention".to_string());
                    }
                }
                "performance" => {
                    if config.execution.max_parallel_tests > 1 {
                        warnings.push("Performance tests should typically run sequentially to avoid interference".to_string());
                    }
                }
                _ => {}
            }
        }

        // Output format compatibility
        if config.output.formats.contains(&"syslog".to_string()) {
            if cfg!(target_os = "windows") {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    category: ValidationCategory::Compatibility,
                    message: "Syslog output may not work properly on Windows".to_string(),
                    path: path_str.clone(),
                    suggestion: Some("Consider using 'file' or 'console' output on Windows".to_string()),
                });
            }
        }
    }

    /// Validate performance implications
    fn validate_performance(&self, config: &TestConfig, warnings: &mut Vec<String>, path: Option<&Path>) {
        // Memory usage estimation
        let estimated_memory_mb = config.performance.memory_limit_mb * config.execution.max_parallel_tests as u64;
        if estimated_memory_mb > 16384 { // 16GB
            warnings.push(format!("Estimated memory usage ({}MB) is very high, ensure sufficient system memory", estimated_memory_mb));
        }

        // Test duration estimation
        if config.performance.enabled {
            let estimated_duration_sec = config.performance.sample_size as u64 * config.performance.max_execution_time_ms / 1000;
            if estimated_duration_sec > 1800 { // 30 minutes
                warnings.push(format!("Estimated performance test duration ({}s) is very long", estimated_duration_sec));
            }
        }

        // Coverage collection overhead
        if config.coverage.enabled && config.execution.max_parallel_tests > 8 {
            warnings.push("Coverage collection with high parallelism may impact performance".to_string());
        }
    }

    /// Validate security settings
    fn validate_security(&self, config: &TestConfig, issues: &mut Vec<ValidationIssue>, warnings: &mut Vec<String>, path: Option<&Path>) {
        let path_str = path.map_or("config".to_string(), |p| p.display().to_string());

        // API key security
        if config.security.api_key_required {
            if config.security.api_key_env_var.is_none() {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    category: ValidationCategory::Security,
                    message: "API key required but no environment variable specified".to_string(),
                    path: path_str.clone(),
                    suggestion: Some("Set security.api_key_env_var to specify the environment variable name".to_string()),
                });
            }
        }

        // Rate limiting
        if config.security.rate_limiting_enabled {
            if config.security.requests_per_minute.unwrap_or(0) == 0 {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    category: ValidationCategory::Security,
                    message: "Rate limiting enabled but requests per minute is 0".to_string(),
                    path: path_str.clone(),
                    suggestion: Some("Set security.requests_per_minute to a positive value".to_string()),
                });
            }
        }

        // Output directory security
        if config.output.directory.starts_with("/tmp/") {
            warnings.push("Output directory in /tmp may be insecure on multi-user systems".to_string());
        }

        // Database security
        if let Some(db_config) = &config.database {
            if db_config.url.contains("password=") {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    category: ValidationCategory::Security,
                    message: "Database URL contains password in plaintext".to_string(),
                    path: path_str.clone(),
                    suggestion: Some("Use environment variables or configuration files for database credentials".to_string()),
                });
            }
        }
    }

    /// Validate best practices
    fn validate_best_practices(&self, config: &TestConfig, suggestions: &mut Vec<String>, path: Option<&Path>) {
        // Cleanup practices
        if !config.output.cleanup_on_success {
            suggestions.push("Consider enabling cleanup_on_success to save disk space".to_string());
        }

        // Coverage practices
        if config.coverage.enabled && config.coverage.threshold_percentage < 70.0 {
            suggestions.push("Consider increasing coverage threshold to at least 70% for better code quality".to_string());
        }

        // Retry practices
        if config.execution.retry_count == 0 {
            suggestions.push("Consider setting retry_count to at least 1 for flaky test resilience".to_string());
        }

        // Logging practices
        if config.logging.level == "debug" && config.test_environment.name.as_deref() == Some("production") {
            suggestions.push("Consider using more appropriate logging level for production".to_string());
        }

        // Performance monitoring
        if !config.monitoring.enabled && config.performance.enabled {
            suggestions.push("Consider enabling monitoring for performance tests to get better insights".to_string());
        }

        // Parallel execution
        if config.execution.max_parallel_tests == 1 && config.test_environment.name.as_deref() == Some("development") {
            suggestions.push("Consider using parallel execution for faster feedback in development".to_string());
        }
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new(ConfigLoader::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::*;

    #[test]
    fn test_validator_creation() {
        let loader = ConfigLoader::default();
        let validator = ConfigValidator::new(loader);
        assert!(validator.loader.config_dir.ends_with("tests/config"));
    }

    #[test]
    fn test_default_validator() {
        let validator = ConfigValidator::default();
        assert!(validator.loader.config_dir.ends_with("tests/config"));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = TestConfig {
            test_environment: TestEnvironment {
                name: "test".to_string(),
                description: "Test environment".to_string(),
            },
            execution: ExecutionConfig {
                timeout_seconds: 30,
                max_parallel_tests: 4,
                retry_count: 1,
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

        let validator = ConfigValidator::default();
        let result = validator.validate_config(&config, None).unwrap();

        assert!(result.is_valid);
        assert!(result.issues.is_empty());
    }
}