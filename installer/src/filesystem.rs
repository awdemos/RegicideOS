use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

fn sanitize_input(input: &str) -> String {
    input.chars().filter(|c| !c.is_control()).collect()
}

fn has_traversal_components(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::CurDir
        )
    })
}

// Validate that a path is rooted under allowed_base and contains no traversal
// components. Non-existent paths are accepted only when their parent directory
// already exists and is within the base.
pub fn validate_safe_path(path: &str, allowed_base: &str) -> Result<PathBuf> {
    if path.contains('\0') {
        bail!("Path access denied: null byte in path");
    }

    let sanitized = sanitize_input(path);

    let absolute_path = if sanitized.starts_with('/') {
        PathBuf::from(&sanitized)
    } else {
        std::env::current_dir()?.join(&sanitized)
    };

    // Reject traversal components before any canonicalization so a non-existent
    // path such as base/foo/../etc cannot escape the allowed base.
    if has_traversal_components(&absolute_path) {
        bail!("Path access denied: traversal component in path");
    }

    let base_path = Path::new(allowed_base)
        .canonicalize()
        .with_context(|| format!("Base directory does not exist: {allowed_base}"))?;

    let path_to_check = if absolute_path.exists() {
        absolute_path
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", absolute_path.display()))?
    } else {
        let parent = absolute_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no parent directory"))?;

        if !parent.exists() {
            bail!("Parent directory does not exist: {}", parent.display());
        }

        let canonical_parent = parent.canonicalize().with_context(|| {
            format!(
                "Failed to canonicalize parent directory: {}",
                parent.display()
            )
        })?;
        if !canonical_parent.starts_with(&base_path) {
            bail!(
                "Path access denied: parent {} is outside allowed base {}",
                canonical_parent.display(),
                base_path.display()
            );
        }

        let file_name = absolute_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no file name"))?;
        canonical_parent.join(file_name)
    };

    // Ensure the path is within the allowed base directory
    if !path_to_check.starts_with(&base_path) {
        bail!(
            "Path access denied: {} is outside allowed base {}",
            path_to_check.display(),
            base_path.display()
        );
    }

    Ok(path_to_check)
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
