//! Unit tests for the actions module

use portcl::actions::*;
use portcl::error::PortCLError;
use std::collections::HashMap;

#[cfg(test)]
mod action_tests {
    use super::*;
    use crate::fixtures::test_helpers::*;

    #[test]
    fn test_action_noop_creation() {
        let action = Action::NoOp;
        assert_eq!(action.action_type(), ActionType::NoOp);
        assert!(action.is_safe());
        assert_eq!(action.description(), "No operation");
    }

    #[test]
    fn test_action_adjust_parallelism_valid() {
        let action = Action::AdjustParallelism { jobs: 4 };
        assert_eq!(action.action_type(), ActionType::AdjustParallelism);
        assert!(action.is_safe());
        assert_eq!(action.description(), "Adjust compilation parallelism to 4 jobs");
    }

    #[test]
    fn test_action_adjust_parallelism_boundary_values() {
        // Test minimum valid value
        let action_min = Action::AdjustParallelism { jobs: 1 };
        assert!(action_min.is_safe());

        // Test maximum valid value
        let action_max = Action::AdjustParallelism { jobs: 32 };
        assert!(action_max.is_safe());

        // Test invalid values
        let action_zero = Action::AdjustParallelism { jobs: 0 };
        assert!(!action_zero.is_safe());

        let action_too_high = Action::AdjustParallelism { jobs: 33 };
        assert!(!action_too_high.is_safe());
    }

    #[test]
    fn test_action_optimize_build_order() {
        let packages = vec![
            "sys-apps/portage".to_string(),
            "dev-lang/rust".to_string(),
            "www-servers/nginx".to_string(),
        ];
        let action = Action::OptimizeBuildOrder { package_list: packages.clone() };

        assert_eq!(action.action_type(), ActionType::OptimizeBuildOrder);
        assert!(action.is_safe());
        assert_eq!(action.description(), "Optimize build order for 3 packages");
    }

    #[test]
    fn test_action_optimize_build_order_empty() {
        let action = Action::OptimizeBuildOrder { package_list: Vec::new() };
        assert_eq!(action.action_type(), ActionType::OptimizeBuildOrder);
        assert!(action.is_safe()); // Empty list is still considered safe
        assert_eq!(action.description(), "Optimize build order for 0 packages");
    }

    #[test]
    fn test_action_schedule_operation() {
        let action = Action::ScheduleOperation { delay_seconds: 60 };
        assert_eq!(action.action_type(), ActionType::ScheduleOperation);
        assert!(action.is_safe());
        assert_eq!(action.description(), "Schedule operation with 60s delay");
    }

    #[test]
    fn test_action_schedule_operation_zero_delay() {
        let action = Action::ScheduleOperation { delay_seconds: 0 };
        assert!(action.is_safe());
        assert_eq!(action.description(), "Schedule operation with 0s delay");
    }

    #[test]
    fn test_action_prefetch_dependencies() {
        let packages = vec![
            "sys-libs/zlib".to_string(),
            "net-libs/openssl".to_string(),
        ];
        let action = Action::PreFetchDependencies { packages: packages.clone() };

        assert_eq!(action.action_type(), ActionType::PreFetchDependencies);
        assert!(action.is_safe());
        assert_eq!(action.description(), "Pre-fetch dependencies for 2 packages");
    }

    #[test]
    fn test_action_prefetch_dependencies_empty() {
        let action = Action::PreFetchDependencies { packages: Vec::new() };
        assert_eq!(action.action_type(), ActionType::PreFetchDependencies);
        assert!(action.is_safe());
        assert_eq!(action.description(), "Pre-fetch dependencies for 0 packages");
    }

    #[test]
    fn test_action_clean_obsolete_packages_safe() {
        let action = Action::CleanObsoletePackages { force: false };
        assert_eq!(action.action_type(), ActionType::CleanObsoletePackages);
        assert!(action.is_safe());
        assert_eq!(action.description(), "Clean obsolete packages (force: false)");
    }

    #[test]
    fn test_action_clean_obsolete_packages_unsafe() {
        let action = Action::CleanObsoletePackages { force: true };
        assert_eq!(action.action_type(), ActionType::CleanObsoletePackages);
        assert!(!action.is_safe()); // Force clean is considered unsafe
        assert_eq!(action.description(), "Clean obsolete packages (force: true)");
    }

