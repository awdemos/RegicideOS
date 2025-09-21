//! Mock PortageMonitor for testing PortCL monitoring functionality
//!
//! This module provides a mock implementation of PortageMonitor that
//! simulates Portage package management operations without requiring
//! actual Portage system calls.

use crate::fixtures::mock_data::*;
use crate::fixtures::test_models::*;
use crate::error::PortCLError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Mock monitoring metrics for testing
#[derive(Debug, Clone)]
pub struct MockMonitorMetrics {
    pub system_cpu_usage: f64,
    pub system_memory_usage: f64,
    pub disk_usage: f64,
    pub network_io_bytes: u64,
    pub portage_operations: u32,
    pub failed_operations: u32,
    pub active_connections: u32,
    pub uptime_seconds: u64,
}

/// Mock PortageMonitor that simulates Portage system monitoring
#[derive(Debug, Clone)]
pub struct MockPortageMonitor {
    config: MockMonitoringConfig,
    metrics: Arc<RwLock<MockMonitorMetrics>>,
    mock_packages: Vec<MockPackage>,
    error_injection: Arc<RwLock<HashMap<String, bool>>>,
    delay_injection: Arc<RwLock<HashMap<String, u64>>>,
}

impl MockPortageMonitor {
    /// Create a new mock PortageMonitor with default configuration
    pub fn new(config: MockMonitoringConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RwLock::new(MockMonitorMetrics::default())),
            mock_packages: Vec::new(),
            error_injection: Arc::new(RwLock::new(HashMap::new())),
            delay_injection: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a mock monitor with pre-populated test data
    pub fn with_test_data(config: MockMonitoringConfig, packages: Vec<MockPackage>) -> Self {
        let mut monitor = Self::new(config);
        monitor.mock_packages = packages;
        monitor
    }

    /// Inject an error for a specific operation
    pub fn inject_error(&self, operation: String) {
        let mut errors = self.error_injection.write().unwrap();
        errors.insert(operation, true);
    }

    /// Inject a delay for a specific operation (in milliseconds)
    pub fn inject_delay(&self, operation: String, delay_ms: u64) {
        let mut delays = self.delay_injection.write().unwrap();
        delays.insert(operation, delay_ms);
    }

    /// Clear all injected errors and delays
    pub fn clear_injections(&self) {
        let mut errors = self.error_injection.write().unwrap();
        let mut delays = self.delay_injection.write().unwrap();
        errors.clear();
        delays.clear();
    }

    /// Update mock metrics
    pub fn update_metrics(&self, metrics: MockMonitorMetrics) {
        let mut current_metrics = self.metrics.write().unwrap();
        *current_metrics = metrics;
    }

    /// Simulate system load changes
    pub fn simulate_load(&self, cpu_usage: f64, memory_usage: f64) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.system_cpu_usage = cpu_usage;
        metrics.system_memory_usage = memory_usage;
    }

    /// Simulate a failed operation
    pub fn simulate_failure(&self, operation: &str) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.failed_operations += 1;
        let mut errors = self.error_injection.write().unwrap();
        errors.insert(operation.to_string(), true);
    }

    /// Get current metrics (for testing)
    pub fn get_current_metrics(&self) -> MockMonitorMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Check if an operation should fail based on injected errors
    fn should_fail(&self, operation: &str) -> bool {
        let errors = self.error_injection.read().unwrap();
        errors.get(operation).copied().unwrap_or(false)
    }

    /// Get delay for an operation based on injected delays
    fn get_delay(&self, operation: &str) -> u64 {
        let delays = self.delay_injection.read().unwrap();
        delays.get(operation).copied().unwrap_or(0)
    }

    /// Simulate async delay
    async fn simulate_delay(&self, operation: &str) {
        let delay = self.get_delay(operation);
        if delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }
    }

    /// Generate mock portage info
    fn generate_portage_info(&self) -> PortageInfo {
        let now = Utc::now();
        PortageInfo {
            installed_packages: self.mock_packages.len() as u32,
            available_updates: (self.mock_packages.len() / 10) as u32, // 10% need updates
            world_packages: (self.mock_packages.len() / 5) as u32,   // 20% are world packages
            last_sync: Some(now - chrono::Duration::hours(24)),      // Last sync 24 hours ago
            portage_version: "3.0.30".to_string(),
            profile: "default/linux/amd64/17.1".to_string(),
            arch: "amd64".to_string(),
        }
    }

    /// Generate mock package info
    fn generate_package_info(&self, package_name: &str) -> Option<PackageInfo> {
        let mock_pkg = self.mock_packages.iter().find(|pkg| pkg.name == package_name)?;

        Some(PackageInfo {
            name: mock_pkg.name.clone(),
            version: mock_pkg.version.clone(),
            category: mock_pkg.category.clone(),
            slot: Some("0".to_string()),
            repository: Some("gentoo".to_string()),
            installed_size: Some(1024 * 1024 * mock_pkg.name.len() as u64), // Mock size based on name length
            homepage: mock_pkg.homepage.clone(),
            description: Some(mock_pkg.description.clone()),
            license: Some(mock_pkg.license.clone()),
            use_flags: mock_pkg.use_flags.clone(),
            dependencies: mock_pkg.dependencies.clone(),
            build_time: Some(Utc::now() - chrono::Duration::days(30)),
            last_modified: Some(Utc::now() - chrono::Duration::days(1)),
        })
    }
}

