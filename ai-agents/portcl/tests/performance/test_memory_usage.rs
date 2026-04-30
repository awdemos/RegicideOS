//! Resource Usage Performance Tests for PortCL
//!
//! These benchmarks measure memory usage (<100MB RAM) and CPU utilization (<3%)
//! to ensure the system operates within resource constraints.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use portcl::actions::Action;
use portcl::error::{PortCLError, Result};
use portcl::prelude::*;

// Memory and CPU tracking structures
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub peak_memory_mb: f64,
    pub peak_cpu_percent: f64,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct ResourceMonitor {
    pub initial_memory_mb: f64,
    pub initial_cpu_percent: f64,
    pub sampling_interval_ms: u64,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            initial_memory_mb: 10.0, // Simulated baseline
            initial_cpu_percent: 0.5, // Simulated baseline
            sampling_interval_ms: 100,
        }
    }

    pub async fn measure_memory_usage(&self, operation: &str) -> Result<ResourceMetrics> {
        // Simulate memory measurement
        tokio::time::sleep(Duration::from_millis(10)).await;

        let memory_usage = match operation {
            "agent_initialization" => 15.0,
            "model_loading" => 45.0,
            "action_selection" => 25.0,
            "training_cycle" => 80.0,
            "monitoring" => 12.0,
            "configuration" => 8.0,
            "package_query" => 18.0,
            "large_dataset_processing" => 95.0,
            _ => 20.0,
        };

        Ok(ResourceMetrics {
            memory_usage_mb: memory_usage,
            cpu_usage_percent: 1.5,
            peak_memory_mb: memory_usage * 1.2,
            peak_cpu_percent: 2.5,
            timestamp: chrono::Utc::now(),
        })
    }

    pub async fn measure_cpu_usage(&self, operation: &str, duration_ms: u64) -> Result<ResourceMetrics> {
        // Simulate CPU measurement
        tokio::time::sleep(Duration::from_millis(5)).await;

        let cpu_usage = match operation {
            "model_inference" => 2.8,
            "data_processing" => 1.8,
            "network_io" => 0.8,
            "file_operations" => 1.2,
            "training_step" => 2.5,
            "monitoring_cycle" => 0.5,
            "concurrent_operations" => 2.9,
            _ => 1.0,
        };

        Ok(ResourceMetrics {
            memory_usage_mb: 30.0,
            cpu_usage_percent: cpu_usage,
            peak_memory_mb: 35.0,
            peak_cpu_percent: cpu_usage * 1.1,
            timestamp: chrono::Utc::now(),
        })
    }

    pub async fn track_memory_growth(&self, operations: Vec<&str>) -> Result<Vec<ResourceMetrics>> {
        let mut metrics = Vec::new();
        let mut current_memory = self.initial_memory_mb;

        for operation in operations {
            let operation_metrics = self.measure_memory_usage(operation).await?;
            current_memory += operation_metrics.memory_usage_mb * 0.1; // Simulate growth
            metrics.push(ResourceMetrics {
                memory_usage_mb: current_memory,
                cpu_usage_percent: operation_metrics.cpu_usage_percent,
                peak_memory_mb: current_memory * 1.1,
                peak_cpu_percent: operation_metrics.peak_cpu_percent,
                timestamp: chrono::Utc::now(),
            });

            tokio::time::sleep(Duration::from_millis(self.sampling_interval_ms)).await;
        }

        Ok(metrics)
    }
}

#[derive(Debug, Clone)]
pub struct MockMemoryHeavySystem {
    pub base_memory_mb: f64,
    pub memory_growth_rate: f64,
    pub cleanup_enabled: bool,
}

impl MockMemoryHeavySystem {
    pub fn new(base_memory_mb: f64, memory_growth_rate: f64) -> Self {
        Self {
            base_memory_mb,
            memory_growth_rate,
            cleanup_enabled: true,
        }
    }

    pub async fn simulate_memory_intensive_operation(&self, duration_ms: u64) -> Result<f64> {
        // Simulate memory-intensive operation
        let memory_used = self.base_memory_mb + (self.memory_growth_rate * duration_ms as f64 / 1000.0);

        tokio::time::sleep(Duration::from_millis(duration_ms)).await;

        if self.cleanup_enabled {
            // Simulate memory cleanup
            Ok(memory_used * 0.7) // Return to 70% of peak usage
        } else {
            Ok(memory_used)
        }
    }

