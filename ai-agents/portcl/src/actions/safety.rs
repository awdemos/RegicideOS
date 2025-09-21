//! Safety checking and rollback management for PortCL actions
//!
//! This module provides safety validation for actions and rollback capabilities
//! to recover from failed operations.

use crate::actions::Action;
use crate::error::{PortCLError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Types of safety checks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SafetyCheckType {
    SystemIntegrity,
    PackageConsistency,
    FileSystemSafety,
    NetworkStability,
    ResourceAvailability,
    ConfigurationValidity,
}

/// Result of a safety check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheck {
    pub check_type: SafetyCheckType,
    pub passed: bool,
    pub message: String,
    pub severity: SafetySeverity,
    pub recommendations: Vec<String>,
}

/// Severity levels for safety issues
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SafetySeverity {
    Info,
    Warning,
    Critical,
    Blocking,
}

/// Configuration for safety checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub enable_system_checks: bool,
    pub enable_package_checks: bool,
    pub enable_filesystem_checks: bool,
    pub strict_mode: bool,
    pub auto_rollback_on_failure: bool,
    pub backup_before_critical: bool,
    pub max_backup_size_mb: u64,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            enable_system_checks: true,
            enable_package_checks: true,
            enable_filesystem_checks: true,
            strict_mode: false,
            auto_rollback_on_failure: true,
            backup_before_critical: true,
            max_backup_size_mb: 1024, // 1GB
        }
    }
}

/// Snapshot of system state before an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: Action,
    pub package_database: HashMap<String, PackageState>,
    pub config_files: HashMap<PathBuf, String>,
    pub disk_usage: u64,
    pub memory_usage: u64,
    pub metadata: HashMap<String, String>,
}

/// State of a package in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageState {
    pub name: String,
    pub version: String,
    pub slot: String,
    pub repository: String,
    pub installed: bool,
    pub masked: bool,
    pub use_flags: Vec<String>,
}

/// Rollback manager for handling system state restoration
#[derive(Debug, Clone)]
pub struct RollbackManager {
    config: SafetyConfig,
    snapshots: HashMap<String, SystemSnapshot>,
    current_snapshot: Option<String>,
}

impl RollbackManager {
    /// Create a new rollback manager
    pub fn new() -> Self {
        Self::with_config(SafetyConfig::default())
    }

    /// Create a new rollback manager with custom configuration
    pub fn with_config(config: SafetyConfig) -> Self {
        Self {
            config,
            snapshots: HashMap::new(),
            current_snapshot: None,
        }
    }

    /// Create a system snapshot before executing an action
    pub async fn create_snapshot(&mut self, action: &Action) -> Result<String> {
        let snapshot_id = self.generate_snapshot_id();

        // Simulate creating a system snapshot
        let snapshot = SystemSnapshot {
            id: snapshot_id.clone(),
            timestamp: chrono::Utc::now(),
            action: action.clone(),
            package_database: self.get_current_package_database().await,
            config_files: self.backup_config_files().await,
            disk_usage: self.get_disk_usage().await,
            memory_usage: self.get_memory_usage().await,
            metadata: HashMap::new(),
        };

        self.snapshots.insert(snapshot_id.clone(), snapshot);
        self.current_snapshot = Some(snapshot_id.clone());

        Ok(snapshot_id)
    }

    /// Rollback to a specific snapshot
    pub async fn rollback_to_snapshot(&mut self, snapshot_id: &str) -> Result<()> {
        let snapshot = self.snapshots.get(snapshot_id).ok_or_else(|| {
            PortCLError::Validation(format!("Snapshot {} not found", snapshot_id))
        })?;

        // Simulate rollback process
        println!("Rolling back to snapshot: {}", snapshot_id);
        println!("Action to undo: {:?}", snapshot.action);

        // Restore package database
        self.restore_package_database(&snapshot.package_database).await?;

        // Restore config files
        self.restore_config_files(&snapshot.config_files).await?;

        // Update current snapshot
        self.current_snapshot = Some(snapshot_id.to_string());

        Ok(())
    }

    /// Rollback to the most recent snapshot
    pub async fn rollback_to_latest(&mut self) -> Result<()> {
        let latest_id = self.current_snapshot.clone().ok_or_else(|| {
            PortCLError::Validation("No snapshot available for rollback".to_string())
        })?;

        self.rollback_to_snapshot(&latest_id).await
    }

    /// Get a list of available snapshots
    pub fn list_snapshots(&self) -> Vec<&SystemSnapshot> {
        self.snapshots.values().collect()
    }

    /// Get a specific snapshot by ID
    pub fn get_snapshot(&self, snapshot_id: &str) -> Option<&SystemSnapshot> {
        self.snapshots.get(snapshot_id)
    }

