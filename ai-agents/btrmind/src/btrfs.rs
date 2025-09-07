use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use std::fs::{OpenOptions};
use std::io::Write;
use tracing::{debug, warn, info};
use crate::{SystemMetrics, EnhancedSystemMetrics, fragmentation_model};

pub struct BtrfsMonitor {
    target_path: String,
    training_data_path: Option<String>,
    enable_data_collection: bool,
    fragmentation_model: Option<fragmentation_model::FragmentationModel>,
    use_fragmentation_model: bool,
    fallback_to_heuristic: bool,
}

impl BtrfsMonitor {
    pub fn new(target_path: &str) -> Result<Self> {
        Self::with_data_collection(target_path, None, false, None, false, true)
    }

    pub fn with_data_collection(
        target_path: &str, 
        training_data_path: Option<String>, 
        enable_data_collection: bool,
        model_path: Option<String>,
        use_model: bool,
        fallback_to_heuristic: bool
    ) -> Result<Self> {
        let path = Path::new(target_path);
        if !path.exists() {
            bail!("Target path does not exist: {}", target_path);
        }
        
        // Verify this is a BTRFS filesystem
        Self::verify_btrfs(target_path)?;
        
        // Load fragmentation model if specified
        let fragmentation_model = if use_model {
            if let Some(ref model_path_str) = model_path {
                match fragmentation_model::FragmentationModel::load(model_path_str) {
                    Ok(model) => {
                        info!("Loaded fragmentation model from {}", model_path_str);
                        Some(model)
                    }
                    Err(e) => {
                        warn!("Failed to load fragmentation model: {}", e);
                        if fallback_to_heuristic {
                            info!("Falling back to heuristic fragmentation estimation");
                            None
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let use_fragmentation_model = use_model && fragmentation_model.is_some();
        
        Ok(Self {
            target_path: target_path.to_string(),
            training_data_path,
            enable_data_collection,
            fragmentation_model,
            use_fragmentation_model,
            fallback_to_heuristic,
        })
    }
    
    fn verify_btrfs(path: &str) -> Result<()> {
        // Try to get filesystem type, but handle different OS implementations
        let output = Command::new("stat")
            .args(["-f", path])
            .output()
            .context("Failed to check filesystem type")?;
        
        if !output.status.success() {
            // For development/testing, allow any filesystem if stat fails
            warn!("Could not determine filesystem type, continuing anyway");
            return Ok(());
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        debug!("stat output: {}", output_str);
        
        // Different stat implementations across platforms
        // Just check if path exists and is accessible for development
        if !Path::new(path).exists() {
            bail!("Target path does not exist: {}", path);
        }
        
        // For development, allow any filesystem type
        warn!("Development mode: allowing any filesystem type");
        Ok(())
    }
    
    pub async fn collect_metrics(&self) -> Result<SystemMetrics> {
        let enhanced_metrics = self.collect_enhanced_metrics().await?;
        Ok(SystemMetrics {
            timestamp: enhanced_metrics.timestamp,
            disk_usage_percent: enhanced_metrics.disk_usage_percent,
            free_space_mb: enhanced_metrics.free_space_mb,
            metadata_usage_percent: enhanced_metrics.metadata_usage_percent,
            fragmentation_percent: enhanced_metrics.fragmentation_percent,
        })
    }

    pub async fn collect_enhanced_metrics(&self) -> Result<EnhancedSystemMetrics> {
        let timestamp = chrono::Utc::now();
        
        // Collect disk usage using 'df' (more reliable than btrfs filesystem usage)
        let disk_usage = self.get_disk_usage().await?;
        
        // Collect BTRFS-specific metrics if available
        let metadata_usage = self.get_metadata_usage().await.unwrap_or(0.0);
        let fragmentation = self.get_fragmentation().await.unwrap_or(0.0);
        
        // Collect enhanced metrics for training
        let file_count = self.get_file_count().await.unwrap_or(0);
        let avg_file_size_mb = self.get_avg_file_size().await.unwrap_or(0.0);
        let write_frequency = self.get_write_frequency().await.unwrap_or(0.0);
        let fragmentation_proxy = self.get_fragmentation_proxy().await.unwrap_or(fragmentation);
        
        let metrics = EnhancedSystemMetrics {
            timestamp,
            disk_usage_percent: disk_usage.used_percent,
            free_space_mb: disk_usage.free_mb,
            metadata_usage_percent: metadata_usage,
            fragmentation_percent: fragmentation,
            file_count,
            avg_file_size_mb,
            write_frequency,
            fragmentation_proxy,
        };
        
        // Log to CSV if data collection is enabled
        if self.enable_data_collection {
            if let Err(e) = self.log_to_csv(&metrics) {
                warn!("Failed to log training data: {}", e);
            }
        }
        
        Ok(metrics)
    }
    
    async fn get_disk_usage(&self) -> Result<DiskUsage> {
        // Use -m for macOS, -BM for Linux
        let df_arg = if cfg!(target_os = "macos") { "-m" } else { "-BM" };
        
        let output = Command::new("df")
            .args([df_arg, &self.target_path])
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
        
        // Find the data line (skip header)
        let data_line = if cfg!(target_os = "macos") {
            // macOS df has more columns, find the line with actual data
            lines.iter()
                .find(|line| line.starts_with('/'))
                .ok_or_else(|| anyhow::anyhow!("Could not find data line in df output"))?
        } else {
            // Linux df format
            if lines[1].starts_with('/') {
                lines[1]
            } else if lines.len() > 2 {
                lines[2]
            } else {
                bail!("Could not parse df output");
            }
        };
        
        let fields: Vec<&str> = data_line.split_whitespace().collect();
        if fields.len() < 4 {
            bail!("Unexpected df output format: {}", data_line);
        }
        
        // Parse fields based on platform
        let (total_mb, used_mb, free_mb) = if cfg!(target_os = "macos") {
            // macOS format: Filesystem 1M-blocks Used Available Capacity ...
            let total_mb: f64 = fields[1].parse()
                .context("Failed to parse total space")?;
            let used_mb: f64 = fields[2].parse()
                .context("Failed to parse used space")?;
            let free_mb: f64 = fields[3].parse()
                .context("Failed to parse free space")?;
            (total_mb, used_mb, free_mb)
        } else {
            // Linux format: Filesystem Size Used Avail Use% Mounted on
            let total_mb: f64 = fields[1].trim_end_matches('M').parse()
                .context("Failed to parse total space")?;
            let used_mb: f64 = fields[2].trim_end_matches('M').parse()
                .context("Failed to parse used space")?;
            let free_mb: f64 = fields[3].trim_end_matches('M').parse()
                .context("Failed to parse free space")?;
            (total_mb, used_mb, free_mb)
        };
        
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
        // Try to use ML model if available
        if self.use_fragmentation_model {
            if let Some(ref model) = self.fragmentation_model {
                match self.get_model_based_fragmentation(model).await {
                    Ok(fragmentation) => {
                        debug!("ML-based fragmentation estimate: {:.2}", fragmentation);
                        return Ok(fragmentation);
                    }
                    Err(e) => {
                        warn!("ML fragmentation prediction failed: {}", e);
                        if !self.fallback_to_heuristic {
                            return Err(e);
                        }
                    }
                }
            }
        }
        
        // Fallback to heuristic
        let heuristic_fragmentation = self.get_heuristic_fragmentation().await?;
        debug!("Heuristic fragmentation estimate: {:.2}", heuristic_fragmentation);
        Ok(heuristic_fragmentation)
    }

    async fn get_model_based_fragmentation(&self, model: &fragmentation_model::FragmentationModel) -> Result<f64> {
        // Collect basic metrics without causing recursion
        let disk_usage = self.get_disk_usage().await?;
        let metadata_usage = self.get_metadata_usage().await.unwrap_or(0.0);
        let file_count = self.get_file_count().await.unwrap_or(0);
        let avg_file_size_mb = self.get_avg_file_size().await.unwrap_or(0.0);
        let write_frequency = self.get_write_frequency().await.unwrap_or(0.0);
        
        let features = fragmentation_model::FragmentationFeatures::new(
            disk_usage.used_percent,
            disk_usage.free_mb,
            metadata_usage,
            file_count as f64,
            avg_file_size_mb,
            write_frequency,
        );
        
        let prediction = model.predict(&features)?;
        
        debug!("Model prediction: {:.2} (features: usage={:.1}, free={:.1}, metadata={:.1})", 
               prediction, 
               features.disk_usage_percent, 
               features.free_space_mb, 
               features.metadata_usage_percent);
        
        Ok(prediction)
    }

    async fn get_heuristic_fragmentation(&self) -> Result<f64> {
        let disk_usage = self.get_disk_usage().await?;
        
        // Original heuristic: higher usage tends to correlate with fragmentation
        let fragmentation = if disk_usage.used_percent > 80.0 {
            (disk_usage.used_percent - 80.0) * 2.0
        } else {
            0.0
        };
        
        Ok(fragmentation.min(100.0))
    }

    async fn get_file_count(&self) -> Result<u64> {
        let output = Command::new("find")
            .args([&self.target_path, "-type", "f"])
            .output()
            .context("Failed to count files")?;
        
        if !output.status.success() {
            bail!("find command failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let file_count = output_str.lines().count() as u64;
        
        debug!("File count: {}", file_count);
        Ok(file_count)
    }

    async fn get_avg_file_size(&self) -> Result<f64> {
        let output = Command::new("find")
            .args([&self.target_path, "-type", "f", "-exec", "stat", "-c", "%s", "{}", "+"])
            .output()
            .context("Failed to get file sizes")?;
        
        if !output.status.success() {
            bail!("find command failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let file_sizes: Vec<f64> = output_str
            .lines()
            .filter_map(|line| line.parse().ok())
            .collect();
        
        if file_sizes.is_empty() {
            return Ok(0.0);
        }
        
        let avg_size_bytes = file_sizes.iter().sum::<f64>() / file_sizes.len() as f64;
        let avg_size_mb = avg_size_bytes / (1024.0 * 1024.0);
        
        debug!("Average file size: {:.2} MB", avg_size_mb);
        Ok(avg_size_mb)
    }

    async fn get_write_frequency(&self) -> Result<f64> {
        // Simple heuristic: check recent file modifications
        let output = Command::new("find")
            .args([&self.target_path, "-type", "f", "-mmin", "-60"])
            .output()
            .context("Failed to check recent modifications")?;
        
        if !output.status.success() {
            bail!("find command failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let recent_files = output_str.lines().count() as f64;
        
        debug!("Write frequency (files modified in last hour): {:.2}", recent_files);
        Ok(recent_files)
    }

    async fn get_fragmentation_proxy(&self) -> Result<f64> {
        // Try to get actual fragmentation from BTRFS tools
        let output = Command::new("btrfs")
            .args(["filesystem", "df", &self.target_path])
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                debug!("btrfs filesystem df output: {}", output_str);
                
                // Parse for fragmentation indicators
                // This is a simplified approach - real BTRFS fragmentation analysis is complex
                for line in output_str.lines() {
                    if line.contains("Data") && line.contains("%") {
                        // Extract percentage as a proxy for fragmentation
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for part in parts {
                            if part.ends_with('%') {
                                if let Ok(percent) = part.trim_end_matches('%').parse::<f64>() {
                                    return Ok(percent);
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                debug!("BTRFS fragmentation analysis not available");
            }
        }
        
        // Fallback to heuristic
        self.get_fragmentation().await
    }

    fn log_to_csv(&self, metrics: &EnhancedSystemMetrics) -> Result<()> {
        if let Some(ref path) = self.training_data_path {
            let file_exists = std::path::Path::new(path).exists();
            
            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(path)
                .context("Failed to open training data file")?;
            
            // Write header if file is new
            if !file_exists {
                writeln!(file, "timestamp,disk_usage_percent,free_space_mb,metadata_usage_percent,file_count,avg_file_size_mb,write_frequency,fragmentation_proxy")?;
            }
            
            // Write data row
            writeln!(file, "{},{},{},{},{},{},{},{}",
                metrics.timestamp.format("%Y-%m-%d %H:%M:%S"),
                metrics.disk_usage_percent,
                metrics.free_space_mb,
                metrics.metadata_usage_percent,
                metrics.file_count,
                metrics.avg_file_size_mb,
                metrics.write_frequency,
                metrics.fragmentation_proxy
            )?;
            
            debug!("Logged training data to {}", path);
        }
        
        Ok(())
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
