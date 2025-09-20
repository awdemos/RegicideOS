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

/// Mock action for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockAction {
    pub id: String,
    pub action_type: String,
    pub target: String,
    pub parameters: HashMap<String, String>,
    pub expected_outcome: String,
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
        MockAction {
            id: "action_001".to_string(),
            action_type: "install".to_string(),
            target: "sys-apps/portage".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("version".to_string(), "3.0.30".to_string());
                params.insert("use_flags".to_string(), "buildkit,doc".to_string());
                params
            },
            expected_outcome: "success".to_string(),
        },
        MockAction {
            id: "action_002".to_string(),
            action_type: "remove".to_string(),
            target: "dev-lang/rust".to_string(),
            parameters: HashMap::new(),
            expected_outcome: "success".to_string(),
        },
    ]
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