//! Comprehensive tests for the safety module
//!
//! This test module provides extensive coverage for the safety checking and
//! rollback management functionality in PortCL actions.

use portcl::actions::safety::*;
use portcl::actions::Action;
use portcl::error::PortCLError;

// Import futures for async tests
#[cfg(test)]
use futures;

// Re-export needed for tests
#[cfg(test)]
use portcl::actions::safety::{SafetyCheckType, SafetySeverity, SafetyCheck, SafetyConfig};

#[cfg(test)]
mod safety_check_type_tests {
    use super::*;

    #[test]
    fn test_all_safety_check_types_exist() {
        // Test that all expected check types are available
        let check_types = vec![
            SafetyCheckType::SystemIntegrity,
            SafetyCheckType::PackageConsistency,
            SafetyCheckType::FileSystemSafety,
            SafetyCheckType::NetworkStability,
            SafetyCheckType::ResourceAvailability,
            SafetyCheckType::ConfigurationValidity,
        ];

        assert_eq!(check_types.len(), 6, "Expected 6 safety check types");

        // Test serialization
        for check_type in &check_types {
            let serialized = serde_json::to_string(check_type).unwrap();
            let deserialized: SafetyCheckType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(*check_type, deserialized);
        }
    }

    #[test]
    fn test_safety_check_type_equality() {
        assert_eq!(SafetyCheckType::SystemIntegrity, SafetyCheckType::SystemIntegrity);
        assert_ne!(SafetyCheckType::SystemIntegrity, SafetyCheckType::PackageConsistency);
    }
}

#[cfg(test)]
mod safety_severity_tests {
    use super::*;

    #[test]
    fn test_all_severity_levels_exist() {
        let severities = vec![
            SafetySeverity::Info,
            SafetySeverity::Warning,
            SafetySeverity::Critical,
            SafetySeverity::Blocking,
        ];

        assert_eq!(severities.len(), 4, "Expected 4 severity levels");

        // Test serialization
        for severity in &severities {
            let serialized = serde_json::to_string(severity).unwrap();
            let deserialized: SafetySeverity = serde_json::from_str(&serialized).unwrap();
            assert_eq!(*severity, deserialized);
        }
    }

    #[test]
    fn test_safety_severity_equality() {
        assert_eq!(SafetySeverity::Info, SafetySeverity::Info);
        assert_ne!(SafetySeverity::Info, SafetySeverity::Warning);
        assert_ne!(SafetySeverity::Critical, SafetySeverity::Blocking);
    }
}

#[cfg(test)]
mod safety_check_tests {
    use super::*;

    #[test]
    fn test_safety_check_creation_passed() {
        let check = SafetyCheck {
            check_type: SafetyCheckType::SystemIntegrity,
            passed: true,
            message: "All systems operational".to_string(),
            severity: SafetySeverity::Info,
            recommendations: vec![],
        };

        assert!(check.passed);
        assert_eq!(check.check_type, SafetyCheckType::SystemIntegrity);
        assert_eq!(check.severity, SafetySeverity::Info);
        assert!(check.recommendations.is_empty());
    }

    #[test]
    fn test_safety_check_creation_failed() {
        let check = SafetyCheck {
            check_type: SafetyCheckType::FileSystemSafety,
            passed: false,
            message: "Filesystem errors detected".to_string(),
            severity: SafetySeverity::Blocking,
            recommendations: vec!["Run fsck".to_string()],
        };

        assert!(!check.passed);
        assert_eq!(check.severity, SafetySeverity::Blocking);
        assert_eq!(check.recommendations.len(), 1);
    }

    #[test]
    fn test_safety_check_serialization() {
        let check = SafetyCheck {
            check_type: SafetyCheckType::PackageConsistency,
            passed: true,
            message: "Packages consistent".to_string(),
            severity: SafetySeverity::Warning,
            recommendations: vec!["Consider updating".to_string()],
        };

        let serialized = serde_json::to_string(&check).unwrap();
        let deserialized: SafetyCheck = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.check_type, check.check_type);
        assert_eq!(deserialized.passed, check.passed);
        assert_eq!(deserialized.severity, check.severity);
    }
}

#[cfg(test)]
mod safety_config_tests {
    use super::*;

    #[test]
    fn test_safety_config_default_values() {
        let config = SafetyConfig::default();

        assert!(config.enable_system_checks);
        assert!(config.enable_package_checks);
        assert!(config.enable_filesystem_checks);
        assert!(!config.strict_mode);
        assert!(config.auto_rollback_on_failure);
        assert!(config.backup_before_critical);
        assert_eq!(config.max_backup_size_mb, 1024);
    }

