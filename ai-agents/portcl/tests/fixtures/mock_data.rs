//! Mock data for testing PortCL components

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mock package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockPackage {
    pub name: String,
    pub category: String,
    pub version: String,
    pub description: String,
    pub homepage: Option<String>,
    pub license: String,
    pub use_flags: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Mock action for testing - mirrors the actual PortCL Action enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MockAction {
    NoOp,
    AdjustParallelism { jobs: u32 },
    OptimizeBuildOrder { package_list: Vec<String> },
    ScheduleOperation { delay_seconds: u64 },
    PreFetchDependencies { packages: Vec<String> },
    CleanObsoletePackages { force: bool },
}

/// ActionType enum for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MockActionType {
    NoOp,
    AdjustParallelism,
    OptimizeBuildOrder,
    ScheduleOperation,
    PreFetchDependencies,
    CleanObsoletePackages,
}

/// Mock configuration for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockConfig {
    pub api_key: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub log_level: String,
    pub enable_ml: bool,
}

/// Mock RL state for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRLState {
    pub episode_count: u32,
    pub total_reward: f64,
    pub success_count: u32,
    pub failure_count: u32,
    pub current_policy: String,
}

/// Mock PortageConfig for testing PortCL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockPortageConfig {
    pub monitoring: MockMonitoringConfig,
    pub rl: MockRLConfig,
    pub actions: MockActionConfig,
    pub safety: MockSafetyConfig,
    pub general: MockGeneralConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockMonitoringConfig {
    pub poll_interval: u64,
    pub portage_path: String,
    pub log_path: String,
    pub metrics_retention_days: u32,
    pub enable_event_tracking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRLConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub exploration_rate: f64,
    pub exploration_decay: f64,
    pub memory_size: usize,
    pub batch_size: usize,
    pub target_update_freq: usize,
    pub model_path: String,
    pub enable_continual_learning: bool,
    pub ewc_importance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockActionConfig {
    pub enable_dry_run: bool,
    pub max_concurrent_actions: usize,
    pub action_timeout: u64,
    pub rollback_enabled: bool,
    pub safe_actions_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSafetyConfig {
    pub max_cpu_usage: f64,
    pub max_memory_usage: f64,
    pub min_disk_space_gb: f64,
    pub critical_packages: Vec<String>,
    pub enable_system_checks: bool,
    pub backup_before_actions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockGeneralConfig {
    pub log_level: String,
    pub data_directory: String,
    pub user: String,
    pub group: String,
    pub enable_metrics_collection: bool,
}

/// Mock learning event for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockLearningEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action_id: String,
    pub reward: f64,
    pub state_before: String,
    pub state_after: String,
    pub learning_rate: f64,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            api_key: "test_api_key".to_string(),
            base_url: "https://api.test.com".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            log_level: "info".to_string(),
            enable_ml: false,
        }
    }
}

impl Default for MockRLState {
    fn default() -> Self {
        Self {
            episode_count: 100,
            total_reward: 1250.5,
            success_count: 85,
            failure_count: 15,
            current_policy: "epsilon_greedy".to_string(),
        }
    }
}

impl Default for MockPortageConfig {
    fn default() -> Self {
        Self {
            monitoring: MockMonitoringConfig::default(),
            rl: MockRLConfig::default(),
            actions: MockActionConfig::default(),
            safety: MockSafetyConfig::default(),
            general: MockGeneralConfig::default(),
        }
    }
}

impl Default for MockMonitoringConfig {
    fn default() -> Self {
        Self {
            poll_interval: 30,
            portage_path: "/usr/bin/portage".to_string(),
            log_path: "/var/log/portcl.log".to_string(),
            metrics_retention_days: 30,
            enable_event_tracking: true,
        }
    }
}

impl Default for MockRLConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            discount_factor: 0.95,
            exploration_rate: 0.1,
            exploration_decay: 0.995,
            memory_size: 10000,
            batch_size: 32,
            target_update_freq: 100,
            model_path: "/var/lib/portcl/model.pt".to_string(),
            enable_continual_learning: true,
            ewc_importance: 1000.0,
        }
    }
}

impl Default for MockActionConfig {
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

impl Default for MockSafetyConfig {
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

impl Default for MockGeneralConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            data_directory: "/var/lib/portcl".to_string(),
            user: "portcl".to_string(),
            group: "portcl".to_string(),
            enable_metrics_collection: true,
        }
    }
}

pub fn sample_packages() -> Vec<MockPackage> {
    vec![
        MockPackage {
            name: "portage".to_string(),
            category: "sys-apps".to_string(),
            version: "3.0.30".to_string(),
            description: "Portage package management system".to_string(),
            homepage: Some("https://wiki.gentoo.org/wiki/Portage".to_string()),
            license: "GPL-2".to_string(),
            use_flags: vec!["buildkit".to_string(), "doc".to_string()],
            dependencies: vec!["python".to_string(), "rsync".to_string()],
        },
        MockPackage {
            name: "rust".to_string(),
            category: "dev-lang".to_string(),
            version: "1.75.0".to_string(),
            description: "Rust programming language".to_string(),
            homepage: Some("https://www.rust-lang.org/".to_string()),
            license: "MIT Apache-2.0".to_string(),
            use_flags: vec!["clippy".to_string(), "rustfmt".to_string()],
            dependencies: vec!["binutils".to_string(), "gcc".to_string()],
        },
    ]
}

