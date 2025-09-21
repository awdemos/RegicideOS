//! Action Executor for PortCL
//!
//! This module implements the action execution system that carries out
//! the actions selected by the reinforcement learning agent.

use crate::actions::Action;
use crate::error::{PortCLError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{Duration, Instant};

/// Configuration for the action executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    pub max_concurrent_actions: u32,
    pub action_timeout_ms: u64,
    pub retry_attempts: u32,
    pub safety_checks_enabled: bool,
    pub rollback_on_failure: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_actions: 5,
            action_timeout_ms: 30000,
            retry_attempts: 3,
            safety_checks_enabled: true,
            rollback_on_failure: true,
        }
    }
}

/// Result of an action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: Action,
    pub success: bool,
    pub duration_ms: u64,
    pub output: String,
    pub error: Option<String>,
    pub metrics: ExecutionMetrics,
}

/// Metrics collected during action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub io_operations: u32,
    pub network_calls: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Main action executor
#[derive(Debug, Clone)]
pub struct ActionExecutor {
    config: ExecutorConfig,
    active_actions: HashMap<String, Instant>,
}

impl ActionExecutor {
    /// Create a new action executor with default configuration
    pub fn new() -> Self {
        Self::with_config(ExecutorConfig::default())
    }

    /// Create a new action executor with custom configuration
    pub fn with_config(config: ExecutorConfig) -> Self {
        Self {
            config,
            active_actions: HashMap::new(),
        }
    }

    /// Execute an action asynchronously
    pub async fn execute(&mut self, action: Action) -> Result<ActionResult> {
        let start_time = Instant::now();
        let action_id = self.generate_action_id(&action);

        // Check if we can execute this action
        self.validate_action(&action).await?;

        // Record the start time
        self.active_actions.insert(action_id.clone(), start_time);

        // Execute the action with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(self.config.action_timeout_ms),
            self.execute_action_internal(action.clone()),
        )
        .await;

        // Clean up active action tracking
        self.active_actions.remove(&action_id);

