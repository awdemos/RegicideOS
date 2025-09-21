//! Mock ActionExecutor for testing PortCL action execution
//!
//! This module provides a mock implementation of ActionExecutor that
//! simulates Portage action operations without requiring actual system calls.

use crate::fixtures::mock_data::*;
use crate::error::PortCLError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

/// Mock execution result that mirrors the real ActionResult
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MockActionResult {
    pub action_id: String,
    pub action_type: String,
    pub status: ExecutionStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub output: String,
    pub error: Option<String>,
    pub metrics: ExecutionMetrics,
    pub changes: Vec<SystemChange>,
}

/// Execution status for actions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

/// Execution metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionMetrics {
    pub cpu_time_ms: u64,
    pub memory_used_bytes: u64,
    pub disk_io_bytes: u64,
    pub network_io_bytes: u64,
    pub packages_processed: u32,
    pub operations_performed: u32,
}

/// System change记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemChange {
    pub change_type: ChangeType,
    pub target: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Type of system change
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ChangeType {
    PackageInstalled,
    PackageRemoved,
    PackageUpdated,
    ConfigurationModified,
    FileCreated,
    FileModified,
    FileRemoved,
    ServiceStarted,
    ServiceStopped,
    ServiceRestarted,
}

/// Mock execution state
#[derive(Debug, Clone)]
pub struct MockExecutionState {
    pub total_actions: u32,
    pub successful_actions: u32,
    pub failed_actions: u32,
    pub cancelled_actions: u32,
    pub average_duration_ms: u64,
    pub total_cpu_time_ms: u64,
    pub peak_memory_usage: u64,
    pub last_execution_time: Option<DateTime<Utc>>,
}

/// Mock ActionExecutor for testing
#[derive(Debug, Clone)]
pub struct MockActionExecutor {
    config: MockActionConfig,
    state: Arc<RwLock<MockExecutionState>>,
    execution_history: Arc<Mutex<Vec<MockActionResult>>>,
    active_executions: Arc<Mutex<HashMap<String, bool>>>,
    error_injection: Arc<RwLock<HashMap<String, bool>>>,
    delay_injection: Arc<RwLock<HashMap<String, u64>>>,
    dry_run_mode: Arc<RwLock<bool>>,
}

