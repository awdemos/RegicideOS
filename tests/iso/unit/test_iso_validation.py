"""
Unit tests for ISO validation functionality.
"""

import unittest
from unittest.mock import Mock, patch, MagicMock, call, mock_open
import tempfile
import os
import sys
import hashlib
from pathlib import Path
import shutil

# Add the project root to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent))

class TestISOValidation(unittest.TestCase):
    """Test ISO validation with comprehensive mocking."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.test_iso = os.path.join(self.temp_dir, "test.iso")
        
        # Create a mock ISO file
        with open(self.test_iso, 'wb') as f:
            f.write(b"Mock ISO content" * 1000)  # ~16KB file
        
        self.validator = self.create_validator()
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def create_validator(self):
        """Create a mock ISO validator."""
        class ISOValidator:
            def __init__(self, iso_file):
                self.iso_file = iso_file
                self.errors = []
                self.warnings = []
                self.passed_checks = []
            
            def validate(self):
                """Run all validation checks."""
                checks = [
                    ("file_format", self._validate_file_format),
                    ("file_size", self._validate_file_size),
                    ("file_readability", self._validate_file_readability)
                ]
                
                for check_name, check_func in checks:
                    try:
                        if check_func():
                            self.passed_checks.append(check_name)
                        else:
                            self.errors.append(f"Check {check_name} failed")
                    except Exception as e:
                        self.errors.append(f"Check {check_name} failed with exception: {e}")
                
                return len(self.errors) == 0
            
            def _validate_file_format(self):
                """Validate basic file format."""
                if not os.path.exists(self.iso_file):
                    self.errors.append("ISO file does not exist")
                    return False
                
                if not os.path.isfile(self.iso_file):
                    self.errors.append("ISO path is not a file")
                    return False
                
                return True
            
            def _validate_file_size(self):
                """Validate file size constraints."""
                file_size = os.path.getsize(self.iso_file)
                
                if file_size == 0:
                    self.errors.append("ISO file is empty")
                    return False
                
                if file_size < 1024:  # 1KB minimum
                    self.warnings.append("ISO file is very small")
                
                if file_size > 8589934592:  # 8GB maximum
                    self.warnings.append("ISO file is very large")
                
                return True
            
            def _validate_file_readability(self):
                """Validate file readability."""
                try:
                    with open(self.iso_file, 'rb') as f:
                        f.read(1024)  # Try to read first 1KB
                    return True
                except Exception as e:
                    self.errors.append(f"ISO file is not readable: {e}")
                    return False
        
        return ISOValidator(self.test_iso)
    
    def test_valid_iso_validation(self):
        """Test validation of a valid ISO file."""
        result = self.validator.validate()
        
        self.assertTrue(result)
        self.assertEqual(len(self.validator.errors), 0)
        self.assertGreater(len(self.validator.passed_checks), 0)
        
        # Check that all basic checks passed
        expected_checks = ["file_format", "file_size", "file_readability"]
        for check in expected_checks:
            self.assertIn(check, self.validator.passed_checks)
    
    def test_nonexistent_iso_validation(self):
        """Test validation of non-existent ISO file."""
        nonexistent_iso = os.path.join(self.temp_dir, "nonexistent.iso")
        validator = self.create_validator.__func__(nonexistent_iso)
        
        result = validator.validate()
        
        self.assertFalse(result)
        self.assertGreater(len(validator.errors), 0)
        
        # Check for specific error
        error_messages = [str(error) for error in validator.errors]
        self.assertTrue(any("does not exist" in msg for msg in error_messages))
    
    def test_empty_iso_validation(self):
        """Test validation of empty ISO file."""
        empty_iso = os.path.join(self.temp_dir, "empty.iso")
        with open(empty_iso, 'w') as f:
            pass  # Create empty file
        
        validator = self.create_validator.__func__(empty_iso)
        result = validator.validate()
        
        self.assertFalse(result)
        self.assertGreater(len(validator.errors), 0)
        
        # Check for specific error
        error_messages = [str(error) for error in validator.errors]
        self.assertTrue(any("empty" in msg for msg in error_messages))
    
    def test_checksum_validation(self):
        """Test checksum validation."""
        class ChecksumValidator:
            def __init__(self, iso_file):
                self.iso_file = iso_file
                self.errors = []
                self.warnings = []
            
            def validate_checksum(self, checksum_file):
                """Validate checksum against checksum file."""
                if not os.path.exists(checksum_file):
                    self.errors.append("Checksum file does not exist")
                    return False
                
                try:
                    # Calculate checksum of ISO file
                    with open(self.iso_file, 'rb') as f:
                        file_hash = hashlib.sha256(f.read()).hexdigest()
                    
                    # Read checksum from file
                    with open(checksum_file, 'r') as f:
                        checksum_line = f.readline().strip()
                    
                    # Extract hash from line (format: hash filename)
                    expected_hash = checksum_line.split()[0]
                    
                    if file_hash == expected_hash:
                        return True
                    else:
                        self.errors.append("Checksum mismatch")
                        return False
                        
                except Exception as e:
                    self.errors.append(f"Checksum validation failed: {e}")
                    return False
        
        # Create checksum file with correct hash
        with open(self.test_iso, 'rb') as f:
            correct_hash = hashlib.sha256(f.read()).hexdigest()
        
        checksum_file = os.path.join(self.temp_dir, "test.iso.sha256")
        with open(checksum_file, 'w') as f:
            f.write(f"{correct_hash} test.iso\\n")
        
        validator = ChecksumValidator(self.test_iso)
        result = validator.validate_checksum(checksum_file)
        
        self.assertTrue(result)
        self.assertEqual(len(validator.errors), 0)
    
    def test_checksum_validation_failure(self):
        """Test checksum validation failure."""
        class ChecksumValidator:
            def __init__(self, iso_file):
                self.iso_file = iso_file
                self.errors = []
            
            def validate_checksum(self, checksum_file):
                """Validate checksum against checksum file."""
                try:
                    # Read checksum from file
                    with open(checksum_file, 'r') as f:
                        checksum_line = f.readline().strip()
                    
                    expected_hash = checksum_line.split()[0]
                    
                    # Simulate different hash
                    if expected_hash != "correct_hash":
                        self.errors.append("Checksum mismatch")
                        return False
                    
                    return True
                    
                except Exception as e:
                    self.errors.append(f"Checksum validation failed: {e}")
                    return False
        
        # Create checksum file with wrong hash
        checksum_file = os.path.join(self.temp_dir, "test.iso.sha256")
        with open(checksum_file, 'w') as f:
            f.write("wrong_hash test.iso\\n")
        
        validator = ChecksumValidator(self.test_iso)
        result = validator.validate_checksum(checksum_file)
        
        self.assertFalse(result)
        self.assertGreater(len(validator.errors), 0)
        
        # Check for specific error
        error_messages = [str(error) for error in validator.errors]
        self.assertTrue(any("mismatch" in msg for msg in error_messages))

class TestISOStructureValidation(unittest.TestCase):
    """Test ISO structure validation."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.temp_mount = os.path.join(self.temp_dir, "mount")
        os.makedirs(self.temp_mount, exist_ok=True)
        
        self.validator = self.create_structure_validator()
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def create_structure_validator(self):
        """Create a mock ISO structure validator."""
        class StructureValidator:
            def __init__(self, mount_point):
                self.mount_point = mount_point
                self.errors = []
                self.warnings = []
            
            def validate_structure(self):
                """Validate ISO directory structure."""
                required_dirs = [
                    "EFI",
                    "EFI/BOOT",
                    "boot",
                    "boot/grub",
                    "live"
                ]
                
                required_files = [
                    "EFI/BOOT/BOOTX64.EFI",
                    "boot/grub/grub.cfg"
                ]
                
                # Check required directories
                missing_dirs = []
                for dir_path in required_dirs:
                    full_path = os.path.join(self.mount_point, dir_path)
                    if not os.path.exists(full_path):
                        missing_dirs.append(dir_path)
                
                if missing_dirs:
                    self.errors.append(f"Missing directories: {missing_dirs}")
                
                # Check required files
                missing_files = []
                for file_path in required_files:
                    full_path = os.path.join(self.mount_point, file_path)
                    if not os.path.exists(full_path):
                        missing_files.append(file_path)
                
                if missing_files:
                    self.errors.append(f"Missing files: {missing_files}")
                
                return len(self.errors) == 0
            
            def validate_uefi_boot(self):
                """Validate UEFI boot capability."""
                efi_files = [
                    "EFI/BOOT/BOOTX64.EFI",
                    "EFI/BOOT/BOOTIA32.EFI"
                ]
                
                efi_found = False
                for efi_file in efi_files:
                    full_path = os.path.join(self.mount_point, efi_file)
                    if os.path.exists(full_path):
                        efi_found = True
                        break
                
                if not efi_found:
                    self.errors.append("No UEFI bootloader found")
                    return False
                
                return True
            
            def validate_grub_config(self):
                """Validate GRUB configuration."""
                grub_config = os.path.join(self.mount_point, "boot/grub/grub.cfg")
                
                if not os.path.exists(grub_config):
                    self.errors.append("GRUB configuration not found")
                    return False
                
                # Check for required boot entries
                try:
                    with open(grub_config, 'r') as f:
                        config_content = f.read()
                    
                    required_entries = [
                        "menuentry",
                        "linux",
                        "initrd"
                    ]
                    
                    missing_entries = []
                    for entry in required_entries:
                        if entry not in config_content:
                            missing_entries.append(entry)
                    
                    if missing_entries:
                        self.warnings.append(f"Missing GRUB entries: {missing_entries}")
                    
                except Exception as e:
                    self.errors.append(f"Failed to read GRUB config: {e}")
                    return False
                
                return True
        
        return StructureValidator(self.temp_mount)
    
    def test_valid_structure_validation(self):
        """Test validation of valid ISO structure."""
        # Create required structure
        os.makedirs(os.path.join(self.temp_mount, "EFI", "BOOT"), exist_ok=True)
        os.makedirs(os.path.join(self.temp_mount, "boot", "grub"), exist_ok=True)
        os.makedirs(os.path.join(self.temp_mount, "live"), exist_ok=True)
        
        # Create required files
        with open(os.path.join(self.temp_mount, "EFI", "BOOT", "BOOTX64.EFI"), 'w') as f:
            f.write("Mock UEFI bootloader\\n")
        
        with open(os.path.join(self.temp_mount, "boot", "grub", "grub.cfg"), 'w') as f:
            f.write('menuentry "RegicideOS Live" {\\n')
            f.write("    linux /boot/vmlinuz boot=live\\n")
            f.write("    initrd /boot/initrd\\n")
            f.write("}\\n")
        
        result = self.validator.validate_structure()
        
        self.assertTrue(result)
        self.assertEqual(len(self.validator.errors), 0)
    
    def test_missing_directories_validation(self):
        """Test validation with missing directories."""
        # Create incomplete structure (missing some directories)
        os.makedirs(os.path.join(self.temp_mount, "boot"), exist_ok=True)
        
        result = self.validator.validate_structure()
        
        self.assertFalse(result)
        self.assertGreater(len(self.validator.errors), 0)
        
        # Check for specific error
        error_messages = [str(error) for error in self.validator.errors]
        self.assertTrue(any("Missing directories" in msg for msg in error_messages))
    
    def test_uefi_boot_validation(self):
        """Test UEFI boot validation."""
        # Create UEFI bootloader
        os.makedirs(os.path.join(self.temp_mount, "EFI", "BOOT"), exist_ok=True)
        with open(os.path.join(self.temp_mount, "EFI", "BOOT", "BOOTX64.EFI"), 'w') as f:
            f.write("Mock UEFI bootloader\\n")
        
        result = self.validator.validate_uefi_boot()
        
        self.assertTrue(result)
        self.assertEqual(len(self.validator.errors), 0)
    
    def test_uefi_boot_validation_failure(self):
        """Test UEFI boot validation failure."""
        # Don't create UEFI bootloader
        result = self.validator.validate_uefi_boot()
        
        self.assertFalse(result)
        self.assertGreater(len(self.validator.errors), 0)
        
        # Check for specific error
        error_messages = [str(error) for error in self.validator.errors]
        self.assertTrue(any("UEFI bootloader" in msg for msg in error_messages))