        match result {
            Ok(Ok(action_result)) => Ok(action_result),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(PortCLError::Timeout(format!(
                "Action timed out after {}ms",
                self.config.action_timeout_ms
            ))),
        }
    }

    /// Internal action execution logic
    async fn execute_action_internal(&self, action: Action) -> Result<ActionResult> {
        let start_time = Instant::now();

        // Simulate action execution with realistic timing
        let execution_time_ms = match &action {
            Action::NoOp => 10,
            Action::AdjustParallelism { .. } => 50,
            Action::OptimizeBuildOrder { package_list } => 100 + package_list.len() as u64 * 5,
            Action::ScheduleOperation { .. } => 20,
            Action::PreFetchDependencies { packages } => 200 + packages.len() as u64 * 10,
            Action::CleanObsoletePackages { .. } => 500,
        };

        tokio::time::sleep(Duration::from_millis(execution_time_ms)).await;

        // Simulate action results
        let (success, output, error) = self.simulate_action_result(&action).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(ActionResult {
            action: action.clone(),
            success,
            duration_ms,
            output,
            error,
            metrics: ExecutionMetrics {
                memory_usage_mb: 10.0 + (duration_ms as f64 * 0.01),
                cpu_usage_percent: 5.0 + (duration_ms as f64 * 0.05),
                io_operations: match &action {
                    Action::CleanObsoletePackages { .. } => 50,
                    Action::OptimizeBuildOrder { package_list } => package_list.len() as u32 * 2,
                    _ => 5,
                },
                network_calls: match &action {
                    Action::PreFetchDependencies { .. } => 10,
                    _ => 0,
                },
                timestamp: chrono::Utc::now(),
            },
        })
    }

    /// Validate that an action can be executed
    async fn validate_action(&self, action: &Action) -> Result<()> {
        // Check if we're at the concurrent action limit
        if self.active_actions.len() >= self.config.max_concurrent_actions as usize {
            return Err(PortCLError::Resource(format!(
                "Maximum concurrent actions ({}) reached",
                self.config.max_concurrent_actions
            )));
        }

        // Validate action-specific constraints
        match action {
            Action::AdjustParallelism { jobs } => {
                if *jobs < 1 || *jobs > 32 {
                    return Err(PortCLError::Validation(format!(
                        "Parallel jobs must be between 1 and 32, got {}",
                        jobs
                    )));
                }
            }
            Action::OptimizeBuildOrder { package_list } => {
                if package_list.is_empty() {
                    return Err(PortCLError::Validation(
                        "Package list cannot be empty".to_string(),
                    ));
                }
                if package_list.len() > 1000 {
                    return Err(PortCLError::Validation(
                        "Package list too large (max 1000)".to_string(),
                    ));
                }
            }
            Action::ScheduleOperation { delay_seconds } => {
                if *delay_seconds > 3600 {
                    return Err(PortCLError::Validation(
                        "Delay cannot exceed 1 hour".to_string(),
                    ));
                }
            }
            Action::PreFetchDependencies { packages } => {
                if packages.is_empty() {
                    return Err(PortCLError::Validation(
                        "Packages list cannot be empty".to_string(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Simulate action execution results
    async fn simulate_action_result(&self, action: &Action) -> (bool, String, Option<String>) {
        // Simulate occasional failures (5% failure rate)
        let success = rand::random::<f64>() > 0.05;

        if !success {
            return (
                false,
                String::new(),
                Some(format!("Simulated failure for action: {:?}", action)),
            );
        }

        let output = match action {
            Action::NoOp => "No operation performed".to_string(),
            Action::AdjustParallelism { jobs } => format!("Parallel jobs adjusted to {}", jobs),
            Action::OptimizeBuildOrder { package_list } => {
                format!("Build order optimized for {} packages", package_list.len())
            }
            Action::ScheduleOperation { delay_seconds } => {
                format!("Operation scheduled with {}s delay", delay_seconds)
            }
            Action::PreFetchDependencies { packages } => {
                format!("Dependencies pre-fetched for {} packages", packages.len())
            }
            Action::CleanObsoletePackages { force } => {
                format!("Cleaned obsolete packages (forced: {})", force)
            }
        };

        (true, output, None)
    }

    /// Generate a unique ID for an action
    fn generate_action_id(&self, action: &Action) -> String {
        use uuid::Uuid;
        format!("{}_{}", action.action_type(), Uuid::new_v4())
    }

    /// Get the number of currently active actions
    pub fn active_action_count(&self) -> usize {
        self.active_actions.len()
    }

    /// Check if a specific action type is currently running
    pub fn is_action_type_active(&self, action_type: &str) -> bool {
        self.active_actions.keys().any(|id| id.starts_with(action_type))
    }

    /// Get executor configuration
    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }

    /// Update executor configuration
    pub fn update_config(&mut self, config: ExecutorConfig) {
        self.config = config;
    }

    /// Cancel all active actions
    pub async fn cancel_all_actions(&mut self) -> Result<u32> {
        let cancelled_count = self.active_actions.len() as u32;
        self.active_actions.clear();
        Ok(cancelled_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = ActionExecutor::new();
        assert_eq!(executor.active_action_count(), 0);
    }

    #[tokio::test]
    async fn test_noop_execution() {
        let mut executor = ActionExecutor::new();
        let result = executor.execute(Action::NoOp).await.unwrap();

        assert!(result.success);
        assert_eq!(result.action, Action::NoOp);
        assert!(result.duration_ms > 0);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_adjust_parallelism() {
        let mut executor = ActionExecutor::new();
        let result = executor.execute(Action::AdjustParallelism { jobs: 4 }).await.unwrap();

        assert!(result.success);
        assert_eq!(result.action, Action::AdjustParallelism { jobs: 4 });
        assert!(result.output.contains("4"));
    }

    #[tokio::test]
    async fn test_invalid_parallelism() {
        let mut executor = ActionExecutor::new();
        let result = executor.execute(Action::AdjustParallelism { jobs: 0 }).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PortCLError::Validation(msg) => assert!(msg.contains("between 1 and 32")),
            _ => panic!("Expected validation error"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_action_limit() {
        let config = ExecutorConfig {
            max_concurrent_actions: 1,
            ..Default::default()
        };
        let mut executor = ActionExecutor::with_config(config);

        // Start a long-running action
        let long_action = Action::CleanObsoletePackages { force: false };
        let handle = tokio::spawn(async move {
            executor.execute(long_action).await
        });

        // Give it time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Try to start another action
        let mut executor2 = ActionExecutor::with_config(config);
        let result = executor2.execute(Action::NoOp).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PortCLError::Resource(msg) => assert!(msg.contains("Maximum concurrent actions")),
            _ => panic!("Expected resource error"),
        }

        // Clean up
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_action_timeout() {
        let config = ExecutorConfig {
            action_timeout_ms: 50,
            ..Default::default()
        };
        let mut executor = ActionExecutor::with_config(config);

        // This action normally takes longer than 50ms
        let result = executor.execute(Action::CleanObsoletePackages { force: false }).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PortCLError::Timeout(msg) => assert!(msg.contains("timed out")),
            _ => panic!("Expected timeout error"),
        }
    }
}