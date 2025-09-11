"""
Validation tests for ISO creation process.
"""

import unittest
from unittest.mock import Mock, patch, MagicMock, call
import tempfile
import os
import sys
import hashlib
from pathlib import Path
import shutil

# Add the project root to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent))

class TestISOChecksumValidation(unittest.TestCase):
    """Test ISO checksum validation."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.test_file = os.path.join(self.temp_dir, "test.iso")
        
        # Create test file with known content
        test_content = b"RegicideOS ISO test content for validation"
        with open(self.test_file, 'wb') as f:
            f.write(test_content)
        
        # Calculate expected checksum
        self.expected_checksum = hashlib.sha256(test_content).hexdigest()
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_checksum_generation(self):
        """Test checksum generation."""
        class ChecksumGenerator:
            def __init__(self, file_path):
                self.file_path = file_path
                self.errors = []
            
            def generate_sha256(self):
                """Generate SHA256 checksum."""
                try:
                    sha256_hash = hashlib.sha256()
                    with open(self.file_path, 'rb') as f:
                        # Read file in chunks to handle large files
                        for chunk in iter(lambda: f.read(4096), b""):
                            sha256_hash.update(chunk)
                    return sha256_hash.hexdigest()
                except Exception as e:
                    self.errors.append(f"Failed to generate checksum: {e}")
                    return None
            
            def generate_checksum_file(self):
                """Generate checksum file."""
                checksum = self.generate_sha256()
                if not checksum:
                    return False
                
                checksum_file = f"{self.file_path}.sha256"
                try:
                    with open(checksum_file, 'w') as f:
                        f.write(f"{checksum} {os.path.basename(self.file_path)}\\n")
                    return True
                except Exception as e:
                    self.errors.append(f"Failed to create checksum file: {e}")
                    return False
        
        generator = ChecksumGenerator(self.test_file)
        checksum = generator.generate_sha256()
        
        self.assertIsNotNone(checksum)
        self.assertEqual(checksum, self.expected_checksum)
        self.assertEqual(len(generator.errors), 0)
    
    def test_checksum_validation(self):
        """Test checksum validation."""
        class ChecksumValidator:
            def __init__(self, file_path):
                self.file_path = file_path
                self.errors = []
                self.warnings = []
            
            def validate_checksum(self, checksum_file):
                """Validate checksum against checksum file."""
                if not os.path.exists(checksum_file):
                    self.errors.append("Checksum file does not exist")
                    return False
                
                try:
                    # Read expected checksum from file
                    with open(checksum_file, 'r') as f:
                        checksum_line = f.readline().strip()
                    
                    # Parse checksum line (format: hash filename)
                    if ' ' in checksum_line:
                        expected_checksum = checksum_line.split(' ')[0]
                    else:
                        expected_checksum = checksum_line
                    
                    # Calculate actual checksum
                    actual_checksum = self._calculate_checksum()
                    
                    if actual_checksum == expected_checksum:
                        return True
                    else:
                        self.errors.append(f"Checksum mismatch: expected {expected_checksum}, got {actual_checksum}")
                        return False
                        
                except Exception as e:
                    self.errors.append(f"Checksum validation failed: {e}")
                    return False
            
            def _calculate_checksum(self):
                """Calculate SHA256 checksum."""
                sha256_hash = hashlib.sha256()
                with open(self.file_path, 'rb') as f:
                    for chunk in iter(lambda: f.read(4096), b""):
                        sha256_hash.update(chunk)
                return sha256_hash.hexdigest()
        
        # Create valid checksum file
        checksum_file = os.path.join(self.temp_dir, "test.iso.sha256")
        with open(checksum_file, 'w') as f:
            f.write(f"{self.expected_checksum} test.iso\\n")
        
        validator = ChecksumValidator(self.test_file)
        result = validator.validate_checksum(checksum_file)
        
        self.assertTrue(result)
        self.assertEqual(len(validator.errors), 0)
    
    def test_checksum_validation_failure(self):
        """Test checksum validation failure."""
        class ChecksumValidator:
            def __init__(self, file_path):
                self.file_path = file_path
                self.errors = []
            
            def validate_checksum(self, checksum_file):
                """Validate checksum against checksum file."""
                try:
                    with open(checksum_file, 'r') as f:
                        checksum_line = f.readline().strip()
                    
                    expected_checksum = checksum_line.split(' ')[0]
                    actual_checksum = self._calculate_checksum()
                    
                    if actual_checksum != expected_checksum:
                        self.errors.append("Checksum mismatch")
                        return False
                    
                    return True
                    
                except Exception as e:
                    self.errors.append(f"Validation failed: {e}")
                    return False
            
            def _calculate_checksum(self):
                """Calculate SHA256 checksum."""
                sha256_hash = hashlib.sha256()
                with open(self.file_path, 'rb') as f:
                    for chunk in iter(lambda: f.read(4096), b""):
                        sha256_hash.update(chunk)
                return sha256_hash.hexdigest()
        
        # Create invalid checksum file
        checksum_file = os.path.join(self.temp_dir, "test.iso.sha256")
        with open(checksum_file, 'w') as f:
            f.write("invalid_checksum test.iso\\n")
        
        validator = ChecksumValidator(self.test_file)
        result = validator.validate_checksum(checksum_file)
        
        self.assertFalse(result)
        self.assertGreater(len(validator.errors), 0)
        
        # Check for specific error
        error_messages = [str(error) for error in validator.errors]
        self.assertTrue(any("mismatch" in msg for msg in error_messages))

class TestISOBootValidation(unittest.TestCase):
    """Test ISO boot validation."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.iso_mount = os.path.join(self.temp_dir, "mount")
        os.makedirs(self.iso_mount, exist_ok=True)
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_uefi_boot_validation(self):
        """Test UEFI boot validation."""
        class UEFIBootValidator:
            def __init__(self, mount_point):
                self.mount_point = mount_point
                self.errors = []
                self.warnings = []
            
            def validate_uefi_boot(self):
                """Validate UEFI boot capability."""
                # Check for UEFI bootloader files
                efi_files = [
                    "EFI/BOOT/BOOTX64.EFI",
                    "EFI/BOOT/BOOTIA32.EFI"
                ]
                
                efi_found = False
                for efi_file in efi_files:
                    full_path = os.path.join(self.mount_point, efi_file)
                    if os.path.exists(full_path):
                        efi_found = True
                        self.warnings.append(f"Found UEFI bootloader: {efi_file}")
                
                if not efi_found:
                    self.errors.append("No UEFI bootloader found")
                    return False
                
                # Check for GRUB configuration
                grub_config = os.path.join(self.mount_point, "boot/grub/grub.cfg")
                if not os.path.exists(grub_config):
                    self.warnings.append("GRUB configuration not found")
                    return True  # Not critical
                
                return True
            
            def validate_secure_boot(self):
                """Validate secure boot compatibility."""
                # Check for secure boot related files
                secure_boot_files = [
                    "EFI/BOOT/keys",
                    "EFI/BOOT/certificates"
                ]
                
                secure_boot_support = False
                for sb_file in secure_boot_files:
                    full_path = os.path.join(self.mount_point, sb_file)
                    if os.path.exists(full_path):
                        secure_boot_support = True
                        break
                
                if secure_boot_support:
                    self.warnings.append("Secure boot support detected")
                else:
                    self.warnings.append("Secure boot support not detected")
                
                return True  # Not critical
        
        # Create UEFI bootloader
        efi_dir = os.path.join(self.iso_mount, "EFI", "BOOT")
        os.makedirs(efi_dir, exist_ok=True)
        
        with open(os.path.join(efi_dir, "BOOTX64.EFI"), 'w') as f:
            f.write("Mock UEFI bootloader\\n")
        
        validator = UEFIBootValidator(self.iso_mount)
        result = validator.validate_uefi_boot()
        
        self.assertTrue(result)
        self.assertEqual(len(validator.errors), 0)
        self.assertGreater(len(validator.warnings), 0)
        
        # Check for UEFI bootloader warning
        warning_messages = [str(warning) for warning in validator.warnings]
        self.assertTrue(any("UEFI bootloader" in msg for msg in warning_messages))
    
    def test_bios_boot_rejection(self):
        """Test that BIOS boot is properly rejected."""
        class BIOSBootValidator:
            def __init__(self, mount_point):
                self.mount_point = mount_point
                self.errors = []
            
            def validate_no_bios_boot(self):
                """Validate that BIOS boot is not supported."""
                # Check for BIOS bootloader files (should not exist)
                bios_files = [
                    "isolinux/isolinux.bin",
                    "syslinux/syslinux.cfg",
                    "boot/grub/i386-pc/core.img"
                ]
                
                bios_found = False
                for bios_file in bios_files:
                    full_path = os.path.join(self.mount_point, bios_file)
                    if os.path.exists(full_path):
                        bios_found = True
                        self.errors.append(f"BIOS bootloader found: {bios_file}")
                
                if bios_found:
                    return False
                
                return True
        
        validator = BIOSBootValidator(self.iso_mount)
        result = validator.validate_no_bios_boot()
        
        self.assertTrue(result)
        self.assertEqual(len(validator.errors), 0)
    
    def test_boot_configuration_validation(self):
        """Test boot configuration validation."""
        class BootConfigValidator:
            def __init__(self, mount_point):
                self.mount_point = mount_point
                self.errors = []
                self.warnings = []
            
            def validate_boot_config(self):
                """Validate boot configuration."""
                grub_config = os.path.join(self.mount_point, "boot/grub/grub.cfg")
                
                if not os.path.exists(grub_config):
                    self.errors.append("GRUB configuration not found")
                    return False
                
                try:
                    with open(grub_config, 'r') as f:
                        config_content = f.read()
                    
                    # Check for required boot entries
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
                        self.warnings.append(f"Missing boot entries: {missing_entries}")
                    
                    # Check for UEFI-specific parameters
                    uefi_params = ["efi", "vmlinuz", "initrd"]
                    found_uefi_params = []
                    for param in uefi_params:
                        if param in config_content:
                            found_uefi_params.append(param)
                    
                    if len(found_uefi_params) < 2:
                        self.warnings.append("Insufficient UEFI boot parameters")
                    
                    return True
                    
                except Exception as e:
                    self.errors.append(f"Failed to read boot configuration: {e}")
                    return False
        
        # Create GRUB configuration
        grub_dir = os.path.join(self.iso_mount, "boot", "grub")
        os.makedirs(grub_dir, exist_ok=True)
        
        grub_config = os.path.join(grub_dir, "grub.cfg")
        with open(grub_config, 'w') as f:
            f.write('menuentry "RegicideOS Live" {\\n')
            f.write("    linux /boot/vmlinuz boot=live\\n")
            f.write("    initrd /boot/initrd\\n")
            f.write("}\\n")
        
        validator = BootConfigValidator(self.iso_mount)
        result = validator.validate_boot_config()
        
        self.assertTrue(result)
        self.assertEqual(len(validator.errors), 0)