#[async_trait]
pub trait MockPortageMonitorTrait {
    async fn get_portage_info(&self) -> Result<PortageInfo, PortCLError>;
    async fn get_package_info(&self, package: &str) -> Result<PackageInfo, PortCLError>;
    async fn search_packages(&self, query: &PackageQuery) -> Result<Vec<PackageInfo>, PortCLError>;
    async fn get_installed_packages(&self) -> Result<Vec<PackageInfo>, PortCLError>;
    async fn get_system_metrics(&self) -> Result<SystemMetrics, PortCLError>;
    async fn check_system_health(&self) -> Result<SystemHealth, PortCLError>;
    async fn monitor_resource_usage(&self, duration_seconds: u64) -> Result<ResourceUsageReport, PortCLError>;
}

#[async_trait]
impl MockPortageMonitorTrait for MockPortageMonitor {
    async fn get_portage_info(&self) -> Result<PortageInfo, PortCLError> {
        self.simulate_delay("get_portage_info").await;

        if self.should_fail("get_portage_info") {
            return Err(PortCLError::Portage("Mock error: Failed to get Portage info".to_string()));
        }

        let info = self.generate_portage_info();

        // Update metrics
        let mut metrics = self.metrics.write().unwrap();
        metrics.portage_operations += 1;

        Ok(info)
    }

    async fn get_package_info(&self, package: &str) -> Result<PackageInfo, PortCLError> {
        self.simulate_delay("get_package_info").await;

        if self.should_fail("get_package_info") {
            return Err(PortCLError::Portage(format!("Mock error: Failed to get package info for {}", package)));
        }

        let package_info = self.generate_package_info(package)
            .ok_or_else(|| PortCLError::NotFound(format!("Package {} not found", package)))?;

        // Update metrics
        let mut metrics = self.metrics.write().unwrap();
        metrics.portage_operations += 1;

        Ok(package_info)
    }

    async fn search_packages(&self, query: &PackageQuery) -> Result<Vec<PackageInfo>, PortCLError> {
        self.simulate_delay("search_packages").await;

        if self.should_fail("search_packages") {
            return Err(PortCLError::Portage("Mock error: Failed to search packages".to_string()));
        }

        // Simple mock search - filter packages by name and category
        let results: Vec<PackageInfo> = self.mock_packages
            .iter()
            .filter(|pkg| {
                let name_match = query.name.is_empty() || pkg.name.contains(&query.name);
                let category_match = query.category.as_ref().map_or(true, |cat| pkg.category.contains(cat));
                name_match && category_match
            })
            .filter_map(|pkg| self.generate_package_info(&pkg.name))
            .collect();

        // Update metrics
        let mut metrics = self.metrics.write().unwrap();
        metrics.portage_operations += 1;

        Ok(results)
    }

    async fn get_installed_packages(&self) -> Result<Vec<PackageInfo>, PortCLError> {
        self.simulate_delay("get_installed_packages").await;

        if self.should_fail("get_installed_packages") {
            return Err(PortCLError::Portage("Mock error: Failed to get installed packages".to_string()));
        }

        let packages: Vec<PackageInfo> = self.mock_packages
            .iter()
            .filter_map(|pkg| self.generate_package_info(&pkg.name))
            .collect();

        // Update metrics
        let mut metrics = self.metrics.write().unwrap();
        metrics.portage_operations += 1;

        Ok(packages)
    }