pub fn sample_actions() -> Vec<MockAction> {
    vec![
        MockAction::NoOp,
        MockAction::AdjustParallelism { jobs: 4 },
        MockAction::OptimizeBuildOrder {
            package_list: vec![
                "sys-apps/portage".to_string(),
                "dev-lang/rust".to_string(),
                "sys-kernel/gentoo-kernel".to_string(),
            ],
        },
        MockAction::ScheduleOperation { delay_seconds: 30 },
        MockAction::PreFetchDependencies {
            packages: vec![
                "sys-apps/portage".to_string(),
                "dev-lang/rust".to_string(),
            ],
        },
        MockAction::CleanObsoletePackages { force: false },
    ]
}

impl MockAction {
    pub fn action_type(&self) -> MockActionType {
        match self {
            MockAction::NoOp => MockActionType::NoOp,
            MockAction::AdjustParallelism { .. } => MockActionType::AdjustParallelism,
            MockAction::OptimizeBuildOrder { .. } => MockActionType::OptimizeBuildOrder,
            MockAction::ScheduleOperation { .. } => MockActionType::ScheduleOperation,
            MockAction::PreFetchDependencies { .. } => MockActionType::PreFetchDependencies,
            MockAction::CleanObsoletePackages { .. } => MockActionType::CleanObsoletePackages,
        }
    }

    pub fn is_safe(&self) -> bool {
        match self {
            MockAction::NoOp => true,
            MockAction::AdjustParallelism { jobs } => *jobs >= 1 && *jobs <= 32,
            MockAction::OptimizeBuildOrder { .. } => true,
            MockAction::ScheduleOperation { .. } => true,
            MockAction::PreFetchDependencies { .. } => true,
            MockAction::CleanObsoletePackages { force } => !force,
        }
    }

    pub fn description(&self) -> String {
        match self {
            MockAction::NoOp => "No operation".to_string(),
            MockAction::AdjustParallelism { jobs } => format!("Adjust compilation parallelism to {} jobs", jobs),
            MockAction::OptimizeBuildOrder { package_list } => {
                format!("Optimize build order for {} packages", package_list.len())
            },
            MockAction::ScheduleOperation { delay_seconds } => {
                format!("Schedule operation with {}s delay", delay_seconds)
            },
            MockAction::PreFetchDependencies { packages } => {
                format!("Pre-fetch dependencies for {} packages", packages.len())
            },
            MockAction::CleanObsoletePackages { force } => {
                format!("Clean obsolete packages (force: {})", force)
            },
        }
    }
}

pub fn sample_learning_events() -> Vec<MockLearningEvent> {
    vec![
        MockLearningEvent {
            timestamp: chrono::Utc::now(),
            action_id: "action_001".to_string(),
            reward: 1.0,
            state_before: "package_not_installed".to_string(),
            state_after: "package_installed".to_string(),
            learning_rate: 0.1,
        },
        MockLearningEvent {
            timestamp: chrono::Utc::now(),
            action_id: "action_002".to_string(),
            reward: -0.5,
            state_before: "package_installed".to_string(),
            state_after: "package_removed".to_string(),
            learning_rate: 0.1,
        },
    ]
}

pub fn sample_portage_configs() -> Vec<MockPortageConfig> {
    vec![
        MockPortageConfig {
            monitoring: MockMonitoringConfig {
                poll_interval: 60,
                portage_path: "/usr/bin/portage".to_string(),
                log_path: "/var/log/portcl.log".to_string(),
                metrics_retention_days: 7,
                enable_event_tracking: false,
            },
            rl: MockRLConfig {
                learning_rate: 0.01,
                discount_factor: 0.99,
                exploration_rate: 0.2,
                exploration_decay: 0.999,
                memory_size: 50000,
                batch_size: 64,
                target_update_freq: 200,
                model_path: "/var/lib/portcl/advanced_model.pt".to_string(),
                enable_continual_learning: true,
                ewc_importance: 5000.0,
            },
            actions: MockActionConfig {
                enable_dry_run: false,
                max_concurrent_actions: 5,
                action_timeout: 600,
                rollback_enabled: false,
                safe_actions_only: false,
            },
            safety: MockSafetyConfig {
                max_cpu_usage: 95.0,
                max_memory_usage: 85.0,
                min_disk_space_gb: 10.0,
                critical_packages: vec![
                    "sys-kernel/gentoo-kernel".to_string(),
                    "sys-apps/systemd".to_string(),
                    "sys-apps/portage".to_string(),
                    "net-misc/curl".to_string(),
                ],
                enable_system_checks: false,
                backup_before_actions: false,
            },
            general: MockGeneralConfig {
                log_level: "debug".to_string(),
                data_directory: "/var/lib/portcl".to_string(),
                user: "root".to_string(),
                group: "root".to_string(),
                enable_metrics_collection: true,
            },
        },
        MockPortageConfig::default(),
    ]
}