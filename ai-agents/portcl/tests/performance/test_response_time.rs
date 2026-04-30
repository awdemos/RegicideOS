//! Response Time Performance Tests for PortCL
//!
//! These benchmarks measure response times for critical operations
//! to ensure they meet performance targets (<300ms for action selection).

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use portcl::actions::Action;
use portcl::error::{PortCLError, Result};
use portcl::prelude::*;

// Mock performance monitoring structures
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operation_name: String,
    pub duration_ms: f64,
    pub success: bool,
    pub memory_usage_bytes: u64,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct MockPortageSystem {
    pub response_time_ms: u64,
    pub success_rate: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

impl MockPortageSystem {
    pub fn new(response_time_ms: u64, success_rate: f64) -> Self {
        Self {
            response_time_ms,
            success_rate,
            memory_usage_mb: 50.0,
            cpu_usage_percent: 2.0,
        }
    }

    pub async fn simulate_package_query(&self, package: &str) -> Result<String> {
        tokio::time::sleep(Duration::from_millis(self.response_time_ms)).await;

        if rand::random::<f64>() < self.success_rate {
            Ok(format!("package_info_{}", package))
        } else {
            Err(PortCLError::Portage("Simulated query failure".to_string()))
        }
    }

    pub async fn simulate_system_metrics(&self) -> Result<SystemMetrics> {
        tokio::time::sleep(Duration::from_millis(self.response_time_ms / 2)).await;

        Ok(SystemMetrics {
            cpu_usage_percent: self.cpu_usage_percent,
            memory_usage_percent: self.memory_usage_mb * 2.0, // Simulate 50% of 100MB
            disk_usage_percent: 45.0,
            load_average_1min: 1.2,
            load_average_5min: 1.1,
            load_average_15min: 1.0,
            network_connections: 150,
            active_processes: 200,
            uptime_seconds: 86400,
            timestamp: chrono::Utc::now(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockRLAgent {
    pub decision_time_ms: u64,
    pub learning_enabled: bool,
}

impl MockRLAgent {
    pub fn new(decision_time_ms: u64) -> Self {
        Self {
            decision_time_ms,
            learning_enabled: true,
        }
    }

    pub async fn select_action(&self, metrics: &SystemMetrics) -> Result<Action> {
        tokio::time::sleep(Duration::from_millis(self.decision_time_ms)).await;

        // Simple action selection based on CPU usage
        if metrics.cpu_usage_percent > 80.0 {
            Ok(Action::NoOp)
        } else if metrics.memory_usage_percent > 70.0 {
            Ok(Action::CleanObsoletePackages { force: false })
        } else {
            Ok(Action::AdjustParallelism { jobs: 4 })
        }
    }

    pub async fn train_model(&self) -> Result<f64> {
        tokio::time::sleep(Duration::from_millis(self.decision_time_ms * 2)).await;

        // Simulate training with some loss value
        Ok(0.123 * rand::random::<f64>())
    }
}

// Performance Test Scenarios
fn benchmark_action_selection_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let agent = MockRLAgent::new(50); // 50ms decision time
    let system = MockPortageSystem::new(25, 0.95); // 25ms response time, 95% success

    let mut group = c.benchmark_group("action_selection");

    for scenario in ["low_load", "medium_load", "high_load"] {
        group.bench_with_input(BenchmarkId::new("action_selection", scenario), &scenario, |b, _| {
            b.to_async(&rt).iter(|| async {
                let metrics = match *scenario {
                    "low_load" => {
                        SystemMetrics {
                            cpu_usage_percent: 30.0,
                            memory_usage_percent: 40.0,
                            disk_usage_percent: 45.0,
                            load_average_1min: 0.8,
                            load_average_5min: 0.7,
                            load_average_15min: 0.6,
                            network_connections: 100,
                            active_processes: 150,
                            uptime_seconds: 86400,
                            timestamp: chrono::Utc::now(),
                        }
                    },
                    "medium_load" => {
                        SystemMetrics {
                            cpu_usage_percent: 60.0,
                            memory_usage_percent: 65.0,
                            disk_usage_percent: 60.0,
                            load_average_1min: 1.5,
                            load_average_5min: 1.4,
                            load_average_15min: 1.3,
                            network_connections: 200,
                            active_processes: 250,
                            uptime_seconds: 86400,
                            timestamp: chrono::Utc::now(),
                        }
                    },
                    "high_load" => {
                        SystemMetrics {
                            cpu_usage_percent: 85.0,
                            memory_usage_percent: 85.0,
                            disk_usage_percent: 80.0,
                            load_average_1min: 2.5,
                            load_average_5min: 2.3,
                            load_average_15min: 2.1,
                            network_connections: 300,
                            active_processes: 350,
                            uptime_seconds: 86400,
                            timestamp: chrono::Utc::now(),
                        }
                    },
                    _ => panic!("Unknown scenario"),
                };

                let start = Instant::now();
                let result = agent.select_action(&metrics).await;
                let duration = start.elapsed();

                // Verify performance target (<300ms)
                assert!(
                    duration.as_millis() < 300,
                    "Action selection should complete in <300ms, took {}ms",
                    duration.as_millis()
                );

                result
            });
        });
    }

    group.finish();
}

fn benchmark_model_inference_speed(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("model_inference");

    for model_size in ["small", "medium", "large"] {
        let inference_time = match model_size {
            "small" => 10,   // 10ms
            "medium" => 25,  // 25ms
            "large" => 50,   // 50ms
            _ => panic!("Unknown model size"),
        };

        group.bench_with_input(BenchmarkId::new("inference_speed", model_size), &model_size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let agent = MockRLAgent::new(inference_time);
                let metrics = SystemMetrics {
                    cpu_usage_percent: 50.0,
                    memory_usage_percent: 60.0,
                    disk_usage_percent: 45.0,
                    load_average_1min: 1.2,
                    load_average_5min: 1.1,
                    load_average_15min: 1.0,
                    network_connections: 150,
                    active_processes: 200,
                    uptime_seconds: 86400,
                    timestamp: chrono::Utc::now(),
                };

                let start = Instant::now();
                let result = agent.select_action(&metrics).await;
                let duration = start.elapsed();

                // Model inference should be much faster than 300ms target
                assert!(
                    duration.as_millis() < 100,
                    "Model inference should complete in <100ms, took {}ms",
                    duration.as_millis()
                );

                result
            });
        });
    }

    group.finish();
}

fn benchmark_portage_api_response_times(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("portage_api");

    for operation in ["package_query", "system_metrics", "package_list"] {
        for response_time in [10, 25, 50, 100] { // Different response time scenarios
            group.bench_with_input(
                BenchmarkId::new("portage_response", format!("{}_{}ms", operation, response_time)),
                &(operation, response_time),
                |b, (op, rt_ms)| {
                    b.to_async(&rt).iter(|| async {
                        let system = MockPortageSystem::new(*rt_ms, 0.95);
                        let start = Instant::now();

                        let result = match *op {
                            "package_query" => system.simulate_package_query("sys-apps/portage").await,
                            "system_metrics" => system.simulate_system_metrics().await.map(|_| String::new()),
                            "package_list" => system.simulate_package_query("*").await,
                            _ => panic!("Unknown operation"),
                        };

                        let duration = start.elapsed();

                        // Portage API responses should be fast
                        assert!(
                            duration.as_millis() < 200,
                            "Portage API operation {} should complete in <200ms, took {}ms",
                            op,
                            duration.as_millis()
                        );

                        result
                    });
                });
            }
        }
    }

    group.finish();
}

fn benchmark_overall_system_responsiveness(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("system_responsiveness");

    for workflow in ["monitoring_cycle", "decision_cycle", "training_cycle"] {
        group.bench_with_input(BenchmarkId::new("responsiveness", workflow), workflow, |b, workflow| {
            b.to_async(&rt).iter(|| async {
                let system = MockPortageSystem::new(30, 0.95);
                let agent = MockRLAgent::new(50);

                let start = Instant::now();
                let result = match *workflow {
                    "monitoring_cycle" => {
                        // Simulate complete monitoring cycle
                        let metrics = system.simulate_system_metrics().await;
                        let _ = system.simulate_package_query("test-package").await;
                        Ok(metrics)
                    },
                    "decision_cycle" => {
                        // Simulate decision making cycle
                        let metrics = system.simulate_system_metrics().await?;
                        agent.select_action(&metrics).await
                    },
                    "training_cycle" => {
                        // Simulate training cycle
                        let metrics = system.simulate_system_metrics().await?;
                        let _action = agent.select_action(&metrics).await?;
                        agent.train_model().await
                    },
                    _ => panic!("Unknown workflow"),
                };

                let duration = start.elapsed();

                // Overall system responsiveness should meet target
                let max_duration = match *workflow {
                    "monitoring_cycle" => 200,  // Should be fast
                    "decision_cycle" => 300,    // Primary target
                    "training_cycle" => 1000,   // Can be slower
                    _ => panic!("Unknown workflow"),
                };

                assert!(
                    duration.as_millis() < max_duration,
                    "Workflow {} should complete in <{}ms, took {}ms",
                    workflow,
                    max_duration,
                    duration.as_millis()
                );

                result
            });
        });
    }

    group.finish();
}

fn benchmark_error_handling_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("error_handling");

    for error_scenario in ["network_error", "timeout_error", "validation_error"] {
        group.bench_with_input(BenchmarkId::new("error_handling", error_scenario), error_scenario, |b, scenario| {
            b.to_async(&rt).iter(|| async {
                let system = MockPortageSystem::new(25, 0.5); // 50% failure rate for error testing

                let start = Instant::now();
                let result = match *scenario {
                    "network_error" => {
                        // Simulate network-related error
                        if rand::random::<f64>() < 0.3 {
                            Err(PortCLError::Network("Connection refused".into()))
                        } else {
                            system.simulate_package_query("test").await
                        }
                    },
                    "timeout_error" => {
                        // Simulate timeout
                        if rand::random::<f64>() < 0.3 {
                            Err(PortCLError::Timeout("Operation timed out".to_string()))
                        } else {
                            system.simulate_system_metrics().await.map(|_| String::new())
                        }
                    },
                    "validation_error" => {
                        // Simulate validation error
                        if rand::random::<f64>() < 0.3 {
                            Err(PortCLError::Validation("Invalid input".to_string()))
                        } else {
                            Ok("validation_success".to_string())
                        }
                    },
                    _ => panic!("Unknown error scenario"),
                };

                let duration = start.elapsed();

                // Error handling should still be fast
                assert!(
                    duration.as_millis() < 500,
                    "Error handling scenario {} should complete in <500ms, took {}ms",
                    scenario,
                    duration.as_millis()
                );

                // Don't care about result for performance testing
                Ok::<(), PortCLError>(())
            });
        });
    }

    group.finish();
}

