use crate::config::MonitoringConfig;
use crate::error::{PortCLError, Result};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sysinfo::System;
use std::path::Path;
use std::fs;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortageMetrics {
    pub timestamp: DateTime<Utc>,
    pub portage_info: super::portage::PortageInfo,
    pub system_metrics: SystemMetrics,
    pub recent_events: Vec<super::events::PortageEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_total_gb: f64,
    pub memory_used_gb: f64,
    pub disk_usage_percent: f64,
    pub disk_total_gb: f64,
    pub disk_used_gb: f64,
    pub disk_free_gb: f64,
    pub load_average_1min: f64,
    pub load_average_5min: f64,
    pub load_average_15min: f64,
    pub network_io: NetworkIo,
    pub process_count: u32,
    pub uptime_seconds: u64,
    pub temperature_celsius: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIo {
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationMetrics {
    pub active_compilations: u32,
    pub total_compile_time_seconds: u64,
    pub average_compile_time_seconds: f64,
    pub success_rate_percent: f64,
    pub failed_compilations: u32,
    pub parallel_jobs: u32,
    pub cpu_usage_during_compile: f64,
    pub memory_usage_during_compile: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub portage_distdir_size_gb: f64,
    pub portage_pkgdir_size_gb: f64,
    pub temporary_files_size_gb: f64,
    pub write_speed_mb_s: f64,
    pub read_speed_mb_s: f64,
    pub iops: u64,
}

pub struct MetricsCollector {
    config: MonitoringConfig,
    system: System,
}

impl MetricsCollector {
    pub fn new(config: MonitoringConfig) -> Result<Self> {
        Ok(Self {
            config,
            system: System::new_all(),
        })
    }

    pub async fn collect_system_metrics(&mut self) -> Result<SystemMetrics> {
        debug!("Collecting system metrics");

        self.system.refresh_all();

        let cpu_usage = self.get_cpu_usage();
        let memory_usage = self.get_memory_usage();
        let disk_usage = self.get_disk_usage();
        let load_average = self.get_load_average();
        let network_io = self.get_network_io();
        let process_count = self.get_process_count();
        let uptime = self.get_uptime();
        let temperature = self.get_temperature();

        Ok(SystemMetrics {
            cpu_usage_percent: cpu_usage,
            memory_usage_percent: memory_usage.percent,
            memory_total_gb: memory_usage.total,
            memory_used_gb: memory_usage.used,
            disk_usage_percent: disk_usage.percent,
            disk_total_gb: disk_usage.total,
            disk_used_gb: disk_usage.used,
            disk_free_gb: disk_usage.free,
            load_average_1min: load_average.0,
            load_average_5min: load_average.1,
            load_average_15min: load_average.2,
            network_io,
            process_count,
            uptime_seconds: uptime,
            temperature_celsius: temperature,
        })
    }

    pub async fn collect_compilation_metrics(&self) -> Result<CompilationMetrics> {
        debug!("Collecting compilation metrics");

        // Check for active emerge processes
        let active_compilations = self.count_active_emerge_processes().await?;
        let total_compile_time = self.get_total_compile_time().await?;
        let success_rate = self.get_compilation_success_rate().await?;
        let failed_compilations = self.get_failed_compilations_count().await?;
        let parallel_jobs = self.get_parallel_jobs_count().await?;

        let average_compile_time = if active_compilations > 0 {
            total_compile_time as f64 / active_compilations as f64
        } else {
            0.0
        };

        // Get resource usage during compilation
        let (cpu_usage, memory_usage) = self.get_compilation_resource_usage().await?;

        Ok(CompilationMetrics {
            active_compilations,
            total_compile_time_seconds: total_compile_time,
            average_compile_time_seconds: average_compile_time,
            success_rate_percent: success_rate,
            failed_compilations,
            parallel_jobs,
            cpu_usage_during_compile: cpu_usage,
            memory_usage_during_compile: memory_usage,
        })
    }

    pub async fn collect_disk_metrics(&self) -> Result<DiskMetrics> {
        debug!("Collecting disk metrics");

        let portage_distdir_size = self.get_directory_size("/usr/portage/distfiles").await?;
        let portage_pkgdir_size = self.get_directory_size("/var/db/pkg").await?;
        let temp_files_size = self.get_directory_size("/var/tmp/portage").await?;

        let (write_speed, read_speed) = self.get_disk_speed().await?;
        let iops = self.get_iops().await?;

        Ok(DiskMetrics {
            portage_distdir_size_gb: portage_distdir_size,
            portage_pkgdir_size_gb: portage_pkgdir_size,
            temporary_files_size_gb: temp_files_size,
            write_speed_mb_s: write_speed,
            read_speed_mb_s: read_speed,
            iops,
        })
    }

    fn get_cpu_usage(&self) -> f64 {
        let mut total_usage = 0.0;
        let cpu_count = self.system.cpus().len();

        for cpu in self.system.cpus() {
            total_usage += cpu.cpu_usage();
        }

        if cpu_count > 0 {
            total_usage / cpu_count as f64
        } else {
            0.0
        }
    }

    fn get_memory_usage(&self) -> MemoryUsage {
        let total_memory = self.system.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0); // GB
        let used_memory = self.system.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0); // GB

        let usage_percent = if total_memory > 0.0 {
            (used_memory / total_memory) * 100.0
        } else {
            0.0
        };

        MemoryUsage {
            total: total_memory,
            used: used_memory,
            percent: usage_percent,
        }
    }

    fn get_disk_usage(&self) -> DiskUsage {
        let mut total_disk = 0.0;
        let mut used_disk = 0.0;

        for disk in self.system.disks() {
            total_disk += disk.total_space() as f64 / (1024.0 * 1024.0 * 1024.0); // GB
            used_disk += disk.available_space() as f64 / (1024.0 * 1024.0 * 1024.0); // GB
        }

        let free_disk = total_disk - used_disk;
        let usage_percent = if total_disk > 0.0 {
            (used_disk / total_disk) * 100.0
        } else {
            0.0
        };

        DiskUsage {
            total: total_disk,
            used: used_disk,
            free: free_disk,
            percent: usage_percent,
        }
    }

    fn get_load_average(&self) -> (f64, f64, f64) {
        let load_avg = self.system.load_average();
        (load_avg.one, load_avg.five, load_avg.fifteen)
    }

    fn get_network_io(&self) -> NetworkIo {
        let mut bytes_received = 0;
        let mut bytes_transmitted = 0;
        let mut packets_received = 0;
        let mut packets_transmitted = 0;

        for (_interface_name, data) in self.system.networks() {
            bytes_received += data.received();
            bytes_transmitted += data.transmitted();
            packets_received += data.packets_received();
            packets_transmitted += data.packets_transmitted();
        }

        NetworkIo {
            bytes_received,
            bytes_transmitted,
            packets_received,
            packets_transmitted,
        }
    }

    fn get_process_count(&self) -> u32 {
        self.system.processes().len() as u32
    }

    fn get_uptime(&self) -> u64 {
        self.system.uptime()
    }

    fn get_temperature(&self) -> Option<f64> {
        // Try to read CPU temperature from sysfs
        let thermal_path = "/sys/class/thermal/thermal_zone0/temp";
        if Path::new(thermal_path).exists() {
            if let Ok(content) = fs::read_to_string(thermal_path) {
                if let Ok(temp_millis) = content.trim().parse::<i64>() {
                    return Some(temp_millis as f64 / 1000.0);
                }
            }
        }
        None
    }

    async fn count_active_emerge_processes(&self) -> Result<u32> {
        use std::process::Command;

        let output = Command::new("pgrep")
            .args(["-f", "emerge"])
            .output()
            .map_err(|e| PortCLError::System(format!("Failed to check emerge processes: {}", e)))?;

            Ok(String::from_utf8_lossy(&output.stdout).lines().count() as u32)
    }

    async fn get_total_compile_time(&self) -> Result<u64> {
        // This would need to track compilation times from emerge logs
        // For now, return a placeholder
        Ok(0)
    }

    async fn get_compilation_success_rate(&self) -> Result<f64> {
        // Parse emerge log to calculate success rate
        let emerge_log_path = "/var/log/emerge.log";
        if !Path::new(emerge_log_path).exists() {
            return Ok(100.0); // Assume 100% if no log
        }

        // Placeholder implementation
        // In practice, you'd parse the log to count successes vs failures
        Ok(95.0)
    }

    async fn get_failed_compilations_count(&self) -> Result<u32> {
        // Parse emerge log for compilation failures
        Ok(0)
    }

    async fn get_parallel_jobs_count(&self) -> Result<u32> {
        use std::process::Command;

        // Get MAKEOPTS from make.conf or environment
        let output = Command::new("portageq")
            .args(["envvar", "MAKEOPTS"])
            .output()
            .map_err(|e| PortCLError::System(format!("Failed to get MAKEOPTS: {}", e)))?;

        let makeopts = String::from_utf8_lossy(&output.stdout);

        // Parse MAKEOPTS to extract -j value
        for part in makeopts.split_whitespace() {
            if part.starts_with("-j") {
                let jobs = if part.len() > 2 {
                    part[2..].parse().unwrap_or(1)
                } else {
                    1
                };
                return Ok(jobs);
            }
        }

        Ok(1)
    }

    async fn get_compilation_resource_usage(&self) -> Result<(f64, f64)> {
        // Get CPU and memory usage during compilation
        // This is a simplified implementation
        Ok((75.0, 60.0)) // Placeholder values
    }

    async fn get_directory_size(&self, path: &str) -> Result<f64> {
        use std::process::Command;

        if !Path::new(path).exists() {
            return Ok(0.0);
        }

        let output = Command::new("du")
            .args(["-sb", path])
            .output()
            .map_err(|e| PortCLError::System(format!("Failed to get directory size: {}", e)))?;

        let content = String::from_utf8_lossy(&output.stdout);
        let size_str = content.split_whitespace().next().unwrap_or("0");
        let size_bytes = size_str.parse::<u64>().unwrap_or(0);

        Ok(size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) // Convert to GB
    }

    async fn get_disk_speed(&self) -> Result<(f64, f64)> {
        // This would require disk performance monitoring
        // For now, return placeholder values
        Ok((100.0, 150.0)) // write_speed, read_speed in MB/s
    }

    async fn get_iops(&self) -> Result<u64> {
        // This would require I/O performance monitoring
        Ok(1000)
    }
}

#[derive(Debug)]
struct MemoryUsage {
    total: f64,
    used: f64,
    percent: f64,
}

#[derive(Debug)]
struct DiskUsage {
    total: f64,
    used: f64,
    free: f64,
    percent: f64,
}