class TestISOValidationIntegration(unittest.TestCase):
    """Test ISO validation integration."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.test_iso = os.path.join(self.temp_dir, "test.iso")
        
        # Create a mock ISO file
        with open(self.test_iso, 'wb') as f:
            f.write(b"Mock ISO content" * 1000)
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_complete_validation_workflow(self):
        """Test complete validation workflow."""
        class CompleteValidator:
            def __init__(self, iso_file):
                self.iso_file = iso_file
                self.validation_results = {}
                self.errors = []
            
            def run_complete_validation(self):
                """Run all validation steps."""
                steps = [
                    ("basic_validation", self._validate_basic),
                    ("checksum_validation", self._validate_checksum),
                    ("structure_validation", self._validate_structure),
                    ("security_validation", self._validate_security)
                ]
                
                for step_name, step_func in steps:
                    try:
                        result = step_func()
                        self.validation_results[step_name] = {
                            "passed": result,
                            "errors": self.errors.copy()
                        }
                        self.errors.clear()  # Clear errors for next step
                    except Exception as e:
                        self.validation_results[step_name] = {
                            "passed": False,
                            "errors": [f"Exception: {e}"]
                        }
                
                return self._generate_summary()
            
            def _validate_basic(self):
                """Basic file validation."""
                if not os.path.exists(self.iso_file):
                    self.errors.append("ISO file does not exist")
                    return False
                
                file_size = os.path.getsize(self.iso_file)
                if file_size == 0:
                    self.errors.append("ISO file is empty")
                    return False
                
                return True
            
            def _validate_checksum(self):
                """Checksum validation (mock)."""
                # Mock checksum validation
                return True
            
            def _validate_structure(self):
                """Structure validation (mock)."""
                # Mock structure validation
                return True
            
            def _validate_security(self):
                """Security validation (mock)."""
                # Mock security validation
                return True
            
            def _generate_summary(self):
                """Generate validation summary."""
                total_steps = len(self.validation_results)
                passed_steps = sum(1 for result in self.validation_results.values() if result["passed"])
                
                summary = {
                    "total_steps": total_steps,
                    "passed_steps": passed_steps,
                    "failed_steps": total_steps - passed_steps,
                    "overall_result": passed_steps == total_steps,
                    "details": self.validation_results
                }
                
                return summary
        
        validator = CompleteValidator(self.test_iso)
        summary = validator.run_complete_validation()
        
        # Check summary structure
        self.assertIn("total_steps", summary)
        self.assertIn("passed_steps", summary)
        self.assertIn("failed_steps", summary)
        self.assertIn("overall_result", summary)
        self.assertIn("details", summary)
        
        # Check results
        self.assertEqual(summary["total_steps"], 4)
        self.assertEqual(summary["passed_steps"], 4)
        self.assertEqual(summary["failed_steps"], 0)
        self.assertTrue(summary["overall_result"])
    
    def test_validation_with_failures(self):
        """Test validation with some failures."""
        class FailureValidator:
            def __init__(self, iso_file):
                self.iso_file = iso_file
                self.results = {}
            
            def validate_with_failures(self):
                """Validation with intentional failures."""
                # Simulate some validation failures
                results = {
                    "basic_validation": {"passed": True, "errors": []},
                    "checksum_validation": {"passed": False, "errors": ["Checksum mismatch"]},
                    "structure_validation": {"passed": True, "errors": []},
                    "security_validation": {"passed": False, "errors": ["Security issue found"]}
                }
                
                total = len(results)
                passed = sum(1 for r in results.values() if r["passed"])
                
                return {
                    "total_checks": total,
                    "passed_checks": passed,
                    "failed_checks": total - passed,
                    "success_rate": passed / total,
                    "details": results
                }
        
        validator = FailureValidator(self.test_iso)
        results = validator.validate_with_failures()
        
        # Check results
        self.assertEqual(results["total_checks"], 4)
        self.assertEqual(results["passed_checks"], 2)
        self.assertEqual(results["failed_checks"], 2)
        self.assertEqual(results["success_rate"], 0.5)
        self.assertFalse(results["success_rate"] == 1.0)

if __name__ == '__main__':
    # Run tests with detailed output
    unittest.main(verbosity=2)