pub mod logging;
pub mod error;
pub mod serde_utils;

pub use logging::setup_logging;
pub use error::handle_error;
pub use serde_utils::{to_json_string, from_json_string};

use crate::error::{PortCLError, Result};
use std::path::Path;
use tokio::fs;

pub async fn ensure_directory_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).await
            .map_err(|e| PortCLError::Io(e))?;
    }
    Ok(())
}

pub async fn read_file_content(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .await
        .map_err(|e| PortCLError::Io(e))
}

pub async fn write_file_content(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_directory_exists(parent).await?;
    }

    fs::write(path, content)
        .await
        .map_err(|e| PortCLError::Io(e))
}

pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m {}s", seconds / 3600, (seconds % 3600) / 60, seconds % 60)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

pub fn parse_package_name(package: &str) -> Result<(String, String)> {
    if package.contains('/') {
        let parts: Vec<&str> = package.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    Err(PortCLError::Validation(format!(
        "Invalid package name format: {}. Expected category/name", package
    )))
}

pub fn validate_package_name(package: &str) -> bool {
    // Basic validation for Gentoo package names
    if !package.contains('/') {
        return false;
    }

    let parts: Vec<&str> = package.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let category = parts[0];
    let name = parts[1];

    // Category should be lowercase alphanumeric with hyphens
    if !category.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return false;
    }

    // Package name should be lowercase alphanumeric with hyphens and underscores
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3661), "1h 1m 1s");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_parse_package_name() {
        assert!(parse_package_name("sys-apps/portage").is_ok());
        assert!(parse_package_name("invalid-package-name").is_err());
        assert!(parse_package_name("sys-apps/portage-3.0.30").is_ok());
    }

    #[test]
    fn test_validate_package_name() {
        assert!(validate_package_name("sys-apps/portage"));
        assert!(validate_package_name("dev-lang/rust"));
        assert!(!validate_package_name("invalid_package"));
        assert!(!validate_package_name("Sys-Apps/Portage"));
        assert!(!validate_package_name("sys-apps/"));
    }
}