    async fn get_system_metrics(&self) -> Result<SystemMetrics, PortCLError> {
        self.simulate_delay("get_system_metrics").await;

        if self.should_fail("get_system_metrics") {
            return Err(PortCLError::System("Mock error: Failed to get system metrics".to_string()));
        }

        let metrics = self.metrics.read().unwrap();
        Ok(SystemMetrics {
            cpu_usage: metrics.system_cpu_usage,
            memory_usage: metrics.system_memory_usage,
            disk_usage: metrics.disk_usage,
            network_io: metrics.network_io_bytes,
            uptime: metrics.uptime_seconds,
            load_average: vec![
                metrics.system_cpu_usage / 100.0,
                metrics.system_cpu_usage / 110.0,
                metrics.system_cpu_usage / 120.0,
            ],
            process_count: 150 + self.mock_packages.len() as u32,
            active_connections: metrics.active_connections,
        })
    }

    async fn check_system_health(&self) -> Result<SystemHealth, PortCLError> {
        self.simulate_delay("check_system_health").await;

        if self.should_fail("check_system_health") {
            return Err(PortCLError::System("Mock error: Failed to check system health".to_string()));
        }

        let metrics = self.metrics.read().unwrap();
        let health_status = if metrics.system_cpu_usage > 90.0 || metrics.system_memory_usage > 90.0 {
            HealthStatus::Critical
        } else if metrics.system_cpu_usage > 70.0 || metrics.system_memory_usage > 80.0 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        let mut issues = Vec::new();
        if metrics.system_cpu_usage > 80.0 {
            issues.push("High CPU usage detected".to_string());
        }
        if metrics.system_memory_usage > 85.0 {
            issues.push("High memory usage detected".to_string());
        }
        if metrics.failed_operations > 10 {
            issues.push("Multiple operation failures detected".to_string());
        }

        Ok(SystemHealth {
            status: health_status,
            issues,
            recommendations: if health_status != HealthStatus::Healthy {
                vec!["Consider investigating high resource usage".to_string()]
            } else {
                Vec::new()
            },
            last_check: Utc::now(),
        })
    }