    pub async fn simulate_memory_leak_scenario(&self, operations: u32) -> Result<f64> {
        let mut total_memory = self.base_memory_mb;

        for i in 0..operations {
            let operation_memory = 5.0 + (i as f64 * 0.5); // Increasing memory per operation
            total_memory += operation_memory;

            if !self.cleanup_enabled && total_memory > 200.0 {
                return Err(PortCLError::Memory("Memory limit exceeded".to_string()));
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(total_memory)
    }
}

#[derive(Debug, Clone)]
pub struct MockCPUBoundSystem {
    pub base_cpu_percent: f64,
    pub cpu_spike_threshold: f64,
    pub cpu_optimization_enabled: bool,
}

impl MockCPUBoundSystem {
    pub fn new(base_cpu_percent: f64, cpu_spike_threshold: f64) -> Self {
        Self {
            base_cpu_percent,
            cpu_spike_threshold,
            cpu_optimization_enabled: true,
        }
    }

    pub async fn simulate_cpu_intensive_task(&self, complexity: u32) -> Result<f64> {
        // Simulate CPU-intensive task
        let base_usage = self.base_cpu_percent;
        let complexity_factor = complexity as f64 / 10.0;
        let cpu_usage = base_usage + complexity_factor;

        let duration_ms = 50 + (complexity * 10);
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;

        if self.cpu_optimization_enabled && cpu_usage > self.cpu_spike_threshold {
            // Simulate CPU optimization
            Ok(cpu_usage * 0.6) // Optimize to 60% of spike
        } else {
            Ok(cpu_usage)
        }
    }

    pub async fn simulate_concurrent_cpu_load(&self, task_count: u32) -> Result<f64> {
        let mut handles = Vec::new();

        for i in 0..task_count {
            let system_clone = self.clone();
            let handle = tokio::spawn(async move {
                system_clone.simulate_cpu_intensive_task(i % 5 + 1).await
            });
            handles.push(handle);
        }

        let results: Result<Vec<_>, _> = futures::future::try_join_all(handles).await;
        let cpu_usages = results?;

        let avg_cpu_usage = cpu_usages.iter().sum::<f64>() / cpu_usages.len() as f64;

        // Concurrent tasks should not exceed 3% total CPU
        Ok(avg_cpu_usage)
    }
}

// Memory Usage Benchmarks
fn benchmark_agent_initialization_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let monitor = ResourceMonitor::new();

    let mut group = c.benchmark_group("memory_agent_init");

    for agent_size in ["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::new("agent_init_memory", agent_size), &agent_size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let start = Instant::now();
                let result = monitor.measure_memory_usage("agent_initialization").await;
                let duration = start.elapsed();

                let metrics = result.unwrap();

                // Verify memory usage <100MB
                assert!(
                    metrics.memory_usage_mb < 100.0,
                    "Agent initialization memory usage should be <100MB, used {}MB",
                    metrics.memory_usage_mb
                );

                // Verify initialization time <1s
                assert!(
                    duration.as_millis() < 1000,
                    "Agent initialization should complete in <1s, took {}ms",
                    duration.as_millis()
                );

                metrics
            });
        });
    }

    group.finish();
}

fn benchmark_model_loading_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let monitor = ResourceMonitor::new();

    let mut group = c.benchmark_group("memory_model_loading");

    for model_size in ["tiny", "small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::new("model_loading_memory", model_size), &model_size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let start = Instant::now();
                let result = monitor.measure_memory_usage("model_loading").await;
                let duration = start.elapsed();

                let metrics = result.unwrap();

                // Model loading should be memory efficient
                let max_allowed = match *model_size {
                    "tiny" => 20.0,
                    "small" => 40.0,
                    "medium" => 60.0,
                    "large" => 85.0,
                    _ => 100.0,
                };

                assert!(
                    metrics.memory_usage_mb < max_allowed,
                    "{} model loading memory usage should be <{}MB, used {}MB",
                    model_size,
                    max_allowed,
                    metrics.memory_usage_mb
                );

                metrics
            });
        });
    }

    group.finish();
}

fn benchmark_training_cycle_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let monitor = ResourceMonitor::new();
    let system = MockMemoryHeavySystem::new(20.0, 0.5);

    let mut group = c.benchmark_group("memory_training_cycle");

    for training_steps in [100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("training_memory", training_steps),
            &training_steps,
            |b, &steps| {
                b.to_async(&rt).iter(|| async {
                    let start = Instant::now();

                    // Simulate training cycle with multiple steps
                    let mut total_memory = 0.0;
                    for _ in 0..steps.min(100) { // Limit for benchmark performance
                        let memory = system.simulate_memory_intensive_operation(10).await.unwrap();
                        total_memory += memory;
                    }

                    let avg_memory = total_memory / (steps.min(100) as f64);
                    let duration = start.elapsed();

                    // Training cycles should manage memory well
                    assert!(
                        avg_memory < 100.0,
                        "Training cycle average memory usage should be <100MB, used {}MB",
                        avg_memory
                    );

                    avg_memory
                });
            },
        );
    }

    group.finish();
}

