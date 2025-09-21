use portcl::config::{
    PortageConfig, MonitoringConfig, RLConfig, ActionConfig, SafetyConfig, GeneralConfig,
    validate_config
};
use portcl::error::{PortCLError, Result};
use portcl::prelude::*;

use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};
use std::fs;
use std::collections::HashMap;

// Test utilities
fn create_test_portage_config() -> PortageConfig {
    PortageConfig {
        monitoring: MonitoringConfig {
            poll_interval: 60,
            portage_path: PathBuf::from("/usr/bin/portage"),
            log_path: PathBuf::from("/var/log/portcl.log"),
            metrics_retention_days: 30,
            enable_event_tracking: true,
        },
        rl: RLConfig {
            learning_rate: 0.001,
            discount_factor: 0.95,
            exploration_rate: 0.1,
            exploration_decay: 0.995,
            memory_size: 10000,
            batch_size: 32,
            target_update_freq: 100,
            model_path: PathBuf::from("/var/lib/portcl/model.pt"),
            enable_continual_learning: true,
            ewc_importance: 1000.0,
        },
        actions: ActionConfig {
            enable_dry_run: true,
            max_concurrent_actions: 3,
            action_timeout: 300,
            rollback_enabled: true,
            safe_actions_only: true,
        },
        safety: SafetyConfig {
            max_cpu_usage: 90.0,
            max_memory_usage: 80.0,
            min_disk_space_gb: 5.0,
            critical_packages: vec![
                "sys-kernel/gentoo-kernel".to_string(),
                "sys-apps/systemd".to_string(),
                "sys-apps/portage".to_string(),
            ],
            enable_system_checks: true,
            backup_before_actions: true,
        },
        general: GeneralConfig {
            log_level: "info".to_string(),
            data_directory: PathBuf::from("/var/lib/portcl"),
            user: "portcl".to_string(),
            group: "portcl".to_string(),
            enable_metrics_collection: true,
        },
    }
}

fn create_test_config_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

#[tokio::test]
async fn test_portage_config_default() {
    let config = PortageConfig::default();

    // Check monitoring defaults
    assert_eq!(config.monitoring.poll_interval, 30);
    assert_eq!(config.monitoring.portage_path, PathBuf::from("/usr/bin/portage"));
    assert_eq!(config.monitoring.log_path, PathBuf::from("/var/log/portcl.log"));
    assert_eq!(config.monitoring.metrics_retention_days, 30);
    assert!(config.monitoring.enable_event_tracking);

    // Check RL defaults
    assert_eq!(config.rl.learning_rate, 0.001);
    assert_eq!(config.rl.discount_factor, 0.95);
    assert_eq!(config.rl.exploration_rate, 0.1);
    assert_eq!(config.rl.exploration_decay, 0.995);
    assert_eq!(config.rl.memory_size, 10000);
    assert_eq!(config.rl.batch_size, 32);
    assert_eq!(config.rl.target_update_freq, 100);
    assert_eq!(config.rl.model_path, PathBuf::from("/var/lib/portcl/model.pt"));
    assert!(config.rl.enable_continual_learning);
    assert_eq!(config.rl.ewc_importance, 1000.0);

    // Check action defaults
    assert!(config.actions.enable_dry_run);
    assert_eq!(config.actions.max_concurrent_actions, 3);
    assert_eq!(config.actions.action_timeout, 300);
    assert!(config.actions.rollback_enabled);
    assert!(config.actions.safe_actions_only);

    // Check safety defaults
    assert_eq!(config.safety.max_cpu_usage, 90.0);
    assert_eq!(config.safety.max_memory_usage, 80.0);
    assert_eq!(config.safety.min_disk_space_gb, 5.0);
    assert!(!config.safety.critical_packages.is_empty());
    assert!(config.safety.enable_system_checks);
    assert!(config.safety.backup_before_actions);

    // Check general defaults
    assert_eq!(config.general.log_level, "info");
    assert_eq!(config.general.data_directory, PathBuf::from("/var/lib/portcl"));
    assert_eq!(config.general.user, "portcl");
    assert_eq!(config.general.group, "portcl");
    assert!(config.general.enable_metrics_collection);
}

