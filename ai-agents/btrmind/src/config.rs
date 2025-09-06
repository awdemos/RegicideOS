use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub monitoring: MonitoringConfig,
    pub thresholds: ThresholdConfig,
    pub actions: ActionConfig,
    pub learning: LearningConfig,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    #[serde(default = "default_target_path")]
    pub target_path: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,
    #[serde(default = "default_trend_window")]
    pub trend_analysis_window: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    #[serde(default = "default_warning_level")]
    pub warning_level: f64,
    #[serde(default = "default_critical_level")]
    pub critical_level: f64,
    #[serde(default = "default_emergency_level")]
    pub emergency_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    #[serde(default = "default_true")]
    pub enable_compression: bool,
    #[serde(default = "default_true")]
    pub enable_balance: bool,
    #[serde(default = "default_true")]
    pub enable_snapshot_cleanup: bool,
    #[serde(default = "default_true")]
    pub enable_temp_cleanup: bool,
    #[serde(default = "default_temp_paths")]
    pub temp_paths: Vec<String>,
    #[serde(default = "default_snapshot_keep")]
    pub snapshot_keep_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    #[serde(default = "default_model_path")]
    pub model_path: String,
    #[serde(default = "default_model_update_interval")]
    pub model_update_interval: u64,
    #[serde(default = "default_reward_smoothing")]
    pub reward_smoothing: f64,
    #[serde(default = "default_exploration_rate")]
    pub exploration_rate: f64,
    #[serde(default = "default_learning_rate")]
    pub learning_rate: f64,
    #[serde(default = "default_discount_factor")]
    pub discount_factor: f64,
}

// Default value functions
fn default_target_path() -> String { "/".to_string() }
fn default_poll_interval() -> u64 { 60 }
fn default_trend_window() -> u64 { 24 }
fn default_warning_level() -> f64 { 85.0 }
fn default_critical_level() -> f64 { 95.0 }
fn default_emergency_level() -> f64 { 98.0 }
fn default_true() -> bool { true }
fn default_temp_paths() -> Vec<String> {
    vec![
        "/tmp".to_string(),
        "/var/tmp".to_string(),
        "/var/cache".to_string(),
        "/home/*/.cache".to_string(),
    ]
}
fn default_snapshot_keep() -> usize { 10 }
fn default_model_path() -> String { "/var/lib/btrmind/model.safetensors".to_string() }
fn default_model_update_interval() -> u64 { 3600 }
fn default_reward_smoothing() -> f64 { 0.95 }
fn default_exploration_rate() -> f64 { 0.1 }
fn default_learning_rate() -> f64 { 0.001 }
fn default_discount_factor() -> f64 { 0.99 }

impl Default for Config {
    fn default() -> Self {
        Self {
            monitoring: MonitoringConfig {
                target_path: default_target_path(),
                poll_interval: default_poll_interval(),
                trend_analysis_window: default_trend_window(),
            },
            thresholds: ThresholdConfig {
                warning_level: default_warning_level(),
                critical_level: default_critical_level(),
                emergency_level: default_emergency_level(),
            },
            actions: ActionConfig {
                enable_compression: default_true(),
                enable_balance: default_true(),
                enable_snapshot_cleanup: default_true(),
                enable_temp_cleanup: default_true(),
                temp_paths: default_temp_paths(),
                snapshot_keep_count: default_snapshot_keep(),
            },
            learning: LearningConfig {
                model_path: default_model_path(),
                model_update_interval: default_model_update_interval(),
                reward_smoothing: default_reward_smoothing(),
                exploration_rate: default_exploration_rate(),
                learning_rate: default_learning_rate(),
                discount_factor: default_discount_factor(),
            },
            dry_run: false,
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            // Create default config if it doesn't exist
            let default_config = Config::default();
            default_config.save(path)?;
            return Ok(default_config);
        }
        
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;
        
        Ok(config)
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }
        
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;
        
        Ok(())
    }
    
    pub fn validate(&self) -> Result<()> {
        // Validate thresholds
        if self.thresholds.warning_level >= self.thresholds.critical_level {
            anyhow::bail!("Warning level must be less than critical level");
        }
        
        if self.thresholds.critical_level >= self.thresholds.emergency_level {
            anyhow::bail!("Critical level must be less than emergency level");
        }
        
        if self.thresholds.emergency_level >= 100.0 {
            anyhow::bail!("Emergency level must be less than 100%");
        }
        
        // Validate learning parameters
        if self.learning.learning_rate <= 0.0 || self.learning.learning_rate > 1.0 {
            anyhow::bail!("Learning rate must be between 0 and 1");
        }
        
        if self.learning.discount_factor < 0.0 || self.learning.discount_factor > 1.0 {
            anyhow::bail!("Discount factor must be between 0 and 1");
        }
        
        if self.learning.exploration_rate < 0.0 || self.learning.exploration_rate > 1.0 {
            anyhow::bail!("Exploration rate must be between 0 and 1");
        }
        
        // Validate paths
        let target_path = Path::new(&self.monitoring.target_path);
        if !target_path.exists() {
            anyhow::bail!("Target path does not exist: {}", self.monitoring.target_path);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.thresholds.warning_level, 85.0);
        assert_eq!(config.thresholds.critical_level, 95.0);
        assert_eq!(config.thresholds.emergency_level, 98.0);
    }
    
    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.thresholds.warning_level, deserialized.thresholds.warning_level);
        assert_eq!(config.learning.learning_rate, deserialized.learning.learning_rate);
    }
    
    #[test]
    fn test_config_save_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::default();
        
        config.save(temp_file.path()).unwrap();
        let loaded_config = Config::load(temp_file.path()).unwrap();
        
        assert_eq!(config.thresholds.warning_level, loaded_config.thresholds.warning_level);
    }
    
    #[test]
    fn test_invalid_thresholds() {
        let mut config = Config::default();
        config.thresholds.warning_level = 95.0;
        config.thresholds.critical_level = 85.0; // Invalid: less than warning
        
        assert!(config.validate().is_err());
    }
}