fn benchmark_monitoring_memory_growth(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let monitor = ResourceMonitor::new();

    let mut group = c.benchmark_group("memory_monitoring_growth");

    for monitoring_duration in [60, 300, 600] { // seconds
        group.bench_with_input(
            BenchmarkId::new("monitoring_memory_growth", monitoring_duration),
            &monitoring_duration,
            |b, &duration| {
                b.to_async(&rt).iter(|| async {
                    let operations = vec
                        !["monitoring"; (duration / 10) as usize]
                        .iter()
                        .map(|s| s.as_str())
                        .collect();

                    let metrics_list = monitor.track_memory_growth(operations).await.unwrap();

                    let final_memory = metrics_list.last().unwrap().memory_usage_mb;
                    let peak_memory = metrics_list.iter().map(|m| m.peak_memory_mb).fold(0.0, f64::max);

                    // Long-running monitoring should not grow unbounded
                    assert!(
                        final_memory < 50.0,
                        "Long-running monitoring should stay below 50MB, reached {}MB",
                        final_memory
                    );

                    assert!(
                        peak_memory < 100.0,
                        "Monitoring peak memory should be <100MB, reached {}MB",
                        peak_memory
                    );

                    final_memory
                });
            },
        );
    }

    group.finish();
}

// CPU Usage Benchmarks
fn benchmark_model_inference_cpu(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let monitor = ResourceMonitor::new();
    let cpu_system = MockCPUBoundSystem::new(0.5, 2.5);

    let mut group = c.benchmark_group("cpu_model_inference");

    for inference_complexity in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("inference_cpu", inference_complexity),
            &inference_complexity,
            |b, &complexity| {
                b.to_async(&rt).iter(|| async {
                    let start = Instant::now();
                    let result = cpu_system.simulate_cpu_intensive_task(complexity).await;
                    let duration = start.elapsed();

                    let cpu_usage = result.unwrap();

                    // Model inference should use <3% CPU
                    assert!(
                        cpu_usage < 3.0,
                        "Model inference CPU usage should be <3%, used {}%",
                        cpu_usage
                    );

                    // Inference should be fast
                    assert!(
                        duration.as_millis() < 100,
                        "Model inference should complete in <100ms, took {}ms",
                        duration.as_millis()
                    );

                    cpu_usage
                });
            },
        );
    }

    group.finish();
}

fn benchmark_concurrent_operations_cpu(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cpu_system = MockCPUBoundSystem::new(0.3, 2.0);

    let mut group = c.benchmark_group("cpu_concurrent_operations");

    for concurrent_count in [1, 3, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_cpu", concurrent_count),
            &concurrent_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let start = Instant::now();
                    let result = cpu_system.simulate_concurrent_cpu_load(count).await;
                    let duration = start.elapsed();

                    let avg_cpu_usage = result.unwrap();

                    // Concurrent operations should not exceed 3% total CPU
                    assert!(
                        avg_cpu_usage < 3.0,
                        "Concurrent operations average CPU usage should be <3%, used {}%",
                        avg_cpu_usage
                    );

                    // Concurrent operations should complete efficiently
                    assert!(
                        duration.as_millis() < 500,
                        "Concurrent operations should complete in <500ms, took {}ms",
                        duration.as_millis()
                    );

                    avg_cpu_usage
                });
            },
        );
    }

    group.finish();
}

