use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

fn sanitize_input(input: &str) -> String {
    // Remove null bytes and control characters
    input
        .chars()
        .filter(|c| *c != '\0' && !c.is_control() || *c == '\t' || *c == '\n' || *c == '\r')
        .collect()
}

// Path traversal protection
pub fn validate_safe_path(path: &str, allowed_base: &str) -> Result<PathBuf> {
    // Remove any dangerous characters
    let sanitized = sanitize_input(path);

    // Convert to absolute path
    let absolute_path = if sanitized.starts_with('/') {
        PathBuf::from(&sanitized)
    } else {
        std::env::current_dir()?.join(&sanitized)
    };

    // Get canonical path for allowed base (must exist)
    let base_path = Path::new(allowed_base)
        .canonicalize()
        .with_context(|| format!("Base directory does not exist: {allowed_base}"))?;

    // For validation, check if the path would be within base after creation
    // We need to handle the case where the path doesn't exist yet (for directory creation)
    let path_to_check = if absolute_path.exists() {
        absolute_path
            .canonicalize()
            .unwrap_or_else(|_| absolute_path.clone())
    } else {
        // For non-existent paths, validate the parent directory exists and is within bounds
        let parent = absolute_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no parent directory"))?;

        if !parent.exists() {
            bail!("Parent directory does not exist: {}", parent.display());
        }

        parent
            .canonicalize()
            .map(|p| p.join(absolute_path.file_name().unwrap_or_default()))
            .unwrap_or(absolute_path.clone())
    };

    // Ensure the path is within the allowed base directory
    if !path_to_check.starts_with(&base_path) {
        bail!(
            "Path access denied: {} is outside allowed base {}",
            path_to_check.display(),
            base_path.display()
        );
    }

    // Additional checks for dangerous patterns
    let path_str = absolute_path.to_string_lossy();
    let dangerous_patterns = [
        "..", "~", "$HOME", "/etc/", "/root/", "/var/", "/usr/", "/bin/", "/sbin/", "/lib/",
        "/proc/", "/sys/", "/dev/",
    ];

    for pattern in &dangerous_patterns {
        if path_str.contains(pattern) && !path_str.starts_with(allowed_base) {
            bail!("Path access denied: dangerous pattern detected");
        }
    }

    Ok(absolute_path)
}

// Safe file operations with path validation
pub fn safe_create_dir_all(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::create_dir_all(validated_path).with_context(|| "Failed to create directory")?;
    Ok(())
}

pub fn safe_write_file(path: &str, content: &[u8], allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::write(validated_path, content).with_context(|| "Failed to write file")?;
    Ok(())
}

pub fn safe_read_file(path: &str, allowed_base: &str) -> Result<String> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::read_to_string(&validated_path).with_context(|| "Failed to read file")
}

pub fn safe_remove_file(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::remove_file(validated_path).with_context(|| "Failed to remove file")?;
    Ok(())
}

pub fn safe_remove_dir_all(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::remove_dir_all(validated_path).with_context(|| "Failed to remove directory")?;
    Ok(())
}
