#!/usr/bin/env python3
"""
BtrMind Integration Tests
Test BtrMind functionality with real BTRFS filesystems
"""

import os
import sys
import tempfile
import subprocess
import json
import time
from pathlib import Path

# Add the ai-agents/btrmind directory to Python path
sys.path.insert(0, '/Users/a/code/RegicideOS/ai-agents/btrmind')

class BtrMindIntegrationTester:
    def __init__(self):
        self.test_dir = None
        self.btrfs_image = None
        self.mount_point = None
        self.btrmind_binary = '/Users/a/code/RegicideOS/target/release/btrmind'
        self.test_results = []
        
    def setup_test_environment(self):
        """Set up test environment with temporary BTRFS filesystem."""
        print("Setting up test environment...")
        
        # Create temporary directory
        self.test_dir = tempfile.mkdtemp(prefix='btrmind-test-')
        self.btrfs_image = os.path.join(self.test_dir, 'test-btrfs.img')
        self.mount_point = os.path.join(self.test_dir, 'mount')
        
        # Create mount point
        os.makedirs(self.mount_point)
        
        # Create BTRFS image file
        print(f"Creating BTRFS image: {self.btrfs_image}")
        subprocess.run(['dd', 'if=/dev/zero', 'of=' + self.btrfs_image, 'bs=1M', 'count=512'], check=True)
        subprocess.run(['mkfs.btrfs', self.btrfs_image], check=True)
        
        # Mount BTRFS filesystem
        print(f"Mounting BTRFS filesystem at: {self.mount_point}")
        subprocess.run(['mount', '-o', 'loop', self.btrfs_image, self.mount_point], check=True)
        
        # Create test subvolumes and data
        self.create_test_data()
        
        print("✓ Test environment setup complete")
        
    def create_test_data(self):
        """Create test subvolumes and data."""
        # Create subvolumes
        subvolumes = ['@', '@home', '@var', '@tmp']
        for subvol in subvolumes:
            subvol_path = os.path.join(self.mount_point, subvol)
            subprocess.run(['btrfs', 'subvolume', 'create', subvol_path], check=True)
            
        # Create test files to consume space
        test_file = os.path.join(self.mount_point, '@', 'test-data.bin')
        with open(test_file, 'wb') as f:
            f.write(os.urandom(50 * 1024 * 1024))  # 50MB test file
            
    def create_test_config(self):
        """Create test configuration for BtrMind."""
        config_content = f"""
[monitoring]
target_path = "{self.mount_point}"
poll_interval = 2

[thresholds]
warning_level = 70.0
critical_level = 85.0
emergency_level = 95.0

[actions]
enable_compression = true
enable_balance = true
enable_snapshot_cleanup = true
enable_temp_cleanup = true
temp_paths = ["{self.mount_point}/@tmp"]
snapshot_keep_count = 5

[learning]
model_path = "{self.test_dir}/test_model"
model_update_interval = 300
reward_smoothing = 0.9
exploration_rate = 0.2
learning_rate = 0.01
discount_factor = 0.95

dry_run = true
"""
        
        config_file = os.path.join(self.test_dir, 'btrmind-test.toml')
        with open(config_file, 'w') as f:
            f.write(config_content)
            
        return config_file
        
    def test_metrics_collection(self):
        """Test BtrMind metrics collection."""
        print("Testing metrics collection...")
        
        try:
            # Run metrics collection
            result = subprocess.run([
                self.btrmind_binary,
                '--config', self.create_test_config(),
                'metrics'
            ], capture_output=True, text=True, timeout=30)
            
            if result.returncode == 0:
                metrics_output = result.stdout
                # Check for expected metrics
                if 'disk_usage' in metrics_output and 'timestamp' in metrics_output:
                    self.test_results.append(("Metrics Collection", "PASSED"))
                    print("✓ Metrics collection successful")
                    return True
                else:
                    self.test_results.append(("Metrics Collection", "FAILED - Invalid output format"))
                    print("✗ Metrics collection failed - Invalid output format")
                    return False
            else:
                self.test_results.append(("Metrics Collection", f"FAILED - Return code {result.returncode}"))
                print(f"✗ Metrics collection failed - Return code {result.returncode}")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            self.test_results.append(("Metrics Collection", "FAILED - Timeout"))
            print("✗ Metrics collection failed - Timeout")
            return False
        except Exception as e:
            self.test_results.append(("Metrics Collection", f"FAILED - {str(e)}"))
            print(f"✗ Metrics collection failed - {str(e)}")
            return False
            
    def test_analysis_functionality(self):
        """Test BtrMind analysis functionality."""
        print("Testing analysis functionality...")
        
        try:
            # Run analysis
            result = subprocess.run([
                self.btrmind_binary,
                '--config', self.create_test_config(),
                'analyze'
            ], capture_output=True, text=True, timeout=30)
            
            if result.returncode == 0:
                analysis_output = result.stdout
                # Check for analysis results
                if 'recommendations' in analysis_output.lower() or 'analysis' in analysis_output.lower():
                    self.test_results.append(("Analysis Functionality", "PASSED"))
                    print("✓ Analysis functionality successful")
                    return True
                else:
                    self.test_results.append(("Analysis Functionality", "FAILED - Invalid output format"))
                    print("✗ Analysis functionality failed - Invalid output format")
                    return False
            else:
                self.test_results.append(("Analysis Functionality", f"FAILED - Return code {result.returncode}"))
                print(f"✗ Analysis functionality failed - Return code {result.returncode}")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            self.test_results.append(("Analysis Functionality", "FAILED - Timeout"))
            print("✗ Analysis functionality failed - Timeout")
            return False
        except Exception as e:
            self.test_results.append(("Analysis Functionality", f"FAILED - {str(e)}"))
            print(f"✗ Analysis functionality failed - {str(e)}")
            return False
            
    def test_optimization_actions(self):
        """Test BtrMind optimization actions in dry-run mode."""
        print("Testing optimization actions...")
        
        try:
            # Run optimization
            result = subprocess.run([
                self.btrmind_binary,
                '--config', self.create_test_config(),
                'optimize'
            ], capture_output=True, text=True, timeout=60)
            
            if result.returncode == 0:
                optimize_output = result.stdout
                # Check for dry-run mode execution
                if 'dry' in optimize_output.lower() or 'simulation' in optimize_output.lower():
                    self.test_results.append(("Optimization Actions", "PASSED"))
                    print("✓ Optimization actions successful")
                    return True
                else:
                    self.test_results.append(("Optimization Actions", "FAILED - Not in dry-run mode"))
                    print("✗ Optimization actions failed - Not in dry-run mode")
                    return False
            else:
                self.test_results.append(("Optimization Actions", f"FAILED - Return code {result.returncode}"))
                print(f"✗ Optimization actions failed - Return code {result.returncode}")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            self.test_results.append(("Optimization Actions", "FAILED - Timeout"))
            print("✗ Optimization actions failed - Timeout")
            return False
        except Exception as e:
            self.test_results.append(("Optimization Actions", f"FAILED - {str(e)}"))
            print(f"✗ Optimization actions failed - {str(e)}")
            return False
            
    def test_configuration_validation(self):
        """Test BtrMind configuration validation."""
        print("Testing configuration validation...")
        
        try:
            # Test with valid configuration
            result = subprocess.run([
                self.btrmind_binary,
                '--config', self.create_test_config(),
                'config'
            ], capture_output=True, text=True, timeout=10)
            
            if result.returncode == 0:
                self.test_results.append(("Configuration Validation", "PASSED"))
                print("✓ Configuration validation successful")
                return True
            else:
                self.test_results.append(("Configuration Validation", f"FAILED - Return code {result.returncode}"))
                print(f"✗ Configuration validation failed - Return code {result.returncode}")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            self.test_results.append(("Configuration Validation", "FAILED - Timeout"))
            print("✗ Configuration validation failed - Timeout")
            return False
        except Exception as e:
            self.test_results.append(("Configuration Validation", f"FAILED - {str(e)}"))
            print(f"✗ Configuration validation failed - {str(e)}")
            return False
            
    def test_btrfs_operations(self):
        """Test BTRFS-specific operations."""
        print("Testing BTRFS operations...")
        
        try:
            # Test subvolume listing
            result = subprocess.run(['btrfs', 'subvolume', 'list', self.mount_point], 
                                  capture_output=True, text=True, timeout=10)
            
            if result.returncode == 0:
                # Check if our test subvolumes are listed
                if '@' in result.stdout and '@home' in result.stdout:
                    self.test_results.append(("BTRFS Operations", "PASSED"))
                    print("✓ BTRFS operations successful")
                    return True
                else:
                    self.test_results.append(("BTRFS Operations", "FAILED - Subvolumes not found"))
                    print("✗ BTRFS operations failed - Subvolumes not found")
                    return False
            else:
                self.test_results.append(("BTRFS Operations", f"FAILED - Return code {result.returncode}"))
                print(f"✗ BTRFS operations failed - Return code {result.returncode}")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            self.test_results.append(("BTRFS Operations", "FAILED - Timeout"))
            print("✗ BTRFS operations failed - Timeout")
            return False
        except Exception as e:
            self.test_results.append(("BTRFS Operations", f"FAILED - {str(e)}"))
            print(f"✗ BTRFS operations failed - {str(e)}")
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
        """Run all integration tests."""
        print("=== BtrMind Integration Tests ===")
        
        try:
            # Setup test environment
            self.setup_test_environment()
            
            # Build BtrMind if needed
            if not os.path.exists(self.btrmind_binary):
                print("Building BtrMind...")
                subprocess.run(['cargo', 'build', '--release'], cwd='/Users/a/code/RegicideOS/ai-agents/btrmind', check=True)
                print("✓ BtrMind built successfully")
            
            # Run all tests
            test_functions = [
                self.test_configuration_validation,
                self.test_btrfs_operations,
                self.test_metrics_collection,
                self.test_analysis_functionality,
                self.test_optimization_actions
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
        print("\n=== Integration Test Results ===")
        
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
            print("🎉 All integration tests passed!")
        else:
            print("⚠ Some integration tests failed. Check the output above for details.")

def main():
    """Main test runner."""
    tester = BtrMindIntegrationTester()
    tester.run_all_tests()

if __name__ == "__main__":
    main()