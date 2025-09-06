use anyhow::{Context, Result};
use std::process::Command;
use tracing::{info, warn, debug};
use crate::config::ActionConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    NoOperation = 0,
    DeleteTempFiles = 1,
    CompressFiles = 2,
    BalanceMetadata = 3,
    CleanupSnapshots = 4,
}

impl Action {
    pub fn from_id(id: usize) -> Option<Self> {
        match id {
            0 => Some(Action::NoOperation),
            1 => Some(Action::DeleteTempFiles),
            2 => Some(Action::CompressFiles),
            3 => Some(Action::BalanceMetadata),
            4 => Some(Action::CleanupSnapshots),
            _ => None,
        }
    }
    
    pub fn all_actions() -> Vec<Action> {
        vec![
            Action::NoOperation,
            Action::DeleteTempFiles,
            Action::CompressFiles,
            Action::BalanceMetadata,
            Action::CleanupSnapshots,
        ]
    }
    
    pub fn action_count() -> usize {
        5
    }
}

pub struct ActionExecutor {
    config: ActionConfig,
    dry_run: bool,
}

impl ActionExecutor {
    pub fn new(config: ActionConfig, dry_run: bool) -> Self {
        Self { config, dry_run }
    }
    
    pub async fn execute_action(&self, action: Action) -> Result<ActionResult> {
        if self.dry_run {
            info!("DRY-RUN: Would execute action: {:?}", action);
            return Ok(ActionResult {
                action,
                success: true,
                space_freed_mb: 0.0,
                message: "Dry run - no action taken".to_string(),
            });
        }
        
        match action {
            Action::NoOperation => self.no_operation().await,
            Action::DeleteTempFiles => self.delete_temp_files().await,
            Action::CompressFiles => self.compress_files().await,
            Action::BalanceMetadata => self.balance_metadata().await,
            Action::CleanupSnapshots => self.cleanup_snapshots().await,
        }
    }
    
    async fn no_operation(&self) -> Result<ActionResult> {
        debug!("No operation - monitoring only");
        Ok(ActionResult {
            action: Action::NoOperation,
            success: true,
            space_freed_mb: 0.0,
            message: "No action taken".to_string(),
        })
    }
    
    async fn delete_temp_files(&self) -> Result<ActionResult> {
        if !self.config.enable_temp_cleanup {
            return Ok(ActionResult {
                action: Action::DeleteTempFiles,
                success: true,
                space_freed_mb: 0.0,
                message: "Temp cleanup disabled in config".to_string(),
            });
        }
        
        info!("Cleaning up temporary files");
        let mut total_freed = 0.0;
        let mut messages = Vec::new();
        
        for temp_path in &self.config.temp_paths {
            match self.cleanup_path(temp_path).await {
                Ok(freed) => {
                    total_freed += freed;
                    messages.push(format!("Cleaned {} ({:.1}MB freed)", temp_path, freed));
                }
                Err(e) => {
                    warn!("Failed to clean {}: {}", temp_path, e);
                    messages.push(format!("Failed to clean {}: {}", temp_path, e));
                }
            }
        }
        
        Ok(ActionResult {
            action: Action::DeleteTempFiles,
            success: true,
            space_freed_mb: total_freed,
            message: messages.join("; "),
        })
    }
    