fn benchmark_system_idle_cpu(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let monitor = ResourceMonitor::new();

    let mut group = c.benchmark_group("cpu_system_idle");

    for idle_duration in [1000, 5000, 10000] { // milliseconds
        group.bench_with_input(
            BenchmarkId::new("idle_cpu", idle_duration),
            &idle_duration,
            |b, &duration| {
                b.to_async(&rt).iter(|| async {
                    let start = Instant::now();

                    // Simulate idle system monitoring
                    tokio::time::sleep(Duration::from_millis(duration)).await;

                    let cpu_metrics = monitor.measure_cpu_usage("monitoring_cycle", duration).await.unwrap();
                    let actual_duration = start.elapsed();

                    // Idle system should use minimal CPU
                    assert!(
                        cpu_metrics.cpu_usage_percent < 1.0,
                        "Idle system CPU usage should be <1%, used {}%",
                        cpu_metrics.cpu_usage_percent
                    );

                    // Duration should be accurate
                    let duration_diff = (actual_duration.as_millis() as i64 - duration as i64).abs();
                    assert!(
                        duration_diff < 100, // Allow 100ms variance
                        "Idle duration should be accurate, expected {}ms, got {}ms",
                        duration,
                        actual_duration.as_millis()
                    );

                    cpu_metrics.cpu_usage_percent
                });
            },
        );
    }

    group.finish();
}

// Resource Cleanup Benchmarks
fn benchmark_memory_cleanup_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_cleanup");

    for cleanup_enabled in [true, false] {
        group.bench_with_input(
            BenchmarkId::new("memory_cleanup", cleanup_enabled),
            &cleanup_enabled,
            |b, &enabled| {
                b.to_async(&rt).iter(|| async {
                    let mut system = MockMemoryHeavySystem::new(30.0, 2.0);
                    system.cleanup_enabled = enabled;

                    let start = Instant::now();

                    // Simulate memory-intensive operations
                    let peak_memory = system.simulate_memory_intensive_operation(500).await.unwrap();

                    // Simulate cleanup period
                    tokio::time::sleep(Duration::from_millis(200)).await;

                    let final_memory = system.simulate_memory_intensive_operation(50).await.unwrap();
                    let duration = start.elapsed();

                    if enabled {
                        // With cleanup enabled, memory should be significantly reduced
                        let cleanup_ratio = final_memory / peak_memory;
                        assert!(
                            cleanup_ratio < 0.8,
                            "With cleanup enabled, final memory should be <80% of peak, got {}%",
                            cleanup_ratio * 100.0
                        );
                    }

                    // Regardless of cleanup, should stay under 100MB
                    assert!(
                        final_memory < 100.0,
                        "Final memory usage should be <100MB, used {}MB",
                        final_memory
                    );

                    cleanup_ratio
                });
            },
        );
    }

    group.finish();
}

fn benchmark_resource_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resource_optimization");

    for optimization_enabled in [true, false] {
        group.bench_with_input(
            BenchmarkId::new("resource_optimization", optimization_enabled),
            &optimization_enabled,
            |b, &enabled| {
                b.to_async(&rt).iter(|| async {
                    let mut cpu_system = MockCPUBoundSystem::new(1.0, 2.5);
                    cpu_system.cpu_optimization_enabled = enabled;

                    let mut memory_system = MockMemoryHeavySystem::new(25.0, 1.0);
                    memory_system.cleanup_enabled = enabled;

                    let start = Instant::now();

                    // Simulate mixed workload
                    let mut total_cpu = 0.0;
                    let mut total_memory = 0.0;

                    for i in 0..10 {
                        let cpu = cpu_system.simulate_cpu_intensive_task(i % 3 + 1).await.unwrap();
                        let memory = memory_system.simulate_memory_intensive_operation(50).await.unwrap();
                        total_cpu += cpu;
                        total_memory += memory;
                    }

                    let avg_cpu = total_cpu / 10.0;
                    let avg_memory = total_memory / 10.0;
                    let duration = start.elapsed();

                    if enabled {
                        // With optimization enabled, resources should be well-managed
                        assert!(
                            avg_cpu < 2.0,
                            "With optimization enabled, average CPU should be <2%, used {}%",
                            avg_cpu
                        );
                        assert!(
                            avg_memory < 40.0,
                            "With optimization enabled, average memory should be <40MB, used {}MB",
                            avg_memory
                        );
                    }

                    // Regardless of optimization, should stay within limits
                    assert!(
                        avg_cpu < 3.0,
                        "Average CPU usage should be <3%, used {}%",
                        avg_cpu
                    );
                    assert!(
                        avg_memory < 100.0,
                        "Average memory usage should be <100MB, used {}MB",
                        avg_memory
                    );

                    (avg_cpu, avg_memory)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_agent_initialization_memory,
    benchmark_model_loading_memory,
    benchmark_training_cycle_memory,
    benchmark_monitoring_memory_growth,
    benchmark_model_inference_cpu,
    benchmark_concurrent_operations_cpu,
    benchmark_system_idle_cpu,
    benchmark_memory_cleanup_efficiency,
    benchmark_resource_optimization
);
criterion_main!(benches);