//! Portage-specific actions for PortCL
//!
//! This module defines actions that interact directly with the Portage
//! package management system.

use crate::error::{PortCLError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Types of Portage-specific actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PortageActionType {
    Install,
    Uninstall,
    Update,
    Search,
    Info,
    Depends,
    Rdepends,
    Emerge,
    Sync,
    Clean,
    Mask,
    Unmask,
}

/// Parameters for Portage actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionParams {
    InstallParams {
        packages: Vec<String>,
        use_flags: HashSet<String>,
        verbose: bool,
        pretend: bool,
    },
    UninstallParams {
        packages: Vec<String>,
        clean_deps: bool,
        config_files: String,
    },
    UpdateParams {
        packages: Vec<String>,
        deep: bool,
        newuse: bool,
        backtrack: u32,
    },
    SearchParams {
        query: String,
        search_desc: bool,
        regex: bool,
    },
    InfoParams {
        packages: Vec<String>,
        verbose: bool,
    },
    DependsParams {
        packages: Vec<String>,
        deep: bool,
    },
    EmergeParams {
        packages: Vec<String>,
        use_flags: HashSet<String>,
        features: HashSet<String>,
        jobs: Option<u32>,
        load_average: Option<f64>,
    },
    SyncParams {
        repos: Vec<String>,
        quiet: bool,
    },
    CleanParams {
        packages: bool,
        distfiles: bool,
        packages_filter: String,
    },
    MaskParams {
        packages: Vec<String>,
    },
    UnmaskParams {
        packages: Vec<String>,
    },
}

/// A Portage-specific action with its parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortageAction {
    pub action_type: PortageActionType,
    pub params: ActionParams,
    pub priority: i32,
    pub timeout_ms: u64,
}

impl PortageAction {
    /// Create a new Portage action
    pub fn new(action_type: PortageActionType, params: ActionParams) -> Self {
        Self {
            action_type,
            params,
            priority: 5, // Default priority
            timeout_ms: 30000, // Default timeout
        }
    }

    /// Create a new Portage action with custom priority and timeout
    pub fn with_config(action_type: PortageActionType, params: ActionParams, priority: i32, timeout_ms: u64) -> Self {
        Self {
            action_type,
            params,
            priority,
            timeout_ms,
        }
    }

    /// Check if this action is safe to execute
    pub fn is_safe(&self) -> bool {
        match &self.params {
            ActionParams::InstallParams { pretend, .. } => *pretend,
            ActionParams::UninstallParams { .. } => false, // Uninstalls are inherently risky
            ActionParams::UpdateParams { .. } => true,
            ActionParams::SearchParams { .. } => true,
            ActionParams::InfoParams { .. } => true,
            ActionParams::DependsParams { .. } => true,
            ActionParams::EmergeParams { .. } => true,
            ActionParams::SyncParams { .. } => true,
            ActionParams::CleanParams { .. } => true,
            ActionParams::MaskParams { .. } => false, // Masking can break systems
            ActionParams::UnmaskParams { .. } => false, // Unmasking can be risky
        }
    }

    /// Get a description of this action
    pub fn description(&self) -> String {
        match (&self.action_type, &self.params) {
            (PortageActionType::Install, ActionParams::InstallParams { packages, .. }) => {
                format!("Install {} packages", packages.len())
            }
            (PortageActionType::Uninstall, ActionParams::UninstallParams { packages, .. }) => {
                format!("Uninstall {} packages", packages.len())
            }
            (PortageActionType::Update, ActionParams::UpdateParams { packages, .. }) => {
                format!("Update {} packages", packages.len())
            }
            (PortageActionType::Search, ActionParams::SearchParams { query, .. }) => {
                format!("Search for '{}'", query)
            }
            (PortageActionType::Info, ActionParams::InfoParams { packages, .. }) => {
                format!("Get info for {} packages", packages.len())
            }
            (PortageActionType::Depends, ActionParams::DependsParams { packages, .. }) => {
                format!("Get dependencies for {} packages", packages.len())
            }
            (PortageActionType::Emerge, ActionParams::EmergeParams { packages, .. }) => {
                format!("Emerge {} packages", packages.len())
            }
            (PortageActionType::Sync, ActionParams::SyncParams { repos, .. }) => {
                if repos.is_empty() {
                    "Sync all repositories".to_string()
                } else {
                    format!("Sync {} repositories", repos.len())
                }
            }
            (PortageActionType::Clean, ActionParams::CleanParams { .. }) => {
                "Clean package system".to_string()
            }
            (PortageActionType::Mask, ActionParams::MaskParams { packages, .. }) => {
                format!("Mask {} packages", packages.len())
            }
            (PortageActionType::Unmask, ActionParams::UnmaskParams { packages, .. }) => {
                format!("Unmask {} packages", packages.len())
            }
            (PortageActionType::Rdepends, _) => {
                "Get reverse dependencies".to_string()
            }
            _ => "Unknown action".to_string(),
        }
    }