    async fn cleanup_path(&self, path: &str) -> Result<f64> {
        // Get initial size
        let initial_size = self.get_directory_size(path).await.unwrap_or(0.0);
        
        // Handle glob patterns like /home/*/.cache
        if path.contains('*') {
            return self.cleanup_glob_pattern(path).await;
        }
        
        // Clean specific directories
        match path {
            "/tmp" | "/var/tmp" => {
                // Clean files older than 7 days
                let output = Command::new("find")
                    .args([path, "-type", "f", "-atime", "+7", "-delete"])
                    .output()
                    .context("Failed to clean temporary files")?;
                
                if !output.status.success() {
                    warn!("find command failed: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
            "/var/cache" => {
                // Clean package caches and other system caches
                self.clean_system_cache().await?;
            }
            _ => {
                // Generic cleanup for other paths
                debug!("Skipping cleanup for path: {}", path);
            }
        }
        
        // Calculate freed space
        let final_size = self.get_directory_size(path).await.unwrap_or(initial_size);
        let freed = (initial_size - final_size).max(0.0);
        
        debug!("Freed {:.1}MB from {}", freed, path);
        Ok(freed)
    }
    
    async fn cleanup_glob_pattern(&self, pattern: &str) -> Result<f64> {
        // Simple implementation for /home/*/.cache pattern
        if pattern == "/home/*/.cache" {
            let output = Command::new("find")
                .args(["/home", "-maxdepth", "2", "-type", "d", "-name", ".cache"])
                .output()
                .context("Failed to find cache directories")?;
            
            if output.status.success() {
                let cache_dirs = String::from_utf8_lossy(&output.stdout);
                let mut total_freed = 0.0;
                
                for cache_dir in cache_dirs.lines() {
                    if let Ok(freed) = self.cleanup_cache_directory(cache_dir).await {
                        total_freed += freed;
                    }
                }
                
                return Ok(total_freed);
            }
        }
        
        Ok(0.0)
    }
    
    async fn cleanup_cache_directory(&self, cache_dir: &str) -> Result<f64> {
        let initial_size = self.get_directory_size(cache_dir).await.unwrap_or(0.0);
        
        // Clean cache files older than 30 days
        let output = Command::new("find")
            .args([cache_dir, "-type", "f", "-atime", "+30", "-delete"])
            .output()
            .context("Failed to clean cache directory")?;
        
        if !output.status.success() {
            debug!("Cache cleanup failed for {}: {}", cache_dir, String::from_utf8_lossy(&output.stderr));
        }
        
        let final_size = self.get_directory_size(cache_dir).await.unwrap_or(initial_size);
        Ok((initial_size - final_size).max(0.0))
    }
    
    async fn clean_system_cache(&self) -> Result<()> {
        // Clean package manager caches
        let cache_commands = [
            // Clean apt cache (Debian/Ubuntu)
            ("apt-get", vec!["clean"]),
            // Clean dnf cache (Fedora/RHEL)
            ("dnf", vec!["clean", "all"]),
            // Clean pacman cache (Arch)
            ("paccache", vec!["-r"]),
            // Clean portage distfiles (Gentoo)
            ("eclean", vec!["distfiles"]),
        ];
        
        for (cmd, args) in &cache_commands {
            if let Ok(output) = Command::new(cmd).args(args).output() {
                if output.status.success() {
                    debug!("Successfully ran: {} {}", cmd, args.join(" "));
                }
            }
        }
        
        Ok(())
    }
    
    async fn get_directory_size(&self, path: &str) -> Result<f64> {
        let output = Command::new("du")
            .args(["-sm", path])
            .output()
            .context("Failed to get directory size")?;
        
        if !output.status.success() {
            return Ok(0.0);
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let size_str = output_str.split_whitespace().next().unwrap_or("0");
        let size_mb: f64 = size_str.parse().unwrap_or(0.0);
        
        Ok(size_mb)
    }
    
    async fn compress_files(&self) -> Result<ActionResult> {
        if !self.config.enable_compression {
            return Ok(ActionResult {
                action: Action::CompressFiles,
                success: true,
                space_freed_mb: 0.0,
                message: "Compression disabled in config".to_string(),
            });
        }
        
        info!("Compressing files");
        
        // For BTRFS, we can use filesystem-level compression
        let output = Command::new("btrfs")
            .args(["filesystem", "defragment", "-r", "-v", "-clzo", "/"])
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                Ok(ActionResult {
                    action: Action::CompressFiles,
                    success: true,
                    space_freed_mb: 0.0, // Compression doesn't free space immediately
                    message: "BTRFS compression/defragmentation completed".to_string(),
                })
            }
            Ok(output) => {
                warn!("BTRFS compression failed: {}", String::from_utf8_lossy(&output.stderr));
                Ok(ActionResult {
                    action: Action::CompressFiles,
                    success: false,
                    space_freed_mb: 0.0,
                    message: format!("BTRFS compression failed: {}", String::from_utf8_lossy(&output.stderr)),
                })
            }
            Err(e) => {
                warn!("Failed to run BTRFS compression: {}", e);
                Ok(ActionResult {
                    action: Action::CompressFiles,
                    success: false,
                    space_freed_mb: 0.0,
                    message: format!("Failed to run BTRFS compression: {}", e),
                })
            }
        }
    }
    
    async fn balance_metadata(&self) -> Result<ActionResult> {
        if !self.config.enable_balance {
            return Ok(ActionResult {
                action: Action::BalanceMetadata,
                success: true,
                space_freed_mb: 0.0,
                message: "Balance disabled in config".to_string(),
            });
        }
        
        info!("Balancing BTRFS metadata");
        
        let output = Command::new("btrfs")
            .args(["balance", "start", "-musage=50", "/"])
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                Ok(ActionResult {
                    action: Action::BalanceMetadata,
                    success: true,
                    space_freed_mb: 0.0, // Balance reorganizes but doesn't necessarily free space
                    message: "BTRFS metadata balance completed".to_string(),
                })
            }
            Ok(output) => {
                warn!("BTRFS balance failed: {}", String::from_utf8_lossy(&output.stderr));
                Ok(ActionResult {
                    action: Action::BalanceMetadata,
                    success: false,
                    space_freed_mb: 0.0,
                    message: format!("BTRFS balance failed: {}", String::from_utf8_lossy(&output.stderr)),
                })
            }
            Err(e) => {
                warn!("Failed to run BTRFS balance: {}", e);
                Ok(ActionResult {
                    action: Action::BalanceMetadata,
                    success: false,
                    space_freed_mb: 0.0,
                    message: format!("Failed to run BTRFS balance: {}", e),
                })
            }
        }
    }
    