impl MockActionExecutor {
    /// Create a new mock ActionExecutor with default configuration
    pub fn new(config: MockActionConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(MockExecutionState::default())),
            execution_history: Arc::new(Mutex::new(Vec::new())),
            active_executions: Arc::new(Mutex::new(HashMap::new())),
            error_injection: Arc::new(RwLock::new(HashMap::new())),
            delay_injection: Arc::new(RwLock::new(HashMap::new())),
            dry_run_mode: Arc::new(RwLock::new(config.enable_dry_run)),
        }
    }

    /// Create a mock executor with predefined execution history
    pub fn with_history(config: MockActionConfig, history: Vec<MockActionResult>) -> Self {
        let executor = Self::new(config);
        *executor.execution_history.blocking_lock() = history;
        executor
    }

    /// Inject an error for a specific action type
    pub fn inject_error(&self, action_type: &str) {
        let mut errors = self.error_injection.write().unwrap();
        errors.insert(action_type.to_string(), true);
    }

    /// Inject a delay for a specific action type (in milliseconds)
    pub fn inject_delay(&self, action_type: &str, delay_ms: u64) {
        let mut delays = self.delay_injection.write().unwrap();
        delays.insert(action_type.to_string(), delay_ms);
    }

    /// Clear all injected errors and delays
    pub fn clear_injections(&self) {
        let mut errors = self.error_injection.write().unwrap();
        let mut delays = self.delay_injection.write().unwrap();
        errors.clear();
        delays.clear();
    }

    /// Set dry run mode
    pub fn set_dry_run(&self, dry_run: bool) {
        let mut mode = self.dry_run_mode.write().unwrap();
        *mode = dry_run;
    }

    /// Get current execution state
    pub fn get_state(&self) -> MockExecutionState {
        self.state.read().unwrap().clone()
    }

    /// Get execution history
    pub async fn get_history(&self) -> Vec<MockActionResult> {
        self.execution_history.lock().await.clone()
    }

    /// Get active executions
    pub async fn get_active_executions(&self) -> Vec<String> {
        let active = self.active_executions.lock().await;
        active.iter().filter(|(_, &active)| active).map(|(id, _)| id.clone()).collect()
    }

    /// Cancel an active execution
    pub async fn cancel_execution(&self, action_id: &str) -> Result<(), PortCLError> {
        let mut active = self.active_executions.lock().await;
        if let Some(true) = active.get(action_id) {
            active.insert(action_id.to_string(), false);

            // Update history to mark as cancelled
            let mut history = self.execution_history.lock().await;
            if let Some(result) = history.iter_mut().find(|r| r.action_id == action_id) {
                result.status = ExecutionStatus::Cancelled;
                result.end_time = Utc::now();
            }

            // Update state
            let mut state = self.state.write().unwrap();
            state.cancelled_actions += 1;

            Ok(())
        } else {
            Err(PortCLError::Validation(format!("Action {} is not active or does not exist", action_id)))
        }
    }

    /// Check if an action should fail based on injected errors
    fn should_fail(&self, action_type: &str) -> bool {
        let errors = self.error_injection.read().unwrap();
        errors.get(action_type).copied().unwrap_or(false)
    }

    /// Get delay for an action type based on injected delays
    fn get_delay(&self, action_type: &str) -> u64 {
        let delays = self.delay_injection.read().unwrap();
        delays.get(action_type).copied().unwrap_or(0)
    }

    /// Simulate async delay
    async fn simulate_delay(&self, action_type: &str) {
        let delay = self.get_delay(action_type);
        if delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }
    }

    /// Generate mock execution result for an action
    async fn execute_mock_action(&self, action: MockAction) -> Result<MockActionResult, PortCLError> {
        let action_id = uuid::Uuid::new_v4().to_string();
        let start_time = Utc::now();

        // Register as active execution
        {
            let mut active = self.active_executions.lock().await;
            active.insert(action_id.clone(), true);
        }

        // Simulate execution delay
        let action_type = action.action_type();
        self.simulate_delay(&format!("{:?}", action_type)).await;

        let result = if self.should_fail(&format!("{:?}", action_type)) {
            // Simulate failure
            MockActionResult {
                action_id: action_id.clone(),
                action_type: format!("{:?}", action_type),
                status: ExecutionStatus::Failed,
                start_time,
                end_time: Utc::now(),
                duration_ms: (Utc::now() - start_time).num_milliseconds() as u64,
                output: String::new(),
                error: Some(format!("Mock execution failed for action: {:?}", action_type)),
                metrics: ExecutionMetrics {
                    cpu_time_ms: 100,
                    memory_used_bytes: 1024 * 1024,
                    disk_io_bytes: 512 * 1024,
                    network_io_bytes: 0,
                    packages_processed: 0,
                    operations_performed: 1,
                },
                changes: Vec::new(),
            }
        } else {
            // Simulate success
            let (output, changes) = self.generate_action_output(&action).await;
            MockActionResult {
                action_id: action_id.clone(),
                action_type: format!("{:?}", action_type),
                status: ExecutionStatus::Completed,
                start_time,
                end_time: Utc::now(),
                duration_ms: (Utc::now() - start_time).num_milliseconds() as u64,
                output,
                error: None,
                metrics: ExecutionMetrics {
                    cpu_time_ms: 50,
                    memory_used_bytes: 512 * 1024,
                    disk_io_bytes: 256 * 1024,
                    network_io_bytes: 1024,
                    packages_processed: match action {
                        MockAction::PreFetchDependencies { .. } => 2,
                        MockAction::CleanObsoletePackages { .. } => 3,
                        _ => 1,
                    },
                    operations_performed: 1,
                },
                changes,
            }
        };

        // Update state and history
        {
            let mut active = self.active_executions.lock().await;
            active.remove(&action_id);
        }

        {
            let mut history = self.execution_history.lock().await;
            history.push(result.clone());
        }

        {
            let mut state = self.state.write().unwrap();
            state.total_actions += 1;
            match result.status {
                ExecutionStatus::Completed => state.successful_actions += 1,
                ExecutionStatus::Failed => state.failed_actions += 1,
                ExecutionStatus::Cancelled => state.cancelled_actions += 1,
                _ => {}
            }
            state.average_duration_ms =
                (state.average_duration_ms * (state.total_actions - 1) + result.duration_ms) / state.total_actions;
            state.total_cpu_time_ms += result.metrics.cpu_time_ms;
            state.peak_memory_usage = state.peak_memory_usage.max(result.metrics.memory_used_bytes);
            state.last_execution_time = Some(Utc::now());
        }

        Ok(result)
    }

    /// Generate mock output and changes for different action types
    async fn generate_action_output(&self, action: &MockAction) -> (String, Vec<SystemChange>) {
        let dry_run = *self.dry_run_mode.read().unwrap();
        let prefix = if dry_run { "[DRY RUN] " } else { "" };

        match action {
            MockAction::NoOp => (
                format!("{}No operation performed successfully", prefix),
                Vec::new(),
            ),
            MockAction::AdjustParallelism { jobs } => (
                format!("{}Adjusted compilation parallelism to {} jobs", prefix, jobs),
                vec![SystemChange {
                    change_type: ChangeType::ConfigurationModified,
                    target: "/etc/portage/make.conf".to_string(),
                    old_value: Some("MAKEOPTS=\"-j2\"".to_string()),
                    new_value: Some(format!("MAKEOPTS=\"-j{}\"", jobs)),
                    timestamp: Utc::now(),
                }],
            ),
            MockAction::OptimizeBuildOrder { package_list } => (
                format!("{}Optimized build order for {} packages: {}", prefix, package_list.len(), package_list.join(", ")),
                if !dry_run {
                    package_list.iter().enumerate().map(|(i, pkg)| SystemChange {
                        change_type: ChangeType::FileModified,
                        target: format!("/var/cache/portage/build-order-{}", i),
                        old_value: None,
                        new_value: Some(format!("Optimized order: {}", pkg)),
                        timestamp: Utc::now(),
                    }).collect()
                } else {
                    Vec::new()
                },
            ),
            MockAction::ScheduleOperation { delay_seconds } => (
                format!("{}Scheduled operation with {}s delay", prefix, delay_seconds),
                vec![SystemChange {
                    change_type: ChangeType::FileCreated,
                    target: "/var/lib/portcl/scheduled-operations".to_string(),
                    old_value: None,
                    new_value: Some(format!("Scheduled operation at {}s delay", delay_seconds)),
                    timestamp: Utc::now(),
                }],
            ),
            MockAction::PreFetchDependencies { packages } => (
                format!("{}Pre-fetched dependencies for {} packages: {}", prefix, packages.len(), packages.join(", ")),
                if !dry_run {
                    packages.iter().map(|pkg| SystemChange {
                        change_type: ChangeType::FileCreated,
                        target: format!("/var/cache/portage/distfiles/{}-dep.tar.gz", pkg),
                        old_value: None,
                        new_value: Some(format!("Downloaded dependency: {}", pkg)),
                        timestamp: Utc::now(),
                    }).collect()
                } else {
                    Vec::new()
                },
            ),
            MockAction::CleanObsoletePackages { force } => (
                format!("{}Cleaned obsolete packages (force: {})", prefix, force),
                vec![SystemChange {
                    change_type: ChangeType::FileRemoved,
                    target: "/var/cache/portage/distfiles/obsolete-package.tar.gz".to_string(),
                    old_value: Some("obsolete-package.tar.gz".to_string()),
                    new_value: None,
                    timestamp: Utc::now(),
                }],
            ),
        }
    }
}