    /// Get the estimated execution time for this action
    pub fn estimated_duration_ms(&self) -> u64 {
        match (&self.action_type, &self.params) {
            (PortageActionType::Install, ActionParams::InstallParams { packages, .. }) => {
                5000 + packages.len() as u64 * 2000 // 5s base + 2s per package
            }
            (PortageActionType::Uninstall, ActionParams::UninstallParams { packages, .. }) => {
                3000 + packages.len() as u64 * 1000 // 3s base + 1s per package
            }
            (PortageActionType::Update, ActionParams::UpdateParams { packages, .. }) => {
                10000 + packages.len() as u64 * 5000 // 10s base + 5s per package
            }
            (PortageActionType::Search, _) => 1000,
            (PortageActionType::Info, ActionParams::InfoParams { packages, .. }) => {
                500 + packages.len() as u64 * 100 // 500ms base + 100ms per package
            }
            (PortageActionType::Depends, ActionParams::DependsParams { packages, .. }) => {
                2000 + packages.len() as u64 * 500 // 2s base + 500ms per package
            }
            (PortageActionType::Emerge, ActionParams::EmergeParams { packages, .. }) => {
                15000 + packages.len() as u64 * 10000 // 15s base + 10s per package
            }
            (PortageActionType::Sync, ActionParams::SyncParams { repos, .. }) => {
                30000 + repos.len() as u64 * 5000 // 30s base + 5s per repo
            }
            (PortageActionType::Clean, _) => 5000,
            (PortageActionType::Mask, _) => 500,
            (PortageActionType::Unmask, _) => 500,
            (PortageActionType::Rdepends, _) => 1000,
            _ => 1000,
        }
    }

    /// Validate the action parameters
    pub fn validate(&self) -> Result<()> {
        match &self.params {
            ActionParams::InstallParams { packages, use_flags, .. } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation("Package list cannot be empty".to_string()));
                }
                for pkg in packages {
                    if pkg.trim().is_empty() {
                        return Err(PortCLError::Validation("Package name cannot be empty".to_string()));
                    }
                }
                // Validate USE flags format
                for flag in use_flags {
                    if !flag.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '+' || c == '-') {
                        return Err(PortCLError::Validation(format!("Invalid USE flag: {}", flag)));
                    }
                }
            }
            ActionParams::UninstallParams { packages, .. } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation("Package list cannot be empty".to_string()));
                }
            }
            ActionParams::UpdateParams { packages, backtrack, .. } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation("Package list cannot be empty".to_string()));
                }
                if *backtrack > 100 {
                    return Err(PortCLError::Validation("Backtrack value too high (max 100)".to_string()));
                }
            }
            ActionParams::SearchParams { query, .. } => {
                if query.trim().is_empty() {
                    return Err(PortCLError::Validation("Search query cannot be empty".to_string()));
                }
                if query.len() > 200 {
                    return Err(PortCLError::Validation("Search query too long (max 200 chars)".to_string()));
                }
            }
            ActionParams::InfoParams { packages, .. } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation("Package list cannot be empty".to_string()));
                }
            }
            ActionParams::DependsParams { packages, .. } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation("Package list cannot be empty".to_string()));
                }
            }
            ActionParams::EmergeParams { packages, jobs, load_average, .. } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation("Package list cannot be empty".to_string()));
                }
                if let Some(j) = jobs {
                    if *j < 1 || *j > 64 {
                        return Err(PortCLError::Validation("Jobs must be between 1 and 64".to_string()));
                    }
                }
                if let Some(load) = load_average {
                    if *load < 0.0 || *load > 100.0 {
                        return Err(PortCLError::Validation("Load average must be between 0.0 and 100.0".to_string()));
                    }
                }
            }
            ActionParams::SyncParams { repos, .. } => {
                for repo in repos {
                    if repo.trim().is_empty() {
                        return Err(PortCLError::Validation("Repository name cannot be empty".to_string()));
                    }
                }
            }
            ActionParams::CleanParams { packages_filter, .. } => {
                if !packages_filter.is_empty() && packages_filter.len() > 100 {
                    return Err(PortCLError::Validation("Package filter too long (max 100 chars)".to_string()));
                }
            }
            ActionParams::MaskParams { packages } | ActionParams::UnmaskParams { packages } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation("Package list cannot be empty".to_string()));
                }
            }
        }

        // Validate timeout
        if self.timeout_ms < 1000 {
            return Err(PortCLError::Validation("Timeout must be at least 1000ms".to_string()));
        }

        // Validate priority
        if self.priority < 1 || self.priority > 10 {
            return Err(PortCLError::Validation("Priority must be between 1 and 10".to_string()));
        }

        Ok(())
    }
}