    async fn cleanup_snapshots(&self) -> Result<ActionResult> {
        if !self.config.enable_snapshot_cleanup {
            return Ok(ActionResult {
                action: Action::CleanupSnapshots,
                success: true,
                space_freed_mb: 0.0,
                message: "Snapshot cleanup disabled in config".to_string(),
            });
        }
        
        info!("Cleaning up old snapshots");
        
        // List all snapshots
        let output = Command::new("btrfs")
            .args(["subvolume", "list", "-s", "/"])
            .output();
        
        let snapshots = match output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => {
                return Ok(ActionResult {
                    action: Action::CleanupSnapshots,
                    success: true,
                    space_freed_mb: 0.0,
                    message: "No snapshots found or BTRFS not available".to_string(),
                });
            }
        };
        
        // Parse snapshot list and identify old snapshots to delete
        let snapshot_lines: Vec<&str> = snapshots.lines().collect();
        let snapshots_to_keep = self.config.snapshot_keep_count;
        
        if snapshot_lines.len() <= snapshots_to_keep {
            return Ok(ActionResult {
                action: Action::CleanupSnapshots,
                success: true,
                space_freed_mb: 0.0,
                message: format!("No snapshots to clean (keeping {} snapshots)", snapshots_to_keep),
            });
        }
        
        // This is a simplified implementation - in practice, you'd want more sophisticated
        // snapshot selection logic based on age, type, etc.
        let total_freed = 0.0;
        let snapshots_to_delete = snapshot_lines.len() - snapshots_to_keep;
        
        // For now, just report what would be done
        Ok(ActionResult {
            action: Action::CleanupSnapshots,
            success: true,
            space_freed_mb: total_freed,
            message: format!("Would delete {} old snapshots", snapshots_to_delete),
        })
    }
}

#[derive(Debug)]
pub struct ActionResult {
    pub action: Action,
    pub success: bool,
    pub space_freed_mb: f64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ActionConfig;
    
    #[test]
    fn test_action_enum() {
        assert_eq!(Action::from_id(0), Some(Action::NoOperation));
        assert_eq!(Action::from_id(1), Some(Action::DeleteTempFiles));
        assert_eq!(Action::from_id(5), None);
        assert_eq!(Action::action_count(), 5);
    }
    
    #[tokio::test]
    async fn test_dry_run_mode() {
        let config = ActionConfig {
            enable_compression: true,
            enable_balance: true,
            enable_snapshot_cleanup: true,
            enable_temp_cleanup: true,
            temp_paths: vec!["/tmp".to_string()],
            snapshot_keep_count: 10,
        };
        
        let executor = ActionExecutor::new(config, true);
        let result = executor.execute_action(Action::DeleteTempFiles).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.space_freed_mb, 0.0);
        assert!(result.message.contains("Dry run"));
    }
    
    #[tokio::test]
    async fn test_no_operation() {
        let config = ActionConfig {
            enable_compression: true,
            enable_balance: true,
            enable_snapshot_cleanup: true,
            enable_temp_cleanup: true,
            temp_paths: vec![],
            snapshot_keep_count: 10,
        };
        
        let executor = ActionExecutor::new(config, false);
        let result = executor.execute_action(Action::NoOperation).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.space_freed_mb, 0.0);
    }
}