#[async_trait]
pub trait MockActionExecutorTrait {
    async fn execute(&self, action: MockAction) -> Result<MockActionResult, PortCLError>;
    async fn execute_batch(&self, actions: Vec<MockAction>) -> Vec<Result<MockActionResult, PortCLError>>;
    async fn get_execution_stats(&self) -> MockExecutionState;
    async fn validate_action(&self, action: &MockAction) -> Result<(), PortCLError>;
    async fn rollback_action(&self, action_id: &str) -> Result<(), PortCLError>;
}

#[async_trait]
impl MockActionExecutorTrait for MockActionExecutor {
    async fn execute(&self, action: MockAction) -> Result<MockActionResult, PortCLError> {
        // Check if action is safe to execute
        if self.config.safe_actions_only && !action.is_safe() {
            return Err(PortCLError::Validation(format!("Action {:?} is not marked as safe", action.action_type())));
        }

        // Validate action parameters
        self.validate_action(&action).await?;

        // Check if we can execute concurrently
        let active_count = self.get_active_executions().await.len();
        if active_count >= self.config.max_concurrent_actions {
            return Err(PortCLError::Validation("Maximum concurrent actions limit reached".to_string()));
        }

        // Execute the action
        self.execute_mock_action(action).await
    }

    async fn execute_batch(&self, actions: Vec<MockAction>) -> Vec<Result<MockActionResult, PortCLError>> {
        let mut results = Vec::new();

        // Execute actions sequentially for simplicity, but could be made parallel
        for action in actions {
            let result = self.execute(action).await;
            results.push(result);

            // Check if we should stop on failure
            if self.config.rollback_enabled && results.last().map_or(false, |r| r.is_err()) {
                // Rollback successful actions
                for (i, result) in results.iter().enumerate() {
                    if let Ok(ref action_result) = result {
                        if let Err(_) = self.rollback_action(&action_result.action_id).await {
                            // Log rollback failure but continue
                        }
                    }
                }
                break;
            }
        }

        results
    }