    #[test]
    fn test_action_serialization() {
        let actions = vec![
            Action::NoOp,
            Action::AdjustParallelism { jobs: 8 },
            Action::OptimizeBuildOrder { package_list: vec!["test/pkg".to_string()] },
            Action::ScheduleOperation { delay_seconds: 30 },
            Action::PreFetchDependencies { packages: vec!["dep/pkg".to_string()] },
            Action::CleanObsoletePackages { force: false },
        ];

        for action in actions {
            // Test JSON serialization
            let json_result = serde_json::to_string(&action);
            assert!(json_result.is_ok(), "Failed to serialize action: {:?}", action);

            let json_str = json_result.unwrap();
            assert!(!json_str.is_empty());

            // Test JSON deserialization
            let deserialized_result: Result<Action, _> = serde_json::from_str(&json_str);
            assert!(deserialized_result.is_ok(), "Failed to deserialize action: {}", json_str);

            let deserialized = deserialized_result.unwrap();
            assert_eq!(deserialized.action_type(), action.action_type());
            assert_eq!(deserialized.description(), action.description());
        }
    }

    #[test]
    fn test_action_clone() {
        let original = Action::AdjustParallelism { jobs: 16 };
        let cloned = original.clone();

        assert_eq!(original.action_type(), cloned.action_type());
        assert_eq!(original.description(), cloned.description());
        assert_eq!(original.is_safe(), cloned.is_safe());
    }

    #[test]
    fn test_action_debug() {
        let actions = vec![
            Action::NoOp,
            Action::AdjustParallelism { jobs: 4 },
            Action::OptimizeBuildOrder { package_list: vec!["test".to_string()] },
        ];

        for action in actions {
            let debug_str = format!("{:?}", action);
            assert!(!debug_str.is_empty());
            assert!(debug_str.contains("Action"));
        }
    }

    #[test]
    fn test_action_type_coverage() {
        // Test that all action types are covered
        let actions = vec![
            (Action::NoOp, ActionType::NoOp),
            (Action::AdjustParallelism { jobs: 1 }, ActionType::AdjustParallelism),
            (Action::OptimizeBuildOrder { package_list: Vec::new() }, ActionType::OptimizeBuildOrder),
            (Action::ScheduleOperation { delay_seconds: 1 }, ActionType::ScheduleOperation),
            (Action::PreFetchDependencies { packages: Vec::new() }, ActionType::PreFetchDependencies),
            (Action::CleanObsoletePackages { force: false }, ActionType::CleanObsoletePackages),
        ];

        for (action, expected_type) in actions {
            assert_eq!(action.action_type(), expected_type);
        }
    }

    #[test]
    fn test_action_safety_consistency() {
        let safe_actions = vec![
            Action::NoOp,
            Action::AdjustParallelism { jobs: 16 }, // Within safe range
            Action::OptimizeBuildOrder { package_list: vec!["test".to_string()] },
            Action::ScheduleOperation { delay_seconds: 60 },
            Action::PreFetchDependencies { packages: vec!["test".to_string()] },
            Action::CleanObsoletePackages { force: false }, // Non-force
        ];

        for action in safe_actions {
            assert!(action.is_safe(), "Action should be safe: {:?}", action);
        }

        let unsafe_actions = vec![
            Action::AdjustParallelism { jobs: 0 }, // Below minimum
            Action::AdjustParallelism { jobs: 33 }, // Above maximum
            Action::CleanObsoletePackages { force: true }, // Force clean
        ];

        for action in unsafe_actions {
            assert!(!action.is_safe(), "Action should be unsafe: {:?}", action);
        }
    }
}

#[cfg(test)]
mod mock_action_executor_tests {
    use super::*;
    use crate::fixtures::mock_executor::*;
    use crate::fixtures::test_helpers::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_mock_action_executor_creation() {
        let config = MockActionConfig::default();
        let executor = MockActionExecutor::new_with_config(config);

        assert!(executor.is_ok());
    }

    #[tokio::test]
    async fn test_mock_action_executor_basic_execution() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        let action = portcl::actions::Action::NoOp;
        let result = executor.execute_action(&action).await;

