use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::{PortCLError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortageConfig {
    pub monitoring: MonitoringConfig,
    pub rl: RLConfig,
    pub actions: ActionConfig,
    pub safety: SafetyConfig,
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub poll_interval: u64,
    pub portage_path: PathBuf,
    pub log_path: PathBuf,
    pub metrics_retention_days: u32,
    pub enable_event_tracking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub exploration_rate: f64,
    pub exploration_decay: f64,
    pub memory_size: usize,
    pub batch_size: usize,
    pub target_update_freq: usize,
    pub training_freq: usize,
    pub save_freq: usize,
    pub model_path: PathBuf,
    pub enable_continual_learning: bool,
    pub ewc_importance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    pub enable_dry_run: bool,
    pub max_concurrent_actions: usize,
    pub action_timeout: u64,
    pub rollback_enabled: bool,
    pub safe_actions_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub max_cpu_usage: f64,
    pub max_memory_usage: f64,
    pub min_disk_space_gb: f64,
    pub critical_packages: Vec<String>,
    pub enable_system_checks: bool,
    pub backup_before_actions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: String,
    pub data_directory: PathBuf,
    pub user: String,
    pub group: String,
    pub enable_metrics_collection: bool,
}

impl Default for PortageConfig {
    fn default() -> Self {
        Self {
            monitoring: MonitoringConfig::default(),
            rl: RLConfig::default(),
            actions: ActionConfig::default(),
            safety: SafetyConfig::default(),
            general: GeneralConfig::default(),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            poll_interval: 30,
            portage_path: PathBuf::from("/usr/bin/portage"),
            log_path: PathBuf::from("/var/log/portcl.log"),
            metrics_retention_days: 30,
            enable_event_tracking: true,
        }
    }
}

impl Default for RLConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            discount_factor: 0.95,
            exploration_rate: 0.1,
            exploration_decay: 0.995,
            memory_size: 10000,
            batch_size: 32,
            target_update_freq: 100,
            training_freq: 10,
            save_freq: 1000,
            model_path: PathBuf::from("/var/lib/portcl/model.pt"),
            enable_continual_learning: true,
            ewc_importance: 1000.0,
        }
    }
}

impl Default for ActionConfig {
    fn default() -> Self {
        Self {
            enable_dry_run: true,
            max_concurrent_actions: 3,
            action_timeout: 300,
            rollback_enabled: true,
            safe_actions_only: true,
        }
    }
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
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
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            data_directory: PathBuf::from("/var/lib/portcl"),
            user: "portcl".to_string(),
            group: "portcl".to_string(),
            enable_metrics_collection: true,
        }
    }
}

impl PortageConfig {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PortCLError::Configuration(format!("Failed to read config file: {}", e)))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| PortCLError::Configuration(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| PortCLError::Configuration(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| PortCLError::Configuration(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.monitoring.poll_interval == 0 {
            return Err(PortCLError::Validation("Poll interval must be greater than 0".to_string()));
        }

        if !(0.0..=1.0).contains(&self.rl.learning_rate) {
            return Err(PortCLError::Validation("Learning rate must be between 0 and 1".to_string()));
        }

        if !(0.0..=1.0).contains(&self.rl.discount_factor) {
            return Err(PortCLError::Validation("Discount factor must be between 0 and 1".to_string()));
        }

        if self.rl.batch_size > self.rl.memory_size {
            return Err(PortCLError::Validation("Batch size cannot be larger than memory size".to_string()));
        }

        if self.safety.max_cpu_usage <= 0.0 || self.safety.max_cpu_usage > 100.0 {
            return Err(PortCLError::Validation("Max CPU usage must be between 0 and 100".to_string()));
        }

        Ok(())
    }
}