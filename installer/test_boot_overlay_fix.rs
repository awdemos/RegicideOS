#!/usr/bin/env rust-script

use std::fs;
use std::path::PathBuf;

// Copy the exact functions from main.rs to test them
use anyhow::{bail, Result, Context};

fn sanitize_input(input: &str) -> String {
    input.chars()
        .filter(|c| *c != '\0' && !c.is_control() || *c == '\t' || *c == '\n' || *c == '\r')
        .collect()
}

fn validate_safe_path(path: &str, allowed_base: &str) -> Result<PathBuf> {
    use std::path::{Path, PathBuf};
    
    // Remove any dangerous characters
    let sanitized = sanitize_input(path);
    
    // Convert to absolute path
    let absolute_path = if sanitized.starts_with('/') {
        PathBuf::from(&sanitized)
    } else {
        std::env::current_dir()?.join(&sanitized)
    };
    
    // Get canonical path for allowed base (must exist)
    let base_path = Path::new(allowed_base).canonicalize()
        .with_context(|| format!("Base directory does not exist: {}", allowed_base))?;
    
    // For validation, check if path would be within base after creation
    // We need to handle the case where path doesn't exist yet (for directory creation)
    let path_to_check = if absolute_path.exists() {
        absolute_path.canonicalize()
            .unwrap_or_else(|_| absolute_path.clone())
    } else {
        // For non-existent paths, validate parent directory exists and is within bounds
        let parent = absolute_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no parent directory"))?;
        
        if !parent.exists() {
            bail!("Parent directory does not exist: {}", parent.display());
        }
        
        parent.canonicalize()
            .map(|p| p.join(absolute_path.file_name().unwrap_or_default()))
            .unwrap_or(absolute_path.clone())
    };
    
    // Ensure the path is within the allowed base directory
    if !path_to_check.starts_with(&base_path) {
        bail!("Path access denied: {} is outside allowed base {}", path_to_check.display(), base_path.display());
    }
    
    // Additional checks for dangerous patterns
    let path_str = absolute_path.to_string_lossy();
    let dangerous_patterns = [
        "..", "~", "$HOME", "/etc/", "/root/", "/var/", "/usr/",
        "/bin/", "/sbin/", "/lib/", "/proc/", "/sys/", "/dev/",
    ];
    
    for pattern in &dangerous_patterns {
        if path_str.contains(pattern) && !path_str.starts_with(allowed_base) {
            bail!("Path access denied: dangerous pattern detected");
        }
    }
    
    Ok(absolute_path)
}

fn safe_create_dir_all(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::create_dir_all(validated_path)
        .with_context(|| "Failed to create directory")?;
    Ok(())
}

fn main() -> Result<()> {
    println!("Testing boot_overlay creation fix...");
    
    // Create test environment similar to installer
    fs::create_dir_all("/tmp/test_mnt")?;
    
    // Test 1: Create boot_overlay directory (this was failing before)
    println!("Test 1: Creating /tmp/test_mnt/boot_overlay...");
    match safe_create_dir_all("/tmp/test_mnt/boot_overlay", "/tmp/test_mnt") {
        Ok(()) => println!("✅ SUCCESS: boot_overlay created successfully"),
        Err(e) => {
            println!("❌ FAILED: {}", e);
            return Err(e);
        }
    }
    
    // Verify directory was actually created
    if PathBuf::from("/tmp/test_mnt/boot_overlay").exists() {
        println!("✅ VERIFIED: boot_overlay directory exists");
    } else {
        println!("❌ FAILED: boot_overlay directory was not created");
    }
    
    // Test 2: Try creating nested directory structure
    println!("Test 2: Creating nested structure...");
    match safe_create_dir_all("/tmp/test_mnt/nested/deep/path", "/tmp/test_mnt") {
        Ok(()) => println!("✅ SUCCESS: nested structure created"),
        Err(e) => {
            println!("❌ FAILED: {}", e);
            return Err(e);
        }
    }
    
    // Test 3: Verify security still works (reject paths outside base)
    println!("Test 3: Testing security (should fail)...");
    match safe_create_dir_all("/tmp/evil_path", "/tmp/test_mnt") {
        Ok(()) => {
            println!("❌ FAILED: Security check should have rejected this");
            return Err(anyhow::anyhow!("Security check failed"));
        }
        Err(_) => println!("✅ SUCCESS: Security check correctly rejected path outside base"),
    }
    
    // Cleanup
    fs::remove_dir_all("/tmp/test_mnt")?;
    
    println!("\n🎉 All tests passed! The boot_overlay fix should work.");
    Ok(())
}