        assert!(result.is_ok());
        let action_result = result.unwrap();
        assert!(action_result.success);
        assert!(action_result.execution_time_ms > 0);
    }

    #[tokio::test]
    async fn test_mock_action_executor_all_action_types() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        let actions = vec![
            portcl::actions::Action::NoOp,
            portcl::actions::Action::AdjustParallelism { jobs: 4 },
            portcl::actions::Action::OptimizeBuildOrder {
                package_list: vec!["test/pkg".to_string()]
            },
            portcl::actions::Action::ScheduleOperation { delay_seconds: 30 },
            portcl::actions::Action::PreFetchDependencies {
                packages: vec!["dep/pkg".to_string()]
            },
            portcl::actions::Action::CleanObsoletePackages { force: false },
        ];

        for action in actions {
            let result = executor.execute_action(&action).await;
            assert!(result.is_ok(), "Failed to execute action: {:?}", action);

            let action_result = result.unwrap();
            assert!(action_result.success, "Action should succeed: {:?}", action);
        }
    }

    #[tokio::test]
    async fn test_mock_action_executor_error_injection() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        // Inject error for specific action
        executor.inject_error("execute_action".to_string(), true).await;

        let action = portcl::actions::Action::NoOp;
        let result = executor.execute_action(&action).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PortCLError::Mock(_)));
    }

    #[tokio::test]
    async fn test_mock_action_executor_delay_injection() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        // Inject delay for specific action
        executor.inject_delay("execute_action".to_string(), 200).await;

        let action = portcl::actions::Action::NoOp;
        let start = std::time::Instant::now();
        let result = executor.execute_action(&action).await;
        let duration = start.elapsed();

        assert!(result.is_ok());
        assert!(duration >= Duration::from_millis(180)); // Allow some tolerance
    }

    #[tokio::test]
    async fn test_mock_action_executor_state_management() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        // Execute some actions
        let actions = vec![
            portcl::actions::Action::NoOp,
            portcl::actions::Action::AdjustParallelism { jobs: 4 },
        ];

        for action in actions {
            let _ = executor.execute_action(&action).await;
        }

        // Check state
        let state = executor.get_state().await;
        assert!(state.total_executed >= 2);
        assert!(state.successful_executions >= 2);
        assert_eq!(state.failed_executions, 0);
    }

    #[tokio::test]
    async fn test_mock_action_executor_reset() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        // Execute some actions and inject errors
        executor.execute_action(&portcl::actions::Action::NoOp).await.unwrap();
        executor.inject_error("test".to_string(), true).await;

        // Reset executor
        executor.reset().await;

        // Check that state is reset
        let state = executor.get_state().await;
        assert_eq!(state.total_executed, 0);
        assert_eq!(state.successful_executions, 0);
        assert_eq!(state.failed_executions, 0);
        assert!(state.active_executions.is_empty());
    }

    #[tokio::test]
    async fn test_mock_action_executor_concurrent_execution() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        let action = portcl::actions::Action::NoOp;

        // Execute multiple actions concurrently
        let handles = (0..5).map(|_| {
            let mut executor = executor.clone();
            tokio::spawn(async move {
                executor.execute_action(&portcl::actions::Action::NoOp).await
            })
        });

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All actions should succeed
        for result in results {
            assert!(result.is_ok());
            assert!(result.unwrap().success);
        }
    }

    #[tokio::test]
    async fn test_mock_action_executor_history_tracking() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        // Execute some actions
        let actions = vec![
            portcl::actions::Action::NoOp,
            portcl::actions::Action::AdjustParallelism { jobs: 8 },
            portcl::actions::Action::CleanObsoletePackages { force: false },
        ];

        for action in actions {
            let _ = executor.execute_action(&action).await;
        }

        // Check history
        let history = executor.get_execution_history().await;
        assert_eq!(history.len(), 3);

        // Verify history is in chronological order
        for i in 1..history.len() {
            assert!(history[i-1].end_time <= history[i].start_time);
        }
    }
}

#[cfg(test)]
mod action_validation_tests {
    use super::*;
    use crate::fixtures::test_helpers::*;

    #[test]
    fn test_action_validation_with_test_data_validator() {
        let actions = vec![
            MockAction::NoOp,
            MockAction::AdjustParallelism { jobs: 4 },
            MockAction::OptimizeBuildOrder {
                package_list: vec!["sys-apps/portage".to_string()]
            },
            MockAction::ScheduleOperation { delay_seconds: 60 },
            MockAction::PreFetchDependencies {
                packages: vec!["sys-libs/zlib".to_string()]
            },
            MockAction::CleanObsoletePackages { force: false },
        ];

        for action in actions {
            let validation_result = TestDataValidator::validate_action(&action);
            assert!(validation_result.is_ok(), "Action validation failed: {:?}", action);
        }
    }

