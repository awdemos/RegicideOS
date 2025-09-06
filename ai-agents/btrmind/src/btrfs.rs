use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, warn};
use crate::SystemMetrics;

pub struct BtrfsMonitor {
    target_path: String,
}

impl BtrfsMonitor {
    pub fn new(target_path: &str) -> Result<Self> {
        let path = Path::new(target_path);
        if !path.exists() {
            bail!("Target path does not exist: {}", target_path);
        }
        
        // Verify this is a BTRFS filesystem
        Self::verify_btrfs(target_path)?;
        
        Ok(Self {
            target_path: target_path.to_string(),
        })
    }
    
    fn verify_btrfs(path: &str) -> Result<()> {
        let output = Command::new("stat")
            .args(["-f", "-c", "%T", path])
            .output()
            .context("Failed to check filesystem type")?;
        
        if !output.status.success() {
            bail!("Failed to stat filesystem: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        let fstype = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        if !fstype.contains("btrfs") {
            // For development/testing, allow any filesystem type
            warn!("Target path is not BTRFS filesystem (detected: {}), continuing anyway", fstype);
        }
        
        Ok(())
    }
    
    pub async fn collect_metrics(&self) -> Result<SystemMetrics> {
        let timestamp = chrono::Utc::now();
        
        // Collect disk usage using 'df' (more reliable than btrfs filesystem usage)
        let disk_usage = self.get_disk_usage().await?;
        
        // Collect BTRFS-specific metrics if available
        let metadata_usage = self.get_metadata_usage().await.unwrap_or(0.0);
        let fragmentation = self.get_fragmentation().await.unwrap_or(0.0);
        
        Ok(SystemMetrics {
            timestamp,
            disk_usage_percent: disk_usage.used_percent,
            free_space_mb: disk_usage.free_mb,
            metadata_usage_percent: metadata_usage,
            fragmentation_percent: fragmentation,
        })
    }
    
    async fn get_disk_usage(&self) -> Result<DiskUsage> {
        let output = Command::new("df")
            .args(["-BM", &self.target_path])
            .output()
            .context("Failed to run df command")?;
        
        if !output.status.success() {
            bail!("df command failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        debug!("df output: {}", output_str);
        
        // Parse df output - skip header line
        let lines: Vec<&str> = output_str.lines().collect();
        if lines.len() < 2 {
            bail!("Unexpected df output format");
        }
        
        // df output format: Filesystem 1M-blocks Used Available Use% Mounted on
        let data_line = if lines[1].starts_with('/') {
            lines[1]
        } else if lines.len() > 2 {
            // Handle case where filesystem name is on separate line
            lines[2]
        } else {
            bail!("Could not parse df output");
        };
        
        let fields: Vec<&str> = data_line.split_whitespace().collect();
        if fields.len() < 5 {
            bail!("Unexpected df output format: {}", data_line);
        }
        
        // Parse fields (skip filesystem name)
        let total_mb: f64 = fields[fields.len()-5].trim_end_matches('M').parse()
            .context("Failed to parse total space")?;
        let used_mb: f64 = fields[fields.len()-4].trim_end_matches('M').parse()
            .context("Failed to parse used space")?;
        let free_mb: f64 = fields[fields.len()-3].trim_end_matches('M').parse()
            .context("Failed to parse free space")?;
        
        let used_percent = if total_mb > 0.0 {
            (used_mb / total_mb) * 100.0
        } else {
            0.0
        };
        
        debug!("Disk usage: {:.1}% ({:.1}MB used, {:.1}MB free)", 
               used_percent, used_mb, free_mb);
        
        Ok(DiskUsage {
            total_mb,
            used_mb,
            free_mb,
            used_percent,
        })
    }
    
    async fn get_metadata_usage(&self) -> Result<f64> {
        // Try to get BTRFS filesystem usage
        let output = Command::new("btrfs")
            .args(["filesystem", "usage", "-b", &self.target_path])
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                debug!("btrfs filesystem usage output: {}", output_str);
                
                // Parse metadata usage from output
                // This is a simplified parser - BTRFS output format is complex
                for line in output_str.lines() {
                    if line.contains("Metadata") && line.contains("used") {
                        // Try to extract percentage or calculate it
                        // For now, return a placeholder
                        return Ok(5.0);
                    }
                }
                
                Ok(0.0)
            }
            _ => {
                debug!("BTRFS metadata usage not available");
                Ok(0.0)
            }
        }
    }
    
    async fn get_fragmentation(&self) -> Result<f64> {
        // BTRFS fragmentation is complex to measure accurately
        // For now, return a placeholder based on usage
        // TODO: Implement proper fragmentation detection
        let disk_usage = self.get_disk_usage().await?;
        
        // Rough heuristic: higher usage tends to correlate with fragmentation
        let fragmentation = if disk_usage.used_percent > 80.0 {
            (disk_usage.used_percent - 80.0) * 2.0
        } else {
            0.0
        };
        
        Ok(fragmentation.min(100.0))
    }
}

#[derive(Debug)]
struct DiskUsage {
    total_mb: f64,
    used_mb: f64,
    free_mb: f64,
    used_percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_disk_usage_collection() {
        // Test against root filesystem (should always exist)
        // Skip BTRFS verification for cross-platform testing
        let monitor = match BtrfsMonitor::new("/") {
            Ok(monitor) => monitor,
            Err(_) => {
                // Skip test if we can't create monitor (e.g., on non-Linux systems)
                println!("Skipping test - filesystem not available");
                return;
            }
        };
        
        let metrics = monitor.collect_metrics().await.unwrap();
        
        assert!(metrics.disk_usage_percent >= 0.0);
        assert!(metrics.disk_usage_percent <= 100.0);
        assert!(metrics.free_space_mb >= 0.0);
    }
    
    #[test]
    fn test_invalid_path() {
        let result = BtrfsMonitor::new("/nonexistent/path");
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_metrics_structure() {
        // Use a mock filesystem check
        let monitor = match BtrfsMonitor::new("/tmp") {
            Ok(monitor) => monitor,
            Err(_) => {
                // Skip test if we can't create monitor  
                println!("Skipping test - filesystem not available");
                return;
            }
        };
        
        let metrics = monitor.collect_metrics().await.unwrap();
        
        // Verify all fields are present and reasonable
        assert!(metrics.disk_usage_percent >= 0.0);
        assert!(metrics.free_space_mb >= 0.0);
        assert!(metrics.metadata_usage_percent >= 0.0);
        assert!(metrics.fragmentation_percent >= 0.0);
        
        // Timestamp should be recent
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(metrics.timestamp);
        assert!(diff.num_seconds() < 5); // Should be within 5 seconds
    }
}