#[tokio::test]
async fn test_monitoring_config_default() {
    let config = MonitoringConfig::default();

    assert_eq!(config.poll_interval, 30);
    assert_eq!(config.portage_path, PathBuf::from("/usr/bin/portage"));
    assert_eq!(config.log_path, PathBuf::from("/var/log/portcl.log"));
    assert_eq!(config.metrics_retention_days, 30);
    assert!(config.enable_event_tracking);
}

#[tokio::test]
async fn test_rl_config_default() {
    let config = RLConfig::default();

    assert_eq!(config.learning_rate, 0.001);
    assert_eq!(config.discount_factor, 0.95);
    assert_eq!(config.exploration_rate, 0.1);
    assert_eq!(config.exploration_decay, 0.995);
    assert_eq!(config.memory_size, 10000);
    assert_eq!(config.batch_size, 32);
    assert_eq!(config.target_update_freq, 100);
    assert_eq!(config.model_path, PathBuf::from("/var/lib/portcl/model.pt"));
    assert!(config.enable_continual_learning);
    assert_eq!(config.ewc_importance, 1000.0);
}

#[tokio::test]
async fn test_action_config_default() {
    let config = ActionConfig::default();

    assert!(config.enable_dry_run);
    assert_eq!(config.max_concurrent_actions, 3);
    assert_eq!(config.action_timeout, 300);
    assert!(config.rollback_enabled);
    assert!(config.safe_actions_only);
}

#[tokio::test]
async fn test_safety_config_default() {
    let config = SafetyConfig::default();

    assert_eq!(config.max_cpu_usage, 90.0);
    assert_eq!(config.max_memory_usage, 80.0);
    assert_eq!(config.min_disk_space_gb, 5.0);
    assert!(!config.critical_packages.is_empty());
    assert_eq!(config.critical_packages.len(), 3);
    assert!(config.critical_packages.contains(&"sys-kernel/gentoo-kernel".to_string()));
    assert!(config.critical_packages.contains(&"sys-apps/systemd".to_string()));
    assert!(config.critical_packages.contains(&"sys-apps/portage".to_string()));
    assert!(config.enable_system_checks);
    assert!(config.backup_before_actions);
}

#[tokio::test]
async fn test_general_config_default() {
    let config = GeneralConfig::default();

    assert_eq!(config.log_level, "info");
    assert_eq!(config.data_directory, PathBuf::from("/var/lib/portcl"));
    assert_eq!(config.user, "portcl");
    assert_eq!(config.group, "portcl");
    assert!(config.enable_metrics_collection);
}

#[tokio::test]
async fn test_portage_config_serialization() {
    let config = create_test_portage_config();

    // Test JSON serialization
    let json_result = serde_json::to_string(&config);
    assert!(json_result.is_ok());

    let json_str = json_result.unwrap();
    let deserialized: PortageConfig = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.monitoring.poll_interval, config.monitoring.poll_interval);
    assert_eq!(deserialized.rl.learning_rate, config.rl.learning_rate);
    assert_eq!(deserialized.actions.max_concurrent_actions, config.actions.max_concurrent_actions);
}

#[tokio::test]
async fn test_config_file_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let original_config = create_test_portage_config();

    // Save config
    let save_result = original_config.save(config_path.to_str().unwrap());
    assert!(save_result.is_ok());

    // Load config
    let load_result = PortageConfig::load(config_path.to_str().unwrap());
    assert!(load_result.is_ok());

    let loaded_config = load_result.unwrap();

    // Verify loaded config matches original
    assert_eq!(loaded_config.monitoring.poll_interval, original_config.monitoring.poll_interval);
    assert_eq!(loaded_config.rl.learning_rate, original_config.rl.learning_rate);
    assert_eq!(loaded_config.actions.max_concurrent_actions, original_config.actions.max_concurrent_actions);
    assert_eq!(loaded_config.safety.max_cpu_usage, original_config.safety.max_cpu_usage);
    assert_eq!(loaded_config.general.log_level, original_config.general.log_level);
}