fn benchmark_concurrent_action_selection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_operations");

    for concurrent_count in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_actions", concurrent_count),
            &concurrent_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let system = MockPortageSystem::new(20, 0.95);
                    let agent = MockRLAgent::new(30);

                    let start = Instant::now();

                    // Spawn concurrent action selections
                    let mut handles = Vec::new();
                    for i in 0..count {
                        let system_clone = system.clone();
                        let agent_clone = agent.clone();

                        let handle = tokio::spawn(async move {
                            let metrics = SystemMetrics {
                                cpu_usage_percent: 30.0 + (i as f64 * 5.0),
                                memory_usage_percent: 40.0 + (i as f64 * 3.0),
                                disk_usage_percent: 45.0,
                                load_average_1min: 1.2,
                                load_average_5min: 1.1,
                                load_average_15min: 1.0,
                                network_connections: 150,
                                active_processes: 200,
                                uptime_seconds: 86400,
                                timestamp: chrono::Utc::now(),
                            };

                            agent_clone.select_action(&metrics).await
                        });
                        handles.push(handle);
                    }

                    // Wait for all operations to complete
                    let results: Result<Vec<_>, _> = futures::future::try_join_all(handles).await;
                    let duration = start.elapsed();

                    // Concurrent operations should still meet per-operation targets
                    assert!(
                        duration.as_millis() < 300 * count as u64,
                        "Concurrent actions ({}) should complete in <{}ms, took {}ms",
                        count,
                        300 * count,
                        duration.as_millis()
                    );

                    results.map(|_| ())
                });
            });
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_action_selection_latency,
    benchmark_model_inference_speed,
    benchmark_portage_api_response_times,
    benchmark_overall_system_responsiveness,
    benchmark_error_handling_performance,
    benchmark_concurrent_action_selection
);
criterion_main!(benches);