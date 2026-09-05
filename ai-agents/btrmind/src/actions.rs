use crate::config::ActionConfig;
use anyhow::{bail, Context, Result};
use std::process::Command;
use tracing::{debug, info, warn};

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

    /// Allowed literal directories for temp cleanup. Any configured path outside
    /// this set is ignored to prevent accidental deletion of system data.
    const ALLOWED_TEMP_DIRS: &[&str] = &["/tmp", "/var/tmp", "/var/cache"];
    /// Allowed glob patterns that expand to safe per-user cache directories.
    const ALLOWED_GLOB_PATTERNS: &[&str] = &["/home/*/.cache"];

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
                    messages.push(format!("Cleaned {temp_path} ({freed:.1}MB freed)"));
                }
                Err(e) => {
                    warn!("Failed to clean {}: {}", temp_path, e);
                    messages.push(format!("Failed to clean {temp_path}: {e}"));
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
        // Reject paths that contain shell metacharacters or traversal
        // sequences before any filesystem access.
        if !Self::is_safe_temp_path(path) {
            bail!("Refusing to clean unsafe temp path: {}", path);
        }

        // Handle the single supported glob pattern.
        if path.contains('*') {
            if Self::ALLOWED_GLOB_PATTERNS.contains(&path) {
                return self.cleanup_glob_pattern(path).await;
            }
            bail!("Unsupported glob pattern: {}", path);
        }

        // Only clean explicitly allowlisted directories.
        if !Self::ALLOWED_TEMP_DIRS.contains(&path) {
            debug!("Skipping cleanup for non-allowed path: {}", path);
            return Ok(0.0);
        }

        let initial_size = self.get_directory_size(path).await.unwrap_or(0.0);

        match path {
            "/tmp" | "/var/tmp" => {
                Self::find_delete_older_than(path, 7).context("Failed to clean temporary files")?;
            }
            "/var/cache" => {
                self.clean_system_cache().await?;
            }
            _ => {}
        }

        let final_size = self.get_directory_size(path).await.unwrap_or(initial_size);
        let freed = (initial_size - final_size).max(0.0);

        debug!("Freed {:.1}MB from {}", freed, path);
        Ok(freed)
    }

    /// Validate that a configured temp path is safe to use.
    fn is_safe_temp_path(path: &str) -> bool {
        if path.contains('\0') || path.contains("..") {
            return false;
        }
        // Allow only a restricted character set to avoid shell injection.
        path.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.' | '*'))
    }

    /// Delete files under `path` older than `days` using a fixed argv array.
    fn find_delete_older_than(path: &str, days: u32) -> Result<()> {
        let output = Command::new("find")
            .args([path, "-type", "f", "-atime", "+"])
            .arg(days.to_string())
            .arg("-delete")
            .output()
            .context("Failed to run find for temp cleanup")?;

        if !output.status.success() {
            warn!(
                "find command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    async fn cleanup_glob_pattern(&self, pattern: &str) -> Result<f64> {
        if pattern != "/home/*/.cache" {
            bail!("Unsupported glob pattern: {}", pattern);
        }

        let output = Command::new("find")
            .args(["/home", "-maxdepth", "2", "-type", "d", "-name", ".cache"])
            .output()
            .context("Failed to find cache directories")?;

        if !output.status.success() {
            bail!("find failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let cache_dirs = String::from_utf8_lossy(&output.stdout);
        let mut total_freed = 0.0;

        for cache_dir in cache_dirs.lines() {
            if !Self::is_safe_temp_path(cache_dir) || cache_dir.contains("..") {
                warn!("Skipping unsafe cache directory: {}", cache_dir);
                continue;
            }
            if let Ok(freed) = self.cleanup_cache_directory(cache_dir).await {
                total_freed += freed;
            }
        }

        Ok(total_freed)
    }

    async fn cleanup_cache_directory(&self, cache_dir: &str) -> Result<f64> {
        if !Self::is_safe_temp_path(cache_dir) || cache_dir.contains("..") {
            bail!("Unsafe cache directory: {}", cache_dir);
        }

        let initial_size = self.get_directory_size(cache_dir).await.unwrap_or(0.0);

        Self::find_delete_older_than(cache_dir, 30).context("Failed to clean cache directory")?;

        let final_size = self
            .get_directory_size(cache_dir)
            .await
            .unwrap_or(initial_size);
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
                warn!(
                    "BTRFS compression failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(ActionResult {
                    action: Action::CompressFiles,
                    success: false,
                    space_freed_mb: 0.0,
                    message: format!(
                        "BTRFS compression failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                })
            }
            Err(e) => {
                warn!("Failed to run BTRFS compression: {}", e);
                Ok(ActionResult {
                    action: Action::CompressFiles,
                    success: false,
                    space_freed_mb: 0.0,
                    message: format!("Failed to run BTRFS compression: {e}"),
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
                warn!(
                    "BTRFS balance failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(ActionResult {
                    action: Action::BalanceMetadata,
                    success: false,
                    space_freed_mb: 0.0,
                    message: format!(
                        "BTRFS balance failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                })
            }
            Err(e) => {
                warn!("Failed to run BTRFS balance: {}", e);
                Ok(ActionResult {
                    action: Action::BalanceMetadata,
                    success: false,
                    space_freed_mb: 0.0,
                    message: format!("Failed to run BTRFS balance: {e}"),
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

        let output = Command::new("btrfs")
            .args(["subvolume", "list", "-s", "/"])
            .output();

        let snapshot_paths = match output {
            Ok(output) if output.status.success() => {
                Self::parse_snapshot_paths(&String::from_utf8_lossy(&output.stdout))
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

        let snapshots_to_keep = self.config.snapshot_keep_count;
        if snapshot_paths.len() <= snapshots_to_keep {
            return Ok(ActionResult {
                action: Action::CleanupSnapshots,
                success: true,
                space_freed_mb: 0.0,
                message: format!("No snapshots to clean (keeping {snapshots_to_keep} snapshots)"),
            });
        }

        // Delete oldest snapshots first. The btrfs output is sorted by generation,
        // so the first entries are the oldest.
        let mut total_freed = 0.0;
        let mut deleted = 0usize;
        for path in snapshot_paths
            .iter()
            .take(snapshot_paths.len() - snapshots_to_keep)
        {
            if Self::is_safe_temp_path(path) && !path.contains("..") {
                match Self::delete_snapshot(path) {
                    Ok(freed) => {
                        total_freed += freed;
                        deleted += 1;
                    }
                    Err(e) => warn!("Failed to delete snapshot {}: {}", path, e),
                }
            } else {
                warn!("Skipping unsafe snapshot path: {}", path);
            }
        }

        Ok(ActionResult {
            action: Action::CleanupSnapshots,
            success: true,
            space_freed_mb: total_freed,
            message: format!("Deleted {deleted} old snapshots ({total_freed:.1}MB freed)"),
        })
    }

    /// Parse the output of `btrfs subvolume list -s /` into absolute paths.
    /// Output lines look like: ID 257 gen 10 top level 5 path @home/.snapshots/1/snapshot
    fn parse_snapshot_paths(output: &str) -> Vec<String> {
        output
            .lines()
            .filter_map(|line| line.rsplit_once(" path ").map(|(_, p)| format!("/{}", p)))
            .collect()
    }

    /// Delete a single BTRFS snapshot and return the estimated freed space in MB.
    fn delete_snapshot(path: &str) -> Result<f64> {
        let initial_size = std::fs::metadata(path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);

        let result = Command::new("btrfs")
            .args(["subvolume", "delete", path])
            .output()
            .context(format!("Failed to delete snapshot {}", path))?;

        if !result.status.success() {
            bail!(
                "btrfs subvolume delete failed for {}: {}",
                path,
                String::from_utf8_lossy(&result.stderr)
            );
        }

        Ok(initial_size)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ActionResult {
    action: Action,
    success: bool,
    space_freed_mb: f64,
    message: String,
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
        let result = executor
            .execute_action(Action::DeleteTempFiles)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.space_freed_mb, 0.0);
        assert!(result.message.contains("Dry run"));
    }

    #[tokio::test]
    async fn test_snapshot_paths_parsing() {
        let sample = "ID 257 gen 10 top level 5 path @home/.snapshots/1/snapshot\n\
            ID 258 gen 11 top level 5 path @.snapshots/2/snapshot";
        let paths = ActionExecutor::parse_snapshot_paths(sample);
        assert_eq!(
            paths,
            vec!["/@home/.snapshots/1/snapshot", "/@.snapshots/2/snapshot",]
        );
    }

    #[tokio::test]
    async fn test_cleanup_path_rejects_unsafe_path() {
        let config = ActionConfig {
            enable_compression: false,
            enable_balance: false,
            enable_snapshot_cleanup: false,
            enable_temp_cleanup: true,
            temp_paths: vec![],
            snapshot_keep_count: 10,
        };
        let executor = ActionExecutor::new(config, false);
        assert!(executor.cleanup_path("/etc; rm -rf /").await.is_err());
        assert!(executor.cleanup_path("/tmp/../etc").await.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_path_ignores_non_allowed_dir() {
        let config = ActionConfig {
            enable_compression: false,
            enable_balance: false,
            enable_snapshot_cleanup: false,
            enable_temp_cleanup: true,
            temp_paths: vec![],
            snapshot_keep_count: 10,
        };
        let executor = ActionExecutor::new(config, false);
        assert_eq!(executor.cleanup_path("/home").await.unwrap(), 0.0);
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