    async fn get_execution_stats(&self) -> MockExecutionState {
        self.get_state()
    }

    async fn validate_action(&self, action: &MockAction) -> Result<(), PortCLError> {
        match action {
            MockAction::AdjustParallelism { jobs } => {
                if *jobs == 0 || *jobs > 128 {
                    return Err(PortCLError::Validation("Parallel jobs must be between 1 and 128".to_string()));
                }
            },
            MockAction::ScheduleOperation { delay_seconds } => {
                if *delay_seconds > 86400 { // 24 hours
                    return Err(PortCLError::Validation("Schedule delay cannot exceed 24 hours".to_string()));
                }
            },
            MockAction::OptimizeBuildOrder { package_list } => {
                if package_list.is_empty() {
                    return Err(PortCLError::Validation("Package list cannot be empty".to_string()));
                }
                if package_list.len() > 1000 {
                    return Err(PortCLError::Validation("Package list cannot exceed 1000 packages".to_string()));
                }
            },
            MockAction::PreFetchDependencies { packages } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation("Packages list cannot be empty".to_string()));
                }
            },
            _ => {},
        }
        Ok(())
    }

    async fn rollback_action(&self, action_id: &str) -> Result<(), PortCLError> {
        let mut history = self.execution_history.lock().await;

        if let Some(result) = history.iter().find(|r| r.action_id == action_id) {
            if result.status != ExecutionStatus::Completed {
                return Err(PortCLError::Validation("Cannot rollback incomplete action".to_string()));
            }

            // Simulate rollback by creating a rollback entry
            let rollback_result = MockActionResult {
                action_id: format!("{}-rollback", action_id),
                action_type: format!("{}-rollback", result.action_type),
                status: ExecutionStatus::Completed,
                start_time: Utc::now(),
                end_time: Utc::now(),
                duration_ms: 50,
                output: format!("Rolled back action: {}", action_id),
                error: None,
                metrics: ExecutionMetrics {
                    cpu_time_ms: 25,
                    memory_used_bytes: 256 * 1024,
                    disk_io_bytes: 128 * 1024,
                    network_io_bytes: 0,
                    packages_processed: 0,
                    operations_performed: 1,
                },
                changes: result.changes.iter().map(|change| SystemChange {
                    change_type: match change.change_type {
                        ChangeType::PackageInstalled => ChangeType::PackageRemoved,
                        ChangeType::PackageRemoved => ChangeType::PackageInstalled,
                        ChangeType::ConfigurationModified => ChangeType::ConfigurationModified,
                        _ => ChangeType::FileModified,
                    },
                    target: change.target.clone(),
                    old_value: change.new_value.clone(),
                    new_value: change.old_value.clone(),
                    timestamp: Utc::now(),
                }).collect(),
            };

            history.push(rollback_result);
            Ok(())
        } else {
            Err(PortCLError::NotFound(format!("Action {} not found in execution history", action_id)))
        }
    }
}