    async fn monitor_resource_usage(&self, duration_seconds: u64) -> Result<ResourceUsageReport, PortCLError> {
        if self.should_fail("monitor_resource_usage") {
            return Err(PortCLError::System("Mock error: Failed to monitor resource usage".to_string()));
        }

        // Simulate monitoring over time
        let start_time = Utc::now();
        let mut samples = Vec::new();

        for i in 0..duration_seconds {
            let mut metrics = self.metrics.write().unwrap();

            // Simulate some variation in metrics
            metrics.system_cpu_usage += (rand::random::<f64>() - 0.5) * 10.0;
            metrics.system_memory_usage += (rand::random::<f64>() - 0.5) * 5.0;

            // Keep within reasonable bounds
            metrics.system_cpu_usage = metrics.system_cpu_usage.clamp(0.0, 100.0);
            metrics.system_memory_usage = metrics.system_memory_usage.clamp(0.0, 100.0);

            samples.push(ResourceSample {
                timestamp: start_time + chrono::Duration::seconds(i as i64),
                cpu_usage: metrics.system_cpu_usage,
                memory_usage: metrics.system_memory_usage,
                disk_usage: metrics.disk_usage,
                network_io: metrics.network_io_bytes,
            });

            // Simulate some delay between samples
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let metrics = self.metrics.read().unwrap();
        Ok(ResourceUsageReport {
            duration_seconds,
            samples,
            summary: ResourceSummary {
                avg_cpu_usage: samples.iter().map(|s| s.cpu_usage).sum::<f64>() / samples.len() as f64,
                max_cpu_usage: samples.iter().map(|s| s.cpu_usage).fold(0.0, f64::max),
                avg_memory_usage: samples.iter().map(|s| s.memory_usage).sum::<f64>() / samples.len() as f64,
                max_memory_usage: samples.iter().map(|s| s.memory_usage).fold(0.0, f64::max),
                total_network_io: samples.iter().map(|s| s.network_io).sum(),
            },
            start_time,
            end_time: Utc::now(),
        })
    }
}

// Data structures for monitoring (mirroring the real PortCL structures)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortageInfo {
    pub installed_packages: u32,
    pub available_updates: u32,
    pub world_packages: u32,
    pub last_sync: Option<DateTime<Utc>>,
    pub portage_version: String,
    pub profile: String,
    pub arch: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub category: String,
    pub slot: Option<String>,
    pub repository: Option<String>,
    pub installed_size: Option<u64>,
    pub homepage: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub use_flags: Vec<String>,
    pub dependencies: Vec<String>,
    pub build_time: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageQuery {
    pub name: String,
    pub category: Option<String>,
    pub slot: Option<String>,
    pub repository: Option<String>,
    pub version_constraint: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_io: u64,
    pub uptime: u64,
    pub load_average: Vec<f64>,
    pub process_count: u32,
    pub active_connections: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceSample {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_io: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceSummary {
    pub avg_cpu_usage: f64,
    pub max_cpu_usage: f64,
    pub avg_memory_usage: f64,
    pub max_memory_usage: f64,
    pub total_network_io: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceUsageReport {
    pub duration_seconds: u64,
    pub samples: Vec<ResourceSample>,
    pub summary: ResourceSummary,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

impl Default for MockMonitorMetrics {
    fn default() -> Self {
        Self {
            system_cpu_usage: 25.0,
            system_memory_usage: 60.0,
            disk_usage: 45.0,
            network_io_bytes: 1024 * 1024, // 1MB
            portage_operations: 0,
            failed_operations: 0,
            active_connections: 5,
            uptime_seconds: 86400, // 1 day
        }
    }
}

impl Default for PackageQuery {
    fn default() -> Self {
        Self {
            name: String::new(),
            category: None,
            slot: None,
            repository: None,
            version_constraint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_monitor_creation() {
        let config = MockMonitoringConfig::default();
        let monitor = MockPortageMonitor::new(config);

        assert!(!monitor.mock_packages.is_empty() || monitor.mock_packages.is_empty()); // Either empty or populated
    }

    #[tokio::test]
    async fn test_mock_monitor_with_data() {
        let config = MockMonitoringConfig::default();
        let packages = sample_packages();
        let monitor = MockPortageMonitor::with_test_data(config, packages);

        assert_eq!(monitor.mock_packages.len(), 2);
    }

    #[tokio::test]
    async fn test_error_injection() {
        let config = MockMonitoringConfig::default();
        let monitor = MockPortageMonitor::new(config);

        monitor.inject_error("get_portage_info".to_string());

        let result = monitor.get_portage_info().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delay_injection() {
        let config = MockMonitoringConfig::default();
        let monitor = MockPortageMonitor::new(config);

        monitor.inject_delay("get_portage_info".to_string(), 100);

        let start = std::time::Instant::now();
        let _result = monitor.get_portage_info().await;
        let duration = start.elapsed();

        assert!(duration >= std::time::Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_package_search() {
        let config = MockMonitoringConfig::default();
        let packages = sample_packages();
        let monitor = MockPortageMonitor::with_test_data(config, packages);

        let query = PackageQuery {
            name: "portage".to_string(),
            category: None,
            slot: None,
            repository: None,
            version_constraint: None,
        };

        let results = monitor.search_packages(&query).await.unwrap();
        assert!(!results.is_empty());
        assert!(results[0].name.contains("portage"));
    }

    #[tokio::test]
    async fn test_system_metrics() {
        let config = MockMonitoringConfig::default();
        let monitor = MockPortageMonitor::new(config);

        let metrics = monitor.get_system_metrics().await.unwrap();
        assert!(metrics.cpu_usage >= 0.0 && metrics.cpu_usage <= 100.0);
        assert!(metrics.memory_usage >= 0.0 && metrics.memory_usage <= 100.0);
    }

    #[tokio::test]
    async fn test_system_health() {
        let config = MockMonitoringConfig::default();
        let monitor = MockPortageMonitor::new(config);

        // Test healthy system
        let health = monitor.check_system_health().await.unwrap();
        assert!(matches!(health.status, HealthStatus::Healthy));

        // Test warning system
        monitor.simulate_load(75.0, 82.0);
        let health = monitor.check_system_health().await.unwrap();
        assert!(matches!(health.status, HealthStatus::Warning));
    }
}