#[tokio::test]
async fn test_config_load_invalid_file() {
    let result = PortageConfig::load("/nonexistent/config.toml");
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Configuration(msg) => {
            assert!(msg.contains("Failed to read config file"));
        },
        _ => panic!("Expected Configuration error"),
    }
}

#[tokio::test]
async fn test_config_load_invalid_toml() {
    let invalid_toml = "[monitoring\npoll_interval = 30"; // Missing closing bracket
    let temp_file = create_test_config_file(invalid_toml);

    let result = PortageConfig::load(temp_file.path().to_str().unwrap());
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Configuration(msg) => {
            assert!(msg.contains("Failed to parse config"));
        },
        _ => panic!("Expected Configuration error"),
    }
}

#[tokio::test]
async fn test_portage_config_validation_success() {
    let config = create_test_portage_config();

    let result = config.validate();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_portage_config_validation_invalid_poll_interval() {
    let mut config = create_test_portage_config();
    config.monitoring.poll_interval = 0;

    let result = config.validate();
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Poll interval must be greater than 0"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_portage_config_validation_invalid_learning_rate() {
    let mut config = create_test_portage_config();
    config.rl.learning_rate = 1.5; // > 1.0

    let result = config.validate();
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Learning rate must be between 0 and 1"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_portage_config_validation_invalid_discount_factor() {
    let mut config = create_test_portage_config();
    config.rl.discount_factor = -0.1; // < 0.0

    let result = config.validate();
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Discount factor must be between 0 and 1"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_portage_config_validation_batch_size_larger_than_memory() {
    let mut config = create_test_portage_config();
    config.rl.batch_size = 50000; // > memory_size
    config.rl.memory_size = 10000;

    let result = config.validate();
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Batch size cannot be larger than memory size"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_portage_config_validation_invalid_cpu_usage() {
    let mut config = create_test_portage_config();
    config.safety.max_cpu_usage = 150.0; // > 100.0

    let result = config.validate();
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Max CPU usage must be between 0 and 100"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_success() {
    let config = create_test_portage_config();

    let result = validate_config(&config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_config_function_missing_portage_binary() {
    let mut config = create_test_portage_config();
    config.monitoring.portage_path = PathBuf::from("/nonexistent/portage");

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Portage binary not found"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_low_poll_interval() {
    let mut config = create_test_portage_config();
    config.monitoring.poll_interval = 5; // < 10

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Poll interval must be at least 10 seconds"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_invalid_exploration_rate() {
    let mut config = create_test_portage_config();
    config.rl.exploration_rate = 1.5; // > 1.0

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Exploration rate must be between 0.0 and 1.0"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_zero_batch_size() {
    let mut config = create_test_portage_config();
    config.rl.batch_size = 0;

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Batch size must be greater than 0"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_memory_smaller_than_batch() {
    let mut config = create_test_portage_config();
    config.rl.memory_size = 16;
    config.rl.batch_size = 32; // > memory_size

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Memory size must be greater than or equal to batch size"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_zero_concurrent_actions() {
    let mut config = create_test_portage_config();
    config.actions.max_concurrent_actions = 0;

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Max concurrent actions must be greater than 0"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_zero_action_timeout() {
    let mut config = create_test_portage_config();
    config.actions.action_timeout = 0;

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Action timeout must be greater than 0"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_invalid_cpu_usage_range() {
    let mut config = create_test_portage_config();
    config.safety.max_cpu_usage = -10.0; // < 0.0

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Max CPU usage must be between 0.0 and 100.0"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_invalid_memory_usage_range() {
    let mut config = create_test_portage_config();
    config.safety.max_memory_usage = 150.0; // > 100.0

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Max memory usage must be between 0.0 and 100.0"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_negative_disk_space() {
    let mut config = create_test_portage_config();
    config.safety.min_disk_space_gb = -5.0;

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Min disk space must be non-negative"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_empty_critical_packages() {
    let mut config = create_test_portage_config();
    config.safety.critical_packages = Vec::new();

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("At least one critical package must be specified"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_empty_data_directory() {
    let mut config = create_test_portage_config();
    config.general.data_directory = PathBuf::from("");

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Data directory cannot be empty"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_invalid_log_level() {
    let mut config = create_test_portage_config();
    config.general.log_level = "invalid".to_string();

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Invalid log level"));
            assert!(msg.contains("Must be one of"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_validate_config_function_valid_log_levels() {
    let valid_log_levels = ["trace", "debug", "info", "warn", "error"];

    for &log_level in &valid_log_levels {
        let mut config = create_test_portage_config();
        config.general.log_level = log_level.to_string();

        let result = validate_config(&config);
        assert!(result.is_ok(), "Log level '{}' should be valid", log_level);
    }
}

#[tokio::test]
async fn test_validate_config_function_nonexistent_parent_directory() {
    let mut config = create_test_portage_config();
    config.general.data_directory = PathBuf::from("/nonexistent/directory/portcl");

    let result = validate_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Parent directory does not exist"));
        },
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_config_clone_behavior() {
    let original = create_test_portage_config();
    let cloned = original.clone();

    // Modify original
    let mut modified = original;
    modified.monitoring.poll_interval = 120;
    modified.rl.learning_rate = 0.01;

    // Verify clone is unchanged
    assert_eq!(cloned.monitoring.poll_interval, 60);
    assert_eq!(cloned.rl.learning_rate, 0.001);

    // Verify modified is changed
    assert_eq!(modified.monitoring.poll_interval, 120);
    assert_eq!(modified.rl.learning_rate, 0.01);
}

#[tokio::test]
async fn test_config_debug_format() {
    let config = create_test_portage_config();
    let debug_string = format!("{:?}", config);

    // Debug output should contain key configuration values
    assert!(debug_string.contains("PortageConfig"));
    assert!(debug_string.contains("poll_interval"));
    assert!(debug_string.contains("learning_rate"));
    assert!(debug_string.contains("max_concurrent_actions"));
    assert!(debug_string.contains("max_cpu_usage"));
}

#[tokio::test]
async fn test_config_partial_serialization() {
    // Test that we can serialize and deserialize partial configurations
    let partial_config = PortageConfig {
        monitoring: MonitoringConfig::default(),
        rl: RLConfig {
            learning_rate: 0.01,
            ..Default::default()
        },
        actions: ActionConfig::default(),
        safety: SafetyConfig::default(),
        general: GeneralConfig::default(),
    };

    let json_result = serde_json::to_string(&partial_config);
    assert!(json_result.is_ok());

    let json_str = json_result.unwrap();
    let deserialized: PortageConfig = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.rl.learning_rate, 0.01);
    assert_eq!(deserialized.monitoring.poll_interval, 30); // Default value
}

#[tokio::test]
async fn test_config_edge_values() {
    // Test boundary values for configuration parameters
    let mut config = create_test_portage_config();

    // Valid boundary values
    config.rl.learning_rate = 0.0;
    config.rl.discount_factor = 1.0;
    config.safety.max_cpu_usage = 100.0;
    config.safety.max_memory_usage = 0.0;
    config.safety.min_disk_space_gb = 0.0;

    let result = validate_config(&config);
    assert!(result.is_ok(), "Boundary values should be valid");

    // Invalid boundary values
    config.rl.learning_rate = 1.001; // Slightly above 1.0
    let result = validate_config(&config);
    assert!(result.is_err(), "Slightly above 1.0 should be invalid");
}

#[tokio::test]
async fn test_config_path_handling() {
    let config = create_test_portage_config();

    // Test that paths are handled correctly
    assert_eq!(config.monitoring.portage_path, PathBuf::from("/usr/bin/portage"));
    assert_eq!(config.monitoring.log_path, PathBuf::from("/var/log/portcl.log"));
    assert_eq!(config.rl.model_path, PathBuf::from("/var/lib/portcl/model.pt"));
    assert_eq!(config.general.data_directory, PathBuf::from("/var/lib/portcl"));

    // Test path serialization
    let json_result = serde_json::to_string(&config);
    assert!(json_result.is_ok());

    let json_str = json_result.unwrap();
    let deserialized: PortageConfig = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.monitoring.portage_path, config.monitoring.portage_path);
    assert_eq!(deserialized.general.data_directory, config.general.data_directory);
}

#[tokio::test]
async fn test_config_toml_roundtrip() {
    let original_config = create_test_portage_config();

    // Convert to TOML
    let toml_content = toml::to_string_pretty(&original_config).unwrap();

    // Parse back from TOML
    let parsed_config: PortageConfig = toml::from_str(&toml_content).unwrap();

    // Verify all fields match
    assert_eq!(parsed_config.monitoring.poll_interval, original_config.monitoring.poll_interval);
    assert_eq!(parsed_config.rl.learning_rate, original_config.rl.learning_rate);
    assert_eq!(parsed_config.actions.max_concurrent_actions, original_config.actions.max_concurrent_actions);
    assert_eq!(parsed_config.safety.critical_packages, original_config.safety.critical_packages);
    assert_eq!(parsed_config.general.log_level, original_config.general.log_level);
}

#[tokio::test]
async fn test_config_customization() {
    let base_config = PortageConfig::default();

    // Create custom configuration with specific values
    let custom_config = PortageConfig {
        monitoring: MonitoringConfig {
            poll_interval: 120,
            ..base_config.monitoring
        },
        rl: RLConfig {
            learning_rate: 0.01,
            memory_size: 50000,
            ..base_config.rl
        },
        actions: ActionConfig {
            max_concurrent_actions: 5,
            enable_dry_run: false,
            ..base_config.actions
        },
        safety: SafetyConfig {
            max_cpu_usage: 95.0,
            critical_packages: vec!["sys-apps/portage".to_string()],
            ..base_config.safety
        },
        general: GeneralConfig {
            log_level: "debug".to_string(),
            ..base_config.general
        },
    };

    // Verify custom values
    assert_eq!(custom_config.monitoring.poll_interval, 120);
    assert_eq!(custom_config.rl.learning_rate, 0.01);
    assert_eq!(custom_config.rl.memory_size, 50000);
    assert_eq!(custom_config.actions.max_concurrent_actions, 5);
    assert!(!custom_config.actions.enable_dry_run);
    assert_eq!(custom_config.safety.max_cpu_usage, 95.0);
    assert_eq!(custom_config.safety.critical_packages.len(), 1);
    assert_eq!(custom_config.general.log_level, "debug");
}

#[tokio::test]
async fn test_config_comprehensive_validation() {
    let config = create_test_portage_config();

    // Test comprehensive validation covering all aspects
    let result = validate_config(&config);
    assert!(result.is_ok(), "Comprehensive valid config should pass validation");

    // Test multiple validation errors by creating an invalid config
    let mut invalid_config = create_test_portage_config();
    invalid_config.monitoring.poll_interval = 5; // Too low
    invalid_config.rl.learning_rate = 2.0; // Too high
    invalid_config.actions.max_concurrent_actions = 0; // Invalid
    invalid_config.safety.critical_packages = Vec::new(); // Empty

    let result = validate_config(&invalid_config);
    assert!(result.is_err(), "Invalid config should fail validation");

    // Should get the first validation error encountered
    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            // Should report one of the validation errors
            assert!(msg.contains("Poll interval must be at least 10 seconds") ||
                    msg.contains("Learning rate must be between 0.0 and 1.0") ||
                    msg.contains("Max concurrent actions must be greater than 0") ||
                    msg.contains("At least one critical package must be specified"));
        },
        _ => panic!("Expected Validation error"),
    }
}