impl Default for MockExecutionState {
    fn default() -> Self {
        Self {
            total_actions: 0,
            successful_actions: 0,
            failed_actions: 0,
            cancelled_actions: 0,
            average_duration_ms: 0,
            total_cpu_time_ms: 0,
            peak_memory_usage: 0,
            last_execution_time: None,
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            cpu_time_ms: 0,
            memory_used_bytes: 0,
            disk_io_bytes: 0,
            network_io_bytes: 0,
            packages_processed: 0,
            operations_performed: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_executor_creation() {
        let config = MockActionConfig::default();
        let executor = MockActionExecutor::new(config);

        let state = executor.get_state();
        assert_eq!(state.total_actions, 0);
    }

    #[tokio::test]
    async fn test_execute_noop() {
        let config = MockActionConfig::default();
        let executor = MockActionExecutor::new(config);

        let result = executor.execute(MockAction::NoOp).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Completed);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_error_injection() {
        let config = MockActionConfig::default();
        let executor = MockActionExecutor::new(config);

        executor.inject_error("NoOp");

        let result = executor.execute(MockAction::NoOp).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delay_injection() {
        let config = MockActionConfig::default();
        let executor = MockActionExecutor::new(config);

        executor.inject_delay("NoOp", 100);

        let start = std::time::Instant::now();
        let _result = executor.execute(MockAction::NoOp).await;
        let duration = start.elapsed();

        assert!(duration >= std::time::Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_action_validation() {
        let config = MockActionConfig::default();
        let executor = MockActionExecutor::new(config);

        // Valid action
        let valid_action = MockAction::AdjustParallelism { jobs: 4 };
        assert!(executor.validate_action(&valid_action).await.is_ok());

        // Invalid action
        let invalid_action = MockAction::AdjustParallelism { jobs: 0 };
        assert!(executor.validate_action(&invalid_action).await.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_execution_limit() {
        let mut config = MockActionConfig::default();
        config.max_concurrent_actions = 1;
        let executor = MockActionExecutor::new(config);

        // Start a long-running action
        executor.inject_delay("NoOp", 1000);
        let handle1 = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(MockAction::NoOp).await }
        });

        // Give it time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Try to execute another action - should fail due to concurrency limit
        let handle2 = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(MockAction::NoOp).await }
        });

        let result2 = handle2.await.unwrap();
        assert!(result2.is_err());

        // First action should succeed
        let result1 = handle1.await.unwrap();
        assert!(result1.is_ok());
    }

    #[tokio::test]
    async fn test_rollback() {
        let config = MockActionConfig::default();
        let executor = MockActionExecutor::new(config);

        // Execute an action
        let result = executor.execute(MockAction::AdjustParallelism { jobs: 8 }).await.unwrap();
        let action_id = result.action_id.clone();

        // Rollback the action
        let rollback_result = executor.rollback_action(&action_id).await;
        assert!(rollback_result.is_ok());

        // Check that rollback entry was created
        let history = executor.get_history().await;
        assert!(history.iter().any(|r| r.action_id == format!("{}-rollback", action_id)));
    }
}