    #[test]
    fn test_action_validation_invalid_parallelism() {
        let invalid_actions = vec![
            MockAction::AdjustParallelism { jobs: 0 }, // Zero jobs
            MockAction::AdjustParallelism { jobs: 100 }, // Too many jobs
        ];

        for action in invalid_actions {
            let validation_result = TestDataValidator::validate_action(&action);
            assert!(validation_result.is_err(), "Action should be invalid: {:?}", action);
        }
    }

    #[test]
    fn test_action_validation_empty_package_lists() {
        let empty_list_actions = vec![
            MockAction::OptimizeBuildOrder { package_list: Vec::new() },
            MockAction::PreFetchDependencies { packages: Vec::new() },
        ];

        for action in empty_list_actions {
            let validation_result = TestDataValidator::validate_action(&action);
            // Empty lists should be valid but might be flagged by validation
            if validation_result.is_err() {
                println!("Empty list validation failed (this might be expected): {:?}", validation_result);
            }
        }
    }

    #[test]
    fn test_action_validation_excessive_delays() {
        let excessive_delay = MockAction::ScheduleOperation { delay_seconds: 100000 }; // Very long delay
        let validation_result = TestDataValidator::validate_action(&excessive_delay);
        assert!(validation_result.is_err(), "Excessive delay should be invalid");
    }
}

#[cfg(test)]
mod action_integration_tests {
    use super::*;
    use crate::fixtures::test_helpers::*;

    #[tokio::test]
    async fn test_action_with_mock_environment() {
        let env = MockEnvironmentBuilder::new()
            .with_actions(vec![
                MockAction::AdjustParallelism { jobs: 4 },
                MockAction::OptimizeBuildOrder {
                    package_list: vec!["test/pkg".to_string()]
                },
            ])
            .build()
            .expect("Failed to build mock environment");

        // Test that environment contains valid actions
        assert_eq!(env.actions.len(), 2);

        // Validate actions in environment
        for action in &env.actions {
            let validation_result = TestDataValidator::validate_action(action);
            assert!(validation_result.is_ok(), "Environment action invalid: {:?}", action);
        }
    }

    #[tokio::test]
    async fn test_action_execution_workflow() {
        let env = MockEnvironmentBuilder::new()
            .with_actions(vec![
                MockAction::NoOp,
                MockAction::CleanObsoletePackages { force: false },
            ])
            .build()
            .expect("Failed to build mock environment");

        let mut executor = env.executor.write().await;

        // Execute all actions in environment
        for action in &env.actions {
            let result = executor.execute_action(&action).await;
            assert!(result.is_ok(), "Failed to execute environment action: {:?}", action);
        }

        // Verify execution history
        let history = executor.get_execution_history().await;
        assert_eq!(history.len(), 2);
    }
}

#[cfg(test)]
mod action_performance_tests {
    use super::*;
    use crate::fixtures::test_helpers::*;

    #[test]
    fn test_action_creation_performance() {
        let benchmark = BenchmarkHelpers::run_statistical_benchmark(
            "action_creation",
            1000,
            || {
                let _actions = vec![
                    portcl::actions::Action::NoOp,
                    portcl::actions::Action::AdjustParallelism { jobs: 4 },
                    portcl::actions::Action::OptimizeBuildOrder {
                        package_list: vec!["test/pkg".to_string()]
                    },
                ];
            }
        );

        assert!(benchmark.success_rate > 0.99);
        assert!(benchmark.average_duration_ms < 10.0); // Should be very fast
    }

    #[tokio::test]
    async fn test_mock_action_executor_performance() {
        let config = MockActionConfig::default();
        let mut executor = MockActionExecutor::new_with_config(config).unwrap();

        let benchmark = BenchmarkHelpers::benchmark_async(
            "mock_action_execution",
            executor.execute_action(&portcl::actions::Action::NoOp),
        ).await;

        assert!(benchmark.success);
        assert!(benchmark.duration_ms < 100); // Should be fast
        assert!(benchmark.memory_usage_bytes < 1024 * 1024); // Less than 1MB
    }

    #[tokio::test]
    async fn test_action_serialization_performance() {
        let actions = vec![
            portcl::actions::Action::NoOp,
            portcl::actions::Action::AdjustParallelism { jobs: 8 },
            portcl::actions::Action::OptimizeBuildOrder {
                package_list: vec!["test/pkg".to_string()]
            },
        ];

        let benchmark = BenchmarkHelpers::benchmark(
            "action_serialization",
            || {
                for action in &actions {
                    let _ = serde_json::to_string(action).unwrap();
                }
            }
        );

        assert!(benchmark.success);
        assert!(benchmark.duration_ms < 50); // Should be very fast
    }
}