use crate::error::{PortCLError, Result};
use crate::config::PortageConfig;
use std::path::Path;
use std::process::Command;

pub fn validate_config(config: &PortageConfig) -> Result<()> {
    // Validate monitoring configuration
    if !config.monitoring.portage_path.exists() {
        return Err(PortCLError::Validation(format!(
            "Portage binary not found at: {}",
            config.monitoring.portage_path.display()
        )));
    }

    if config.monitoring.poll_interval < 10 {
        return Err(PortCLError::Validation(
            "Poll interval must be at least 10 seconds".to_string()
        ));
    }

    // Validate RL configuration
    if !(0.0..=1.0).contains(&config.rl.learning_rate) {
        return Err(PortCLError::Validation(
            "Learning rate must be between 0.0 and 1.0".to_string()
        ));
    }

    if !(0.0..=1.0).contains(&config.rl.discount_factor) {
        return Err(PortCLError::Validation(
            "Discount factor must be between 0.0 and 1.0".to_string()
        ));
    }

    if config.rl.exploration_rate < 0.0 || config.rl.exploration_rate > 1.0 {
        return Err(PortCLError::Validation(
            "Exploration rate must be between 0.0 and 1.0".to_string()
        ));
    }

    if config.rl.batch_size == 0 {
        return Err(PortCLError::Validation(
            "Batch size must be greater than 0".to_string()
        ));
    }

    if config.rl.memory_size < config.rl.batch_size {
        return Err(PortCLError::Validation(
            "Memory size must be greater than or equal to batch size".to_string()
        ));
    }

    // Validate action configuration
    if config.actions.max_concurrent_actions == 0 {
        return Err(PortCLError::Validation(
            "Max concurrent actions must be greater than 0".to_string()
        ));
    }

    if config.actions.action_timeout == 0 {
        return Err(PortCLError::Validation(
            "Action timeout must be greater than 0".to_string()
        ));
    }

    // Validate safety configuration
    if config.safety.max_cpu_usage <= 0.0 || config.safety.max_cpu_usage > 100.0 {
        return Err(PortCLError::Validation(
            "Max CPU usage must be between 0.0 and 100.0".to_string()
        ));
    }

    if config.safety.max_memory_usage <= 0.0 || config.safety.max_memory_usage > 100.0 {
        return Err(PortCLError::Validation(
            "Max memory usage must be between 0.0 and 100.0".to_string()
        ));
    }

    if config.safety.min_disk_space_gb < 0.0 {
        return Err(PortCLError::Validation(
            "Min disk space must be non-negative".to_string()
        ));
    }

    if config.safety.critical_packages.is_empty() {
        return Err(PortCLError::Validation(
            "At least one critical package must be specified".to_string()
        ));
    }

    // Validate general configuration
    if config.general.data_directory.as_os_str().is_empty() {
        return Err(PortCLError::Validation(
            "Data directory cannot be empty".to_string()
        ));
    }

    // Validate log levels
    let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_log_levels.contains(&config.general.log_level.as_str()) {
        return Err(PortCLError::Validation(format!(
            "Invalid log level: {}. Must be one of: {:?}",
            config.general.log_level, valid_log_levels
        )));
    }

    // Validate paths exist or can be created
    if let Some(parent) = config.general.data_directory.parent() {
        if !parent.exists() {
            return Err(PortCLError::Validation(format!(
                "Parent directory does not exist: {}",
                parent.display()
            )));
        }
    }

    Ok(())
}

pub fn validate_system_requirements() -> Result<()> {
    // Check if running on Linux
    if !cfg!(target_os = "linux") {
        return Err(PortCLError::Validation(
            "PortCL can only run on Linux systems".to_string()
        ));
    }

    // Check if Portage is available
    if !Path::new("/usr/bin/portage").exists() && !Path::new("/usr/bin/emerge").exists() {
        return Err(PortCLError::Validation(
            "Portage/emerge not found in system".to_string()
        ));
    }

    // Check if we have necessary permissions
    let output = Command::new("id").arg("-u").output();
    if let Ok(output) = output {
        if let Ok(uid_str) = String::from_utf8(output.stdout) {
            let uid = uid_str.trim();
                    if uid != "0" {
                    // Check if we're in the portcl group
                    // This is a simplified check - in practice, you'd want more sophisticated permission handling
                    return Err(PortCLError::Validation(
                        "PortCL requires root privileges or portcl group membership".to_string()
                    ));
                }
            }
        }

    Ok(())
}