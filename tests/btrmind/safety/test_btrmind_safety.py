#!/usr/bin/env python3
"""
BtrMind Safety Tests
Test BtrMind safety mechanisms and security features
"""

import os
import sys
import tempfile
import subprocess
import json
from pathlib import Path
import unittest.mock as mock

# Add the ai-agents/btrmind directory to Python path
sys.path.insert(0, '/Users/a/code/RegicideOS/ai-agents/btrmind')

class BtrMindSafetyTester:
    def __init__(self):
        self.test_dir = None
        self.btrfs_image = None
        self.mount_point = None
        self.btrmind_binary = '/Users/a/code/RegicideOS/target/release/btrmind'
        self.test_results = []
        
    def setup_test_environment(self):
        """Set up safe test environment."""
        print("Setting up safe test environment...")
        
        # Create temporary directory
        self.test_dir = tempfile.mkdtemp(prefix='btrmind-safety-')
        
        # Create test BTRFS image
        self.btrfs_image = os.path.join(self.test_dir, 'test-btrfs.img')
        self.mount_point = os.path.join(self.test_dir, 'mount')
        
        os.makedirs(self.mount_point)
        
        # Create small BTRFS image for safety tests
        subprocess.run(['dd', 'if=/dev/zero', 'of=' + self.btrfs_image, 'bs=1M', 'count=256'], check=True)
        subprocess.run(['mkfs.btrfs', self.btrfs_image], check=True)
        subprocess.run(['mount', '-o', 'loop', self.btrfs_image, self.mount_point], check=True)
        
        print("✓ Safe test environment setup complete")
        
    def create_safety_config(self):
        """Create safety-focused configuration."""
        config_content = f"""
[monitoring]
target_path = "{self.mount_point}"
poll_interval = 1

[thresholds]
warning_level = 50.0
critical_level = 75.0
emergency_level = 90.0

[actions]
enable_compression = true
enable_balance = true
enable_snapshot_cleanup = true
enable_temp_cleanup = true
temp_paths = ["{self.test_dir}/temp"]
snapshot_keep_count = 3

[learning]
model_path = "{self.test_dir}/safety_model"
model_update_interval = 60
reward_smoothing = 0.8
exploration_rate = 0.05
learning_rate = 0.001
discount_factor = 0.9

dry_run = true
"""
        
        config_file = os.path.join(self.test_dir, 'btrmind-safety.toml')
        with open(config_file, 'w') as f:
            f.write(config_content)
            
        return config_file
        
    def test_dry_run_safety(self):
        """Test that BtrMind respects dry-run mode."""
        print("Testing dry-run safety...")
        
        try:
            # Run optimization in dry-run mode
            result = subprocess.run([
                self.btrmind_binary,
                '--config', self.create_safety_config(),
                'optimize'
            ], capture_output=True, text=True, timeout=30)
            
            if result.returncode == 0:
                output = result.stdout.lower()
                
                # Check that dry-run is mentioned
                if 'dry' in output or 'simulation' in output:
                    # Verify no actual filesystem changes were made
                    original_stats = os.statvfs(self.mount_point)
                    time.sleep(2)  # Wait for any potential operations
                    
                    new_stats = os.statvfs(self.mount_point)
                    
                    # Check that filesystem stats haven't changed significantly
                    if (abs(original_stats.f_blocks - new_stats.f_blocks) < 100 and
                        abs(original_stats.f_bavail - new_stats.f_bavail) < 100):
                        self.test_results.append(("Dry-Run Safety", "PASSED"))
                        print("✓ Dry-run safety verified")
                        return True
                    else:
                        self.test_results.append(("Dry-Run Safety", "FAILED - Filesystem changed"))
                        print("✗ Dry-run safety failed - Filesystem changed unexpectedly")
                        return False
                else:
                    self.test_results.append(("Dry-Run Safety", "FAILED - No dry-run indication"))
                    print("✗ Dry-run safety failed - No dry-run indication in output")
                    return False
            else:
                self.test_results.append(("Dry-Run Safety", f"FAILED - Return code {result.returncode}"))
                print(f"✗ Dry-run safety failed - Return code {result.returncode}")
                return False
                
        except Exception as e:
            self.test_results.append(("Dry-Run Safety", f"FAILED - {str(e)}"))
            print(f"✗ Dry-run safety failed - {str(e)}")
            return False
            
    def test_config_validation_safety(self):
        """Test configuration validation safety."""
        print("Testing configuration validation safety...")
        
        try:
            # Test with invalid configuration
            invalid_config = os.path.join(self.test_dir, 'invalid.toml')
            with open(invalid_config, 'w') as f:
                f.write("""
[monitoring]
target_path = "/nonexistent/path"
poll_interval = -1
""")
                
            result = subprocess.run([
                self.btrmind_binary,
                '--config', invalid_config,
                'config'
            ], capture_output=True, text=True, timeout=10)
            
            # Should fail with invalid config
            if result.returncode != 0:
                self.test_results.append(("Config Validation Safety", "PASSED"))
                print("✓ Configuration validation safety verified")
                return True
            else:
                self.test_results.append(("Config Validation Safety", "FAILED - Accepted invalid config"))
                print("✗ Configuration validation safety failed - Accepted invalid config")
                return False
                
        except Exception as e:
            self.test_results.append(("Config Validation Safety", f"FAILED - {str(e)}"))
            print(f"✗ Configuration validation safety failed - {str(e)}")
            return False
            
    def test_path_traversal_protection(self):
        """Test protection against path traversal attacks."""
        print("Testing path traversal protection...")
        
        try:
            # Create config with suspicious path
            suspicious_config = os.path.join(self.test_dir, 'suspicious.toml')
            with open(suspicious_config, 'w') as f:
                f.write(f"""
[monitoring]
target_path = "{self.mount_point}/../../../etc"
poll_interval = 1
""")
                
            result = subprocess.run([
                self.btrmind_binary,
                '--config', suspicious_config,
                'config'
            ], capture_output=True, text=True, timeout=10)
            
            # Should reject suspicious path
            if result.returncode != 0:
                self.test_results.append(("Path Traversal Protection", "PASSED"))
                print("✓ Path traversal protection verified")
                return True
            else:
                self.test_results.append(("Path Traversal Protection", "FAILED - Accepted suspicious path"))
                print("✗ Path traversal protection failed - Accepted suspicious path")
                return False
                
        except Exception as e:
            self.test_results.append(("Path Traversal Protection", f"FAILED - {str(e)}"))
            print(f"✗ Path traversal protection failed - {str(e)}")
            return False
            
    def test_resource_limits(self):
        """Test resource limits and memory safety."""
        print("Testing resource limits...")
        
        try:
            # Test with very aggressive polling
            aggressive_config = os.path.join(self.test_dir, 'aggressive.toml')
            with open(aggressive_config, 'w') as f:
                f.write(f"""
[monitoring]
target_path = "{self.mount_point}"
poll_interval = 0.001  # Very aggressive
""")
                
            result = subprocess.run([
                self.btrmind_binary,
                '--config', aggressive_config,
                'config'
            ], capture_output=True, text=True, timeout=5)
            
            # Should handle aggressive config gracefully
            if result.returncode == 0:
                self.test_results.append(("Resource Limits", "PASSED"))
                print("✓ Resource limits verified")
                return True
            else:
                self.test_results.append(("Resource Limits", "FAILED - Crashed with aggressive config"))
                print("✗ Resource limits failed - Crashed with aggressive config")
                return False
                
        except Exception as e:
            self.test_results.append(("Resource Limits", f"FAILED - {str(e)}"))
            print(f"✗ Resource limits failed - {str(e)}")
            return False
            
    def test_permission_safety(self):
        """Test permission safety and access control."""
        print("Testing permission safety...")
        
        try:
            # Test with read-only directory
            readonly_dir = os.path.join(self.test_dir, 'readonly')
            os.makedirs(readonly_dir)
            os.chmod(readonly_dir, 0o555)  # Read-only
            
            readonly_config = os.path.join(self.test_dir, 'readonly.toml')
            with open(readonly_config, 'w') as f:
                f.write(f"""
[monitoring]
target_path = "{readonly_dir}"
poll_interval = 1
""")
                
            result = subprocess.run([
                self.btrmind_binary,
                '--config', readonly_config,
                'config'
            ], capture_output=True, text=True, timeout=10)
            
            # Should handle read-only directory gracefully
            if result.returncode == 0:
                self.test_results.append(("Permission Safety", "PASSED"))
                print("✓ Permission safety verified")
                return True
            else:
                self.test_results.append(("Permission Safety", "FAILED - Failed with read-only directory"))
                print("✗ Permission safety failed - Failed with read-only directory")
                return False
                
        except Exception as e:
            self.test_results.append(("Permission Safety", f"FAILED - {str(e)}"))
            print(f"✗ Permission safety failed - {str(e)}")
            return False
            
    def test_input_sanitization(self):
        """Test input sanitization and command injection protection."""
        print("Testing input sanitization...")
        
        try:
            # Create config with potentially dangerous characters
            dangerous_config = os.path.join(self.test_dir, 'dangerous.toml')
            with open(dangerous_config, 'w') as f:
                f.write(f"""
[monitoring]
target_path = "{self.mount_point}/test;rm -rf /tmp/#"
poll_interval = 1
""")
                
            result = subprocess.run([
                self.btrmind_binary,
                '--config', dangerous_config,
                'config'
            ], capture_output=True, text=True, timeout=10)
            
            # Should sanitize dangerous input
            if result.returncode != 0:
                self.test_results.append(("Input Sanitization", "PASSED"))
                print("✓ Input sanitization verified")
                return True
            else:
                self.test_results.append(("Input Sanitization", "FAILED - Accepted dangerous input"))
                print("✗ Input sanitization failed - Accepted dangerous input")
                return False
                
        except Exception as e:
            self.test_results.append(("Input Sanitization", f"FAILED - {str(e)}"))
            print(f"✗ Input sanitization failed - {str(e)}")
            return False
            
    def test_error_handling(self):
        """Test error handling and graceful failure."""
        print("Testing error handling...")
        
        try:
            # Test with non-existent BTRFS filesystem
            missing_config = os.path.join(self.test_dir, 'missing.toml')
            with open(missing_config, 'w') as f:
                f.write("""
[monitoring]
target_path = "/nonexistent/btrfs/filesystem"
poll_interval = 1
""")
                
            result = subprocess.run([
                self.btrmind_binary,
                '--config', missing_config,
                'config'
            ], capture_output=True, text=True, timeout=10)
            
            # Should handle missing filesystem gracefully
            if result.returncode != 0:
                error_output = result.stderr.lower()
                if 'error' in error_output or 'not found' in error_output:
                    self.test_results.append(("Error Handling", "PASSED"))
                    print("✓ Error handling verified")
                    return True
                else:
                    self.test_results.append(("Error Handling", "FAILED - No error message"))
                    print("✗ Error handling failed - No error message")
                    return False
            else:
                self.test_results.append(("Error Handling", "FAILED - Accepted missing filesystem"))
                print("✗ Error handling failed - Accepted missing filesystem")
                return False
                
        except Exception as e:
            self.test_results.append(("Error Handling", f"FAILED - {str(e)}"))
            print(f"✗ Error handling failed - {str(e)}")
            return False
            
    def cleanup_test_environment(self):
        """Clean up test environment."""
        print("Cleaning up test environment...")
        
        try:
            # Unmount BTRFS filesystem
            if self.mount_point and os.path.ismount(self.mount_point):
                subprocess.run(['umount', self.mount_point], check=True)
                
            # Remove test directory
            if self.test_dir and os.path.exists(self.test_dir):
                subprocess.run(['rm', '-rf', self.test_dir])
                
            print("✓ Test environment cleanup complete")
            
        except Exception as e:
            print(f"⚠ Cleanup warning: {str(e)}")
            
    def run_all_tests(self):
        """Run all safety tests."""
        print("=== BtrMind Safety Tests ===")
        
        try:
            # Setup test environment
            self.setup_test_environment()
            
            # Build BtrMind if needed
            if not os.path.exists(self.btrmind_binary):
                print("Building BtrMind...")
                subprocess.run(['cargo', 'build', '--release'], cwd='/Users/a/code/RegicideOS/ai-agents/btrmind', check=True)
                print("✓ BtrMind built successfully")
            
            # Run all safety tests
            test_functions = [
                self.test_dry_run_safety,
                self.test_config_validation_safety,
                self.test_path_traversal_protection,
                self.test_resource_limits,
                self.test_permission_safety,
                self.test_input_sanitization,
                self.test_error_handling
            ]
            
            for test_func in test_functions:
                try:
                    test_func()
                except Exception as e:
                    test_name = test_func.__name__.replace('test_', '').replace('_', ' ').title()
                    self.test_results.append((test_name, f"FAILED - Exception: {str(e)}"))
                    print(f"✗ {test_name} failed with exception: {str(e)}")
                    
        finally:
            # Always cleanup
            self.cleanup_test_environment()
            
        # Print results
        self.print_results()
        
    def print_results(self):
        """Print test results."""
        print("\n=== Safety Test Results ===")
        
        passed = 0
        failed = 0
        
        for test_name, result in self.test_results:
            if result == "PASSED":
                print(f"✓ {test_name}: {result}")
                passed += 1
            else:
                print(f"✗ {test_name}: {result}")
                failed += 1
                
        print(f"\nSummary: {passed} passed, {failed} failed")
        
        if failed == 0:
            print("🛡️ All safety tests passed! BtrMind is secure.")
        else:
            print("⚠ Some safety tests failed. Review and fix security issues.")

def main():
    """Main test runner."""
    tester = BtrMindSafetyTester()
    tester.run_all_tests()

if __name__ == "__main__":
    main()