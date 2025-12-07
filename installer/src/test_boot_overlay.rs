#[cfg(test)]
mod boot_overlay_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_boot_overlay_creation_scenario() -> Result<()> {
        // Create test environment exactly like installer does
        fs::create_dir_all("/tmp/test_mnt")?;
        
        // This is the exact call that was failing in the installer
        println!("Testing exact installer scenario: safe_create_dir_all(\"/tmp/test_mnt/boot_overlay\", \"/tmp/test_mnt\")");
        
        let result = safe_create_dir_all("/tmp/test_mnt/boot_overlay", "/tmp/test_mnt");
        
        // Verify it succeeds
        assert!(result.is_ok(), "boot_overlay creation should succeed: {:?}", result);
        
        // Verify directory was actually created
        assert!(std::path::Path::new("/tmp/test_mnt/boot_overlay").exists(), 
                "boot_overlay directory should exist after creation");
        
        // Cleanup
        fs::remove_dir_all("/tmp/test_mnt")?;
        
        println!("✅ boot_overlay creation test passed!");
        Ok(())
    }
    
    #[test] 
    fn test_multiple_boot_overlay_calls() -> Result<()> {
        // Test multiple calls like installer does
        fs::create_dir_all("/tmp/test_mnt")?;
        
        // First call
        let result1 = safe_create_dir_all("/tmp/test_mnt/boot_overlay", "/tmp/test_mnt");
        assert!(result1.is_ok(), "First call should succeed");
        
        // Second call (should not fail even though directory exists)
        let result2 = safe_create_dir_all("/tmp/test_mnt/boot_overlay", "/tmp/test_mnt");
        assert!(result2.is_ok(), "Second call should succeed");
        
        // Cleanup
        fs::remove_dir_all("/tmp/test_mnt")?;
        
        println!("✅ Multiple boot_overlay calls test passed!");
        Ok(())
    }
}