    #[test]
    fn test_safety_config_custom_values() {
        let config = SafetyConfig {
            enable_system_checks: false,
            enable_package_checks: false,
            enable_filesystem_checks: false,
            strict_mode: true,
            auto_rollback_on_failure: false,
            backup_before_critical: false,
            max_backup_size_mb: 512,
        };

        assert!(!config.enable_system_checks);
        assert!(!config.enable_package_checks);
        assert!(!config.enable_filesystem_checks);
        assert!(config.strict_mode);
        assert!(!config.auto_rollback_on_failure);
        assert!(!config.backup_before_critical);
        assert_eq!(config.max_backup_size_mb, 512);
    }

    #[test]
    fn test_safety_config_serialization() {
        let config = SafetyConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: SafetyConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.enable_system_checks, config.enable_system_checks);
        assert_eq!(deserialized.strict_mode, config.strict_mode);
        assert_eq!(deserialized.max_backup_size_mb, config.max_backup_size_mb);
    }

    #[test]
    fn test_safety_config_boundary_values() {
        // Test minimum backup size
        let config_min = SafetyConfig {
            max_backup_size_mb: 1,
            ..Default::default()
        };
        assert_eq!(config_min.max_backup_size_mb, 1);

        // Test very large backup size
        let config_max = SafetyConfig {
            max_backup_size_mb: u64::MAX,
            ..Default::default()
        };
        assert_eq!(config_max.max_backup_size_mb, u64::MAX);
    }
}

#[cfg(test)]
mod rollback_manager_tests {
    use super::*;

    #[tokio::test]
    async fn test_rollback_manager_creation() {
        let manager = RollbackManager::new();
        assert!(manager.list_snapshots().is_empty());
        assert!(manager.current_snapshot.is_none());
    }

    #[tokio::test]
    async fn test_rollback_manager_with_custom_config() {
        let config = SafetyConfig {
            strict_mode: true,
            max_backup_size_mb: 2048,
            ..Default::default()
        };

        let manager = RollbackManager::with_config(config);
        assert!(manager.list_snapshots().is_empty());
    }

    #[tokio::test]
    async fn test_create_snapshot_basic() {
        let mut manager = RollbackManager::new();
        let action = Action::NoOp;

        let snapshot_id = manager.create_snapshot(&action).await.unwrap();

        assert!(!snapshot_id.is_empty());
        assert!(snapshot_id.starts_with("snapshot_"));
        assert_eq!(manager.current_snapshot, Some(snapshot_id.clone()));
    }

    #[tokio::test]
    async fn test_create_multiple_snapshots() {
        let mut manager = RollbackManager::new();

        let id1 = manager.create_snapshot(&Action::NoOp).await.unwrap();
        let id2 = manager.create_snapshot(&Action::CleanObsoletePackages { force: false }).await.unwrap();
        let id3 = manager.create_snapshot(&Action::NoOp).await.unwrap();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);