    /// Remove old snapshots (keep only the most recent N)
    pub async fn cleanup_old_snapshots(&mut self, keep_count: usize) -> Result<u32> {
        let mut snapshot_ids: Vec<_> = self.snapshots.keys().cloned().collect();
        snapshot_ids.sort_by_key(|id| {
            self.snapshots.get(id).unwrap().timestamp
        });

        let to_remove = snapshot_ids.len().saturating_sub(keep_count);
        let mut removed = 0;

        for id in snapshot_ids.iter().take(to_remove) {
            if Some(id) != self.current_snapshot.as_ref() {
                self.snapshots.remove(id);
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Generate a unique snapshot ID
    fn generate_snapshot_id(&self) -> String {
        use uuid::Uuid;
        format!("snapshot_{}", Uuid::new_v4())
    }

    /// Simulate getting current package database
    async fn get_current_package_database(&self) -> HashMap<String, PackageState> {
        let mut packages = HashMap::new();

        // Simulate some installed packages
        packages.insert("sys-apps/portage".to_string(), PackageState {
            name: "sys-apps/portage".to_string(),
            version: "3.0.56".to_string(),
            slot: "0".to_string(),
            repository: "gentoo".to_string(),
            installed: true,
            masked: false,
            use_flags: vec
!["-X".to_string(), "python".to_string(), "ipc".to_string()],
        });

        packages.insert("dev-lang/rust".to_string(), PackageState {
            name: "dev-lang/rust".to_string(),
            version: "1.75.0".to_string(),
            slot: "stable".to_string(),
            repository: "gentoo".to_string(),
            installed: true,
            masked: false,
            use_flags: vec
!["clippy".to_string(), "rustfmt".to_string()],
        });

        packages
    }

    /// Simulate backing up config files
    async fn backup_config_files(&self) -> HashMap<PathBuf, String> {
        let mut configs = HashMap::new();

        // Simulate reading some config files
        configs.insert(
            PathBuf::from("/etc/portage/make.conf"),
            "# Generated by PortCL\nUSE=\"-X python ipc\"\nMAKEOPTS=\"-j4\"\n".to_string(),
        );

        configs.insert(
            PathBuf::from("/etc/portage/package.use"),
            "# Custom USE flags\ndev-lang/rust clippy rustfmt\n".to_string(),
        );

        configs
    }

    /// Simulate restoring package database
    async fn restore_package_database(&self, target_state: &HashMap<String, PackageState>) -> Result<()> {
        println!("Restoring package database with {} packages", target_state.len());
        // Simulate package restoration
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    /// Simulate restoring config files
    async fn restore_config_files(&self, config_files: &HashMap<PathBuf, String>) -> Result<()> {
        println!("Restoring {} configuration files", config_files.len());

        for (path, content) in config_files {
            // In a real implementation, this would write to the filesystem
            println!("Restoring: {}", path.display());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        Ok(())
    }

    /// Simulate getting disk usage
    async fn get_disk_usage(&self) -> u64 {
        // Simulate disk usage check
        50 * 1024 * 1024 * 1024 // 50GB
    }

    /// Simulate getting memory usage
    async fn get_memory_usage(&self) -> u64 {
        // Simulate memory usage check
        8 * 1024 * 1024 * 1024 // 8GB
    }
}

/// Safety checker for validating actions before execution
#[derive(Debug, Clone)]
pub struct SafetyChecker {
    config: SafetyConfig,
    rollback_manager: RollbackManager,
}

impl SafetyChecker {
    /// Create a new safety checker
    pub fn new() -> Self {
        Self::with_config(SafetyConfig::default())
    }

    /// Create a new safety checker with custom configuration
    pub fn with_config(config: SafetyConfig) -> Self {
        Self {
            config: config.clone(),
            rollback_manager: RollbackManager::with_config(config),
        }
    }

    /// Validate an action before execution
    pub async fn validate_action(&self, action: &Action) -> Result<Vec<SafetyCheck>> {
        let mut checks = Vec::new();

        if self.config.enable_system_checks {
            checks.push(self.check_system_integrity(action).await);
            checks.push(self.check_resource_availability(action).await);
        }

        if self.config.enable_package_checks {
            checks.push(self.check_package_consistency(action).await);
        }

        if self.config.enable_filesystem_checks {
            checks.push(self.check_filesystem_safety(action).await);
        }

        // Check if any blocking issues were found
        let blocking_issues = checks.iter().filter(|check|
            check.severity == SafetySeverity::Blocking && !check.passed
        ).count();

        if blocking_issues > 0 {
            return Err(PortCLError::Safety(format!(
                "Action blocked by {} safety check(s)",
                blocking_issues
            )));
        }

        Ok(checks)
    }

    /// Prepare for action execution (create snapshot if needed)
    pub async fn prepare_action(&mut self, action: &Action) -> Result<Option<String>> {
        let checks = self.validate_action(action).await?;

        // Check if this is a critical action that requires a backup
        let is_critical = self.is_critical_action(action) && self.config.backup_before_critical;

        if is_critical {
            let snapshot_id = self.rollback_manager.create_snapshot(action).await?;
            Ok(Some(snapshot_id))
        } else {
            Ok(None)
        }
    }

    /// Handle action failure with rollback if needed
    pub async fn handle_failure(&mut self, snapshot_id: Option<&str>) -> Result<()> {
        if let Some(id) = snapshot_id {
            if self.config.auto_rollback_on_failure {
                println!("Auto-rolling back due to action failure");
                self.rollback_manager.rollback_to_snapshot(id).await?;
            }
        }
        Ok(())
    }

    /// Check system integrity
    async fn check_system_integrity(&self, action: &Action) -> SafetyCheck {
        // Simulate system integrity check
        let passed = rand::random::<f64>() > 0.1; // 90% pass rate

        SafetyCheck {
            check_type: SafetyCheckType::SystemIntegrity,
            passed,
            message: if passed {
                "System integrity check passed".to_string()
            } else {
                "System integrity check failed".to_string()
            },
            severity: if passed { SafetySeverity::Info } else { SafetySeverity::Critical },
            recommendations: if !passed {
                vec
!["Check system logs for errors".to_string()]
            } else {
                vec
![]
            },
        }
    }

    /// Check package consistency
    async fn check_package_consistency(&self, action: &Action) -> SafetyCheck {
        // Simulate package consistency check
        let passed = rand::random::<f64>() > 0.05; // 95% pass rate

        SafetyCheck {
            check_type: SafetyCheckType::PackageConsistency,
            passed,
            message: if passed {
                "Package consistency check passed".to_string()
            } else {
                "Package database inconsistencies detected".to_string()
            },
            severity: if passed { SafetySeverity::Info } else { SafetySeverity::Warning },
            recommendations: if !passed {
                vec
!["Run 'emerge --sync' to update package database".to_string()]
            } else {
                vec
![]
            },
        }
    }

    /// Check filesystem safety
    async fn check_filesystem_safety(&self, action: &Action) -> SafetyCheck {
        // Simulate filesystem safety check
        let passed = rand::random::<f64>() > 0.02; // 98% pass rate

        SafetyCheck {
            check_type: SafetyCheckType::FileSystemSafety,
            passed,
            message: if passed {
                "Filesystem safety check passed".to_string()
            } else {
                "Filesystem may be unstable".to_string()
            },
            severity: if passed { SafetySeverity::Info } else { SafetySeverity::Blocking },
            recommendations: if !passed {
                vec
!["Check filesystem integrity with 'fsck'".to_string()]
            } else {
                vec
![]
            },
        }
    }

    /// Check resource availability
    async fn check_resource_availability(&self, action: &Action) -> SafetyCheck {
        // Simulate resource availability check
        let passed = rand::random::<f64>() > 0.1; // 90% pass rate

        SafetyCheck {
            check_type: SafetyCheckType::ResourceAvailability,
            passed,
            message: if passed {
                "Sufficient resources available".to_string()
            } else {
                "Insufficient system resources".to_string()
            },
            severity: if passed { SafetySeverity::Info } else { SafetySeverity::Warning },
            recommendations: if !passed {
                vec
!["Free up disk space or close memory-intensive applications".to_string()]
            } else {
                vec
![]
            },
        }
    }

    /// Check if an action is critical and requires a backup
    fn is_critical_action(&self, action: &Action) -> bool {
        matches!(action, Action::CleanObsoletePackages { .. })
    }

    /// Get the rollback manager
    pub fn rollback_manager(&self) -> &RollbackManager {
        &self.rollback_manager
    }

    /// Get mutable access to the rollback manager
    pub fn rollback_manager_mut(&mut self) -> &mut RollbackManager {
        &mut self.rollback_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safety_checker_creation() {
        let checker = SafetyChecker::new();
        assert!(!checker.config.strict_mode);
    }

    #[tokio::test]
    async fn test_action_validation() {
        let checker = SafetyChecker::new();
        let action = Action::NoOp;
        let checks = checker.validate_action(&action).await.unwrap();

        assert!(!checks.is_empty());
        // Most checks should pass
        let passed_checks = checks.iter().filter(|c| c.passed).count();
        assert!(passed_checks > 0);
    }

    #[tokio::test]
    async fn test_snapshot_creation() {
        let mut manager = RollbackManager::new();
        let action = Action::CleanObsoletePackages { force: false };

        let snapshot_id = manager.create_snapshot(&action).await.unwrap();

        assert!(!snapshot_id.is_empty());
        assert!(manager.current_snapshot == Some(snapshot_id.clone()));
        assert!(manager.get_snapshot(&snapshot_id).is_some());
    }

    #[tokio::test]
    async fn test_rollback() {
        let mut manager = RollbackManager::new();
        let action = Action::CleanObsoletePackages { force: false };

        let snapshot_id = manager.create_snapshot(&action).await.unwrap();

        // Simulate some changes
        assert!(manager.rollback_to_snapshot(&snapshot_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_critical_action_backup() {
        let mut checker = SafetyChecker::with_config(SafetyConfig {
            backup_before_critical: true,
            ..Default::default()
        });

        let action = Action::CleanObsoletePackages { force: false };
        let snapshot_id = checker.prepare_action(&action).await.unwrap();

        assert!(snapshot_id.is_some());
    }

    #[test]
    fn test_safety_config_default() {
        let config = SafetyConfig::default();
        assert!(config.enable_system_checks);
        assert!(config.auto_rollback_on_failure);
        assert_eq!(config.max_backup_size_mb, 1024);
    }
}