/// Builder for creating Portage actions
pub struct PortageActionBuilder {
    action_type: Option<PortageActionType>,
    params: Option<ActionParams>,
    priority: Option<i32>,
    timeout_ms: Option<u64>,
}

impl PortageActionBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            action_type: None,
            params: None,
            priority: None,
            timeout_ms: None,
        }
    }

    /// Set the action type
    pub fn action_type(mut self, action_type: PortageActionType) -> Self {
        self.action_type = Some(action_type);
        self
    }

    /// Set the action parameters
    pub fn params(mut self, params: ActionParams) -> Self {
        self.params = Some(params);
        self
    }

    /// Set the priority
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set the timeout
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Build the Portage action
    pub fn build(self) -> Result<PortageAction> {
        let action_type = self.action_type.ok_or_else(|| {
            PortCLError::Validation("Action type is required".to_string())
        })?;

        let params = self.params.ok_or_else(|| {
            PortCLError::Validation("Action parameters are required".to_string())
        })?;

        let priority = self.priority.unwrap_or(5);
        let timeout_ms = self.timeout_ms.unwrap_or(30000);

        let action = PortageAction::with_config(action_type, params, priority, timeout_ms);
        action.validate()?;
        Ok(action)
    }
}

impl Default for PortageActionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_action_creation() {
        let packages = vec
!["sys-apps/portage".to_string()];
        let use_flags = ["-X".to_string(), "python".to_string()].into_iter().collect();

        let params = ActionParams::InstallParams {
            packages: packages.clone(),
            use_flags,
            verbose: false,
            pretend: true,
        };

        let action = PortageAction::new(PortageActionType::Install, params);

        assert!(action.is_safe()); // Safe because pretend=true
        assert!(action.description().contains("1 packages"));
        assert!(action.estimated_duration_ms() > 0);
    }

    #[test]
    fn test_action_validation() {
        // Valid action
        let params = ActionParams::SearchParams {
            query: "portage".to_string(),
            search_desc: true,
            regex: false,
        };
        let action = PortageAction::new(PortageActionType::Search, params);
        assert!(action.validate().is_ok());

        // Invalid action - empty package list
        let params = ActionParams::InstallParams {
            packages: vec
![],
            use_flags: HashSet::new(),
            verbose: false,
            pretend: false,
        };
        let action = PortageAction::new(PortageActionType::Install, params);
        assert!(action.validate().is_err());
    }

    #[test]
    fn test_builder_pattern() {
        let action = PortageActionBuilder::new()
            .action_type(PortageActionType::Sync)
            .params(ActionParams::SyncParams {
                repos: vec
!["gentoo".to_string()],
                quiet: true,
            })
            .priority(8)
            .timeout_ms(60000)
            .build()
            .unwrap();

        assert_eq!(action.action_type, PortageActionType::Sync);
        assert_eq!(action.priority, 8);
        assert_eq!(action.timeout_ms, 60000);
    }

    #[test]
    fn test_unsafe_actions() {
        // Uninstall is unsafe
        let params = ActionParams::UninstallParams {
            packages: vec
!["old-package".to_string()],
            clean_deps: true,
            config_files: "ask".to_string(),
        };
        let action = PortageAction::new(PortageActionType::Uninstall, params);
        assert!(!action.is_safe());

        // Mask is unsafe
        let params = ActionParams::MaskParams {
            packages: vec
!["risky-package".to_string()],
        };
        let action = PortageAction::new(PortageActionType::Mask, params);
        assert!(!action.is_safe());
    }
}