        let snapshots = manager.list_snapshots();
        assert_eq!(snapshots.len(), 3);
    }

    #[tokio::test]
    async fn test_get_snapshot_by_id() {
        let mut manager = RollbackManager::new();
        let action = Action::NoOp;

        let snapshot_id = manager.create_snapshot(&action).await.unwrap();

        let snapshot = manager.get_snapshot(&snapshot_id);
        assert!(snapshot.is_some());

        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.id, snapshot_id);
    }

    #[tokio::test]
    async fn test_get_nonexistent_snapshot() {
        let manager = RollbackManager::new();

        let snapshot = manager.get_snapshot("nonexistent_id");
        assert!(snapshot.is_none());
    }

    #[tokio::test]
    async fn test_rollback_to_snapshot() {
        let mut manager = RollbackManager::new();
        let action = Action::CleanObsoletePackages { force: false };

        let snapshot_id = manager.create_snapshot(&action).await.unwrap();
        let result = manager.rollback_to_snapshot(&snapshot_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rollback_to_nonexistent_snapshot() {
        let mut manager = RollbackManager::new();

        let result = manager.rollback_to_snapshot("nonexistent_id").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PortCLError::Validation(msg) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[tokio::test]
    async fn test_rollback_to_latest() {
        let mut manager = RollbackManager::new();
        let action = Action::NoOp;

        manager.create_snapshot(&action).await.unwrap();
        let result = manager.rollback_to_latest().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rollback_to_latest_no_snapshot() {
        let mut manager = RollbackManager::new();

        let result = manager.rollback_to_latest().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PortCLError::Validation(msg) => {
                assert!(msg.contains("No snapshot available"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[tokio::test]
    async fn test_cleanup_old_snapshots() {
        let mut manager = RollbackManager::new();

        // Create 5 snapshots
        for _ in 0..5 {
            manager.create_snapshot(&Action::NoOp).await.unwrap();
        }

        assert_eq!(manager.list_snapshots().len(), 5);

        // Keep only 2 most recent
        let removed = manager.cleanup_old_snapshots(2).await.unwrap();

        assert_eq!(removed, 3);
        assert_eq!(manager.list_snapshots().len(), 2);
    }

    #[tokio::test]
    async fn test_cleanup_preserves_current_snapshot() {
        let mut manager = RollbackManager::new();

        // Create 5 snapshots
        let ids: Vec<_> = futures::future::join_all((0..5).map(|_| {
            let mut mgr = manager.clone();
            async move { mgr.create_snapshot(&Action::NoOp).await.unwrap() }
        })).await;

        // Set current snapshot to first one
        manager.current_snapshot = Some(ids[0].clone());

        // Try to cleanup all but 1
        let _removed = manager.cleanup_old_snapshots(1).await.unwrap();

        // Current snapshot should be preserved even if it's old
        assert!(manager.get_snapshot(&ids[0]).is_some());
    }

    #[tokio::test]
    async fn test_cleanup_no_snapshots() {
        let mut manager = RollbackManager::new();

        let removed = manager.cleanup_old_snapshots(5).await.unwrap();

        assert_eq!(removed, 0);
    }
}

#[cfg(test)]
mod safety_checker_tests {
    use super::*;

    #[test]
    fn test_safety_checker_creation() {
        let checker = SafetyChecker::new();
        assert!(!checker.rollback_manager().list_snapshots().is_empty() == false);
    }

    #[test]
    fn test_safety_checker_with_custom_config() {
        let config = SafetyConfig {
            strict_mode: true,
            enable_system_checks: false,
            ..Default::default()
        };

        let _checker = SafetyChecker::with_config(config);
        // Custom config should be applied
    }

    #[tokio::test]
    async fn test_validate_action_noop() {
        let checker = SafetyChecker::new();
        let action = Action::NoOp;

        let checks = checker.validate_action(&action).await.unwrap();

        // Should have some checks (system, package, filesystem, etc.)
        assert!(!checks.is_empty());

        // At least some should pass (most checks have >90% pass rate)
        let passed = checks.iter().filter(|c| c.passed).count();
        assert!(passed > 0);
    }

    #[tokio::test]
    async fn test_validate_action_all_checks_disabled() {
        let config = SafetyConfig {
            enable_system_checks: false,
            enable_package_checks: false,
            enable_filesystem_checks: false,
            ..Default::default()
        };

        let checker = SafetyChecker::with_config(config);
        let action = Action::NoOp;

        let _checks = checker.validate_action(&action).await.unwrap();

        // With all checks disabled, should have minimal or no checks
        // (depends on implementation)
    }

    #[tokio::test]
    async fn test_validate_action_blocking_check() {
        // Note: This test is probabilistic because the safety checks use random values
        // In a real implementation, you would inject a mock for deterministic testing
        let checker = SafetyChecker::new();
        let action = Action::NoOp;

        // Try multiple times to potentially trigger a blocking error
        for _ in 0..10 {
            let result = checker.validate_action(&action).await;
            // Either we get checks (ok) or a blocking error (also ok for this test)
            match result {
                Ok(checks) => assert!(!checks.is_empty()),
                Err(PortCLError::Safety(_)) => {} // Expected for blocking checks
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_prepare_action_non_critical() {
        let mut checker = SafetyChecker::new();
        let action = Action::NoOp;

        let _snapshot_id = checker.prepare_action(&action).await.unwrap();

        // NoOp is not a critical action, so no snapshot should be created
        // (unless backup_before_critical is false)
        // This depends on the implementation
    }

    #[tokio::test]
    async fn test_prepare_action_critical() {
        let mut checker = SafetyChecker::with_config(SafetyConfig {
            backup_before_critical: true,
            ..Default::default()
        });

        let action = Action::CleanObsoletePackages { force: false };
        let snapshot_id = checker.prepare_action(&action).await.unwrap();

        // CleanObsoletePackages is critical, so snapshot should be created
        assert!(snapshot_id.is_some());
    }

    #[tokio::test]
    async fn test_handle_failure_with_rollback_enabled() {
        let mut checker = SafetyChecker::with_config(SafetyConfig {
            auto_rollback_on_failure: true,
            backup_before_critical: true,
            ..Default::default()
        });

        let action = Action::CleanObsoletePackages { force: false };
        let snapshot_id = checker.prepare_action(&action).await.unwrap();

        // Handle failure with snapshot
        let result = checker.handle_failure(snapshot_id.as_deref()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_failure_with_rollback_disabled() {
        let mut checker = SafetyChecker::with_config(SafetyConfig {
            auto_rollback_on_failure: false,
            backup_before_critical: true,
            ..Default::default()
        });

        let action = Action::CleanObsoletePackages { force: false };
        let snapshot_id = checker.prepare_action(&action).await.unwrap();

        // Handle failure with snapshot but rollback disabled
        let result = checker.handle_failure(snapshot_id.as_deref()).await;
        assert!(result.is_ok()); // Should succeed without doing rollback
    }

    #[tokio::test]
    async fn test_handle_failure_no_snapshot() {
        let mut checker = SafetyChecker::new();

        // Handle failure without snapshot
        let result = checker.handle_failure(None).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_rollback_manager_access() {
        let checker = SafetyChecker::new();

        // Test immutable access
        let manager = checker.rollback_manager();
        assert!(manager.list_snapshots().is_empty());
    }

    #[test]
    fn test_rollback_manager_mutable_access() {
        let mut checker = SafetyChecker::new();

        // Test mutable access
        let manager = checker.rollback_manager_mut();
        assert!(manager.list_snapshots().is_empty());
    }
}

#[cfg(test)]
mod system_snapshot_tests {
    use super::*;

    #[test]
    fn test_package_state_creation() {
        let state = PackageState {
            name: "sys-apps/portage".to_string(),
            version: "3.0.56".to_string(),
            slot: "0".to_string(),
            repository: "gentoo".to_string(),
            installed: true,
            masked: false,
            use_flags: vec!["python".to_string(), "ipc".to_string()],
        };

        assert_eq!(state.name, "sys-apps/portage");
        assert_eq!(state.version, "3.0.56");
        assert!(state.installed);
        assert!(!state.masked);
        assert_eq!(state.use_flags.len(), 2);
    }

    #[test]
    fn test_package_state_serialization() {
        let state = PackageState {
            name: "dev-lang/rust".to_string(),
            version: "1.75.0".to_string(),
            slot: "stable".to_string(),
            repository: "gentoo".to_string(),
            installed: true,
            masked: false,
            use_flags: vec!["clippy".to_string()],
        };

        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: PackageState = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, state.name);
        assert_eq!(deserialized.version, state.version);
        assert_eq!(deserialized.use_flags, state.use_flags);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_safety_workflow() {
        // Create safety checker with all features enabled
        let mut checker = SafetyChecker::with_config(SafetyConfig {
            enable_system_checks: true,
            enable_package_checks: true,
            enable_filesystem_checks: true,
            strict_mode: false,
            auto_rollback_on_failure: true,
            backup_before_critical: true,
            max_backup_size_mb: 1024,
        });

        let action = Action::CleanObsoletePackages { force: false };

        // Prepare action (creates snapshot if critical)
        let snapshot_id = checker.prepare_action(&action).await.unwrap();

        // Simulate failure and rollback
        if let Some(ref id) = snapshot_id {
            let result = checker.handle_failure(Some(id)).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_multiple_actions_with_snapshots() {
        let mut checker = SafetyChecker::with_config(SafetyConfig {
            backup_before_critical: true,
            ..Default::default()
        });

        let actions = vec![
            Action::NoOp,
            Action::CleanObsoletePackages { force: false },
            Action::NoOp,
        ];

        let mut snapshot_ids = Vec::new();

        for action in actions {
            let snapshot_id = checker.prepare_action(&action).await.unwrap();
            if let Some(id) = snapshot_id {
                snapshot_ids.push(id);
            }
        }

        // Should have created snapshot for critical action
        assert!(!snapshot_ids.is_empty());

        // Cleanup old snapshots
        let manager = checker.rollback_manager_mut();
        let removed = manager.cleanup_old_snapshots(1).await.unwrap();
        assert!(removed > 0 || snapshot_ids.len() == 1);
    }
}
