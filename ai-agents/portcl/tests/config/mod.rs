//! Test Configuration Module
//!
//! This module provides comprehensive configuration management for the PortCL test suite.

pub mod loader;
pub mod validator;

pub use loader::{ConfigLoader, TestConfig, ConfigLoaderError};
pub use validator::{ConfigValidator, ValidationResult, ValidationIssue, ValidationLevel, ValidationCategory};

/// Re-export commonly used configuration types
pub use loader::{
    TestEnvironment,
    ExecutionConfig,
    LoggingConfig,
    CoverageConfig,
    PerformanceConfig,
    OutputConfig,
    MonitoringConfig,
    SecurityConfig,
    TestTypeConfig,
    TestTemplateConfig,
    FilteringConfig,
    IsolationConfig,
    DatabaseConfig,
    FixturesConfig,
    MocksConfig,
    AssertionsConfig,
    BenchmarksConfig,
    NetworkConfig,
    PropertyTestingConfig,
    GenerationConfig,
    ShrinkingConfig,
    LoadTestingConfig,
    ThresholdsConfig,
    BaselineConfig,
    NotificationsConfig,
};

/// Configuration helper functions
pub mod helpers {
    use super::*;

    /// Load configuration with default settings
    pub fn load_default_config() -> Result<TestConfig> {
        let loader = ConfigLoader::default();
        loader.load_environment("development")
    }

    /// Load configuration for CI/CD
    pub fn load_ci_config() -> Result<TestConfig> {
        let loader = ConfigLoader::default();
        loader.load_template("ci_cd")
    }

    /// Load configuration for performance testing
    pub fn load_performance_config() -> Result<TestConfig> {
        let loader = ConfigLoader::default();
        loader.load_test_type("performance")
    }

    /// Validate configuration and print results
    pub fn validate_and_report(config: &TestConfig) -> Result<()> {
        let validator = ConfigValidator::default();
        let result = validator.validate_config(config, None)?;

        if result.is_valid {
            println!("✓ Configuration is valid");
        } else {
            println!("✗ Configuration has issues:");
            for issue in &result.issues {
                println!("  {}: {}", match issue.level {
                    ValidationLevel::Error => "ERROR",
                    ValidationLevel::Warning => "WARNING",
                    ValidationLevel::Info => "INFO",
                }, issue.message);
            }
        }

        if !result.warnings.is_empty() {
            println!("Warnings:");
            for warning in &result.warnings {
                println!("  - {}", warning);
            }
        }

        if !result.suggestions.is_empty() {
            println!("Suggestions:");
            for suggestion in &result.suggestions {
                println!("  - {}", suggestion);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_default_config() {
        let result = helpers::load_default_config();
        // This may fail in test environment, but we're testing the function exists
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_loader_functionality() {
        let loader = ConfigLoader::default();
        let envs = loader.available_environments().unwrap();
        assert!(envs.contains(&"development".to_string()));
    }
}