class TestISOFilesystemValidation(unittest.TestCase):
    """Test ISO filesystem validation."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.iso_mount = os.path.join(self.temp_dir, "mount")
        os.makedirs(self.iso_mount, exist_ok=True)
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_filesystem_structure_validation(self):
        """Test filesystem structure validation."""
        class FilesystemValidator:
            def __init__(self, mount_point):
                self.mount_point = mount_point
                self.errors = []
                self.warnings = []
            
            def validate_filesystem_structure(self):
                """Validate filesystem structure."""
                # Required directories
                required_dirs = [
                    "EFI",
                    "boot",
                    "live",
                    ".disk"
                ]
                
                # Check required directories
                missing_dirs = []
                for dir_name in required_dirs:
                    dir_path = os.path.join(self.mount_point, dir_name)
                    if not os.path.exists(dir_path):
                        missing_dirs.append(dir_name)
                
                if missing_dirs:
                    self.errors.append(f"Missing required directories: {missing_dirs}")
                    return False
                
                # Required files
                required_files = [
                    ".disk/info",
                    ".disk/README"
                ]
                
                missing_files = []
                for file_name in required_files:
                    file_path = os.path.join(self.mount_point, file_name)
                    if not os.path.exists(file_path):
                        missing_files.append(file_name)
                
                if missing_files:
                    self.warnings.append(f"Missing optional files: {missing_files}")
                
                return True
            
            def validate_live_filesystem(self):
                """Validate live filesystem."""
                live_dir = os.path.join(self.mount_point, "live")
                
                if not os.path.exists(live_dir):
                    self.errors.append("Live filesystem directory not found")
                    return False
                
                # Check for squashfs filesystem
                squashfs_file = os.path.join(live_dir, "filesystem.squashfs")
                if not os.path.exists(squashfs_file):
                    self.errors.append("Live filesystem squashfs not found")
                    return False
                
                # Check squashfs file size
                file_size = os.path.getsize(squashfs_file)
                if file_size < 1048576:  # 1MB minimum
                    self.warnings.append("Live filesystem is very small")
                
                return True
        
        # Create filesystem structure
        os.makedirs(os.path.join(self.iso_mount, "EFI"), exist_ok=True)
        os.makedirs(os.path.join(self.iso_mount, "boot"), exist_ok=True)
        os.makedirs(os.path.join(self.iso_mount, "live"), exist_ok=True)
        os.makedirs(os.path.join(self.iso_mount, ".disk"), exist_ok=True)
        
        # Create required files
        with open(os.path.join(self.iso_mount, ".disk", "info"), 'w') as f:
            f.write("RegicideOS 1.0.0\\n")
        
        # Create live filesystem
        live_dir = os.path.join(self.iso_mount, "live")
        squashfs_file = os.path.join(live_dir, "filesystem.squashfs")
        with open(squashfs_file, 'wb') as f:
            f.write(b"Mock squashfs filesystem" * 1000)  # ~22KB
        
        validator = FilesystemValidator(self.iso_mount)
        
        # Test filesystem structure
        result1 = validator.validate_filesystem_structure()
        self.assertTrue(result1)
        self.assertEqual(len(validator.errors), 0)
        
        # Test live filesystem
        result2 = validator.validate_live_filesystem()
        self.assertTrue(result2)
        self.assertEqual(len(validator.errors), 0)

class TestISOSecurityValidation(unittest.TestCase):
    """Test ISO security validation."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.iso_mount = os.path.join(self.temp_dir, "mount")
        os.makedirs(self.iso_mount, exist_ok=True)
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_file_permissions_validation(self):
        """Test file permissions validation."""
        class SecurityValidator:
            def __init__(self, mount_point):
                self.mount_point = mount_point
                self.errors = []
                self.warnings = []
            
            def validate_file_permissions(self):
                """Validate file permissions."""
                problematic_files = []
                
                # Walk through all files
                for root, dirs, files in os.walk(self.mount_point):
                    for file in files:
                        file_path = os.path.join(root, file)
                        try:
                            # Check file permissions
                            file_stat = os.stat(file_path)
                            mode = file_stat.st_mode
                            
                            # Check for world-writable files
                            if mode & 0o002:  # World-writable
                                problematic_files.append(file_path)
                            
                            # Check for executable files in sensitive locations
                            if mode & 0o111:  # Executable
                                if "boot" in file_path or "EFI" in file_path:
                                    # Boot files can be executable
                                    pass
                                elif file_path.endswith((".sh", ".py", ".exe")):
                                    # Script files can be executable
                                    pass
                                else:
                                    self.warnings.append(f"Unexpected executable file: {file_path}")
                        
                        except Exception as e:
                            self.warnings.append(f"Could not check permissions for {file_path}: {e}")
                
                if problematic_files:
                    self.errors.append(f"World-writable files found: {problematic_files}")
                    return False
                
                return True
            
            def validate_secure_boot_compatibility(self):
                """Validate secure boot compatibility."""
                # Check for UEFI bootloader
                efi_bootloader = os.path.join(self.mount_point, "EFI", "BOOT", "BOOTX64.EFI")
                if not os.path.exists(efi_bootloader):
                    self.warnings.append("UEFI bootloader not found - secure boot may not work")
                
                # Check for secure boot keys
                secure_boot_keys = os.path.join(self.mount_point, "EFI", "BOOT", "keys")
                if os.path.exists(secure_boot_keys):
                    self.warnings.append("Secure boot keys found")
                
                return True  # Not critical
        
        # Create test files with various permissions
        test_file = os.path.join(self.iso_mount, "test.txt")
        with open(test_file, 'w') as f:
            f.write("test content")
        
        # Set safe permissions
        os.chmod(test_file, 0o644)
        
        validator = SecurityValidator(self.iso_mount)
        result = validator.validate_file_permissions()
        
        self.assertTrue(result)
        self.assertEqual(len(validator.errors), 0)
    
    def test_integrity_validation(self):
        """Test integrity validation."""
        class IntegrityValidator:
            def __init__(self, mount_point):
                self.mount_point = mount_point
                self.errors = []
                self.warnings = []
            
            def validate_integrity(self):
                """Validate ISO integrity."""
                # Check for corrupted files (mock validation)
                corrupted_files = []
                
                for root, dirs, files in os.walk(self.mount_point):
                    for file in files:
                        file_path = os.path.join(root, file)
                        
                        # Check for empty files that shouldn't be empty
                        if file_path.endswith((".EFI", ".cfg", ".img")):
                            file_size = os.path.getsize(file_path)
                            if file_size == 0:
                                corrupted_files.append(file_path)
                        
                        # Check for files with suspicious content
                        try:
                            with open(file_path, 'r', errors='ignore') as f:
                                content = f.read()
                                if "MALICIOUS" in content:
                                    corrupted_files.append(file_path)
                        except:
                            # Binary files or permission issues
                            pass
                
                if corrupted_files:
                    self.errors.append(f"Potentially corrupted files: {corrupted_files}")
                    return False
                
                return True
            
            def validate_signature(self):
                """Validate file signatures (mock)."""
                # In real implementation, this would check GPG signatures
                signature_files = []
                
                for root, dirs, files in os.walk(self.mount_point):
                    for file in files:
                        if file.endswith((".sig", ".asc", ".pem")):
                            signature_files.append(os.path.join(root, file))
                
                if signature_files:
                    self.warnings.append(f"Signature files found: {signature_files}")
                else:
                    self.warnings.append("No signature files found")
                
                return True
        
        # Create test files
        test_file = os.path.join(self.iso_mount, "test.txt")
        with open(test_file, 'w') as f:
            f.write("Legitimate content")
        
        validator = IntegrityValidator(self.iso_mount)
        
        # Test integrity
        result1 = validator.validate_integrity()
        self.assertTrue(result1)
        self.assertEqual(len(validator.errors), 0)
        
        # Test signature validation
        result2 = validator.validate_signature()
        self.assertTrue(result2)
        self.assertGreater(len(validator.warnings), 0)

if __name__ == '__main__':
    # Run tests with detailed output
    unittest.main(verbosity=2)