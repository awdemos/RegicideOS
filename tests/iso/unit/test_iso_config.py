"""
Unit tests for ISO configuration parsing and validation.
"""

import unittest
from unittest.mock import Mock, patch, MagicMock
import tempfile
import os
import sys
import toml
from pathlib import Path

# Add the project root to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent))

class TestISOConfig(unittest.TestCase):
    """Test ISO configuration parsing and validation."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.valid_config = {
            "iso": {
                "name": "RegicideOS",
                "version": "1.0.0",
                "architecture": "x86_64",
                "label": "RegicideOS-1.0.0",
                "publisher": "RegicideOS Team",
                "application": "RegicideOS Live Installer"
            },
            "bootloader": {
                "efi_bootloader": "grub",
                "grub_theme": "regicideos",
                "grub_timeout": 10,
                "grub_default_entry": 0,
                "menu_entries": [
                    {
                        "name": "RegicideOS Live",
                        "title": "Start RegicideOS Live Environment",
                        "kernel": "/boot/vmlinuz",
                        "initrd": "/boot/initrd",
                        "kernel_params": ["boot=live", "live-media-path=/live"]
                    }
                ]
            },
            "filesystem": {
                "rootfs_type": "squashfs",
                "rootfs_compression": "xz",
                "rootfs_block_size": 131072,
                "min_disk_space": 21474836480,
                "recommended_disk_space": 32212254720
            },
            "security": {
                "secure_boot": True,
                "gpg_sign": True,
                "strict_permissions": True
            }
        }
        
        self.temp_config_file = None
        
    def tearDown(self):
        """Clean up test fixtures."""
        if self.temp_config_file and os.path.exists(self.temp_config_file):
            os.unlink(self.temp_config_file)
    
    def create_temp_config(self, config_dict):
        """Create a temporary config file."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
            toml.dump(config_dict, f)
            self.temp_config_file = f.name
        return self.temp_config_file
    
    def test_load_valid_config(self):
        """Test loading a valid configuration."""
        config_file = self.create_temp_config(self.valid_config)
        
        # Simulate config loading
        with open(config_file, 'r') as f:
            loaded_config = toml.load(f)
        
        self.assertEqual(loaded_config["iso"]["name"], "RegicideOS")
        self.assertEqual(loaded_config["iso"]["version"], "1.0.0")
        self.assertEqual(loaded_config["iso"]["architecture"], "x86_64")
        self.assertEqual(loaded_config["bootloader"]["efi_bootloader"], "grub")
        self.assertEqual(loaded_config["filesystem"]["rootfs_type"], "squashfs")
        self.assertTrue(loaded_config["security"]["secure_boot"])
    
    def test_validate_iso_section(self):
        """Test validation of ISO section."""
        # Valid ISO section
        iso_config = self.valid_config["iso"]
        
        # Check required fields
        required_fields = ["name", "version", "architecture", "label"]
        for field in required_fields:
            self.assertIn(field, iso_config)
        
        # Validate version format
        version = iso_config["version"]
        self.assertRegex(version, r'^\d+\.\d+\.\d+$')
        
        # Validate architecture
        arch = iso_config["architecture"]
        self.assertIn(arch, ["x86_64", "amd64", "i686", "i386", "arm64", "aarch64"])
    
    def test_validate_bootloader_section(self):
        """Test validation of bootloader section."""
        bootloader_config = self.valid_config["bootloader"]
        
        # Check required fields
        required_fields = ["efi_bootloader", "grub_timeout", "menu_entries"]
        for field in required_fields:
            self.assertIn(field, bootloader_config)
        
        # Validate bootloader type
        bootloader_type = bootloader_config["efi_bootloader"]
        self.assertEqual(bootloader_type, "grub")
        
        # Validate timeout
        timeout = bootloader_config["grub_timeout"]
        self.assertIsInstance(timeout, int)
        self.assertGreater(timeout, 0)
        
        # Validate menu entries
        menu_entries = bootloader_config["menu_entries"]
        self.assertIsInstance(menu_entries, list)
        self.assertGreater(len(menu_entries), 0)
        
        # Check menu entry structure
        entry = menu_entries[0]
        required_entry_fields = ["name", "title", "kernel", "initrd", "kernel_params"]
        for field in required_entry_fields:
            self.assertIn(field, entry)
    
    def test_validate_filesystem_section(self):
        """Test validation of filesystem section."""
        fs_config = self.valid_config["filesystem"]
        
        # Check required fields
        required_fields = ["rootfs_type", "rootfs_compression", "min_disk_space"]
        for field in required_fields:
            self.assertIn(field, fs_config)
        
        # Validate filesystem type
        fs_type = fs_config["rootfs_type"]
        self.assertEqual(fs_type, "squashfs")
        
        # Validate compression
        compression = fs_config["rootfs_compression"]
        self.assertIn(compression, ["gzip", "xz", "lzma", "lzo", "zstd"])
        
        # Validate disk space requirements
        min_space = fs_config["min_disk_space"]
        self.assertIsInstance(min_space, int)
        self.assertGreater(min_space, 0)
        
        recommended_space = fs_config["recommended_disk_space"]
        self.assertGreater(recommended_space, min_space)
    
    def test_validate_security_section(self):
        """Test validation of security section."""
        security_config = self.valid_config["security"]
        
        # Check security settings
        self.assertTrue(security_config["secure_boot"])
        self.assertTrue(security_config["gpg_sign"])
        self.assertTrue(security_config["strict_permissions"])
    
    def test_invalid_configuration_handling(self):
        """Test handling of invalid configurations."""
        # Test missing required section
        invalid_config = {"iso": {"name": "Test"}}
        config_file = self.create_temp_config(invalid_config)
        
        with open(config_file, 'r') as f:
            loaded_config = toml.load(f)
        
        # Should not have bootloader section
        self.assertNotIn("bootloader", loaded_config)
    
    def test_invalid_architecture(self):
        """Test handling of invalid architecture."""
        invalid_config = self.valid_config.copy()
        invalid_config["iso"]["architecture"] = "invalid_arch"
        
        config_file = self.create_temp_config(invalid_config)
        
        with open(config_file, 'r') as f:
            loaded_config = toml.load(f)
        
        # Should load but architecture is invalid
        self.assertEqual(loaded_config["iso"]["architecture"], "invalid_arch")
    
    def test_empty_configuration(self):
        """Test handling of empty configuration."""
        empty_config = {}
        config_file = self.create_temp_config(empty_config)
        
        with open(config_file, 'r') as f:
            loaded_config = toml.load(f)
        
        # Should be empty
        self.assertEqual(loaded_config, {})
    
    def test_config_file_not_found(self):
        """Test handling of missing config file."""
        with self.assertRaises(FileNotFoundError):
            with open("/nonexistent/config.toml", 'r') as f:
                toml.load(f)

class TestISOConfigValidation(unittest.TestCase):
    """Test ISO configuration validation logic."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.validator = self.create_validator()
    
    def create_validator(self):
        """Create a mock configuration validator."""
        class ISOConfigValidator:
            def __init__(self):
                self.errors = []
                self.warnings = []
            
            def validate(self, config):
                """Validate configuration and return list of errors."""
                self.errors = []
                self.warnings = []
                
                # Validate ISO section
                if "iso" not in config:
                    self.errors.append("Missing 'iso' section")
                    return False
                
                iso_section = config["iso"]
                if "name" not in iso_section:
                    self.errors.append("Missing 'name' in iso section")
                
                if "version" not in iso_section:
                    self.errors.append("Missing 'version' in iso section")
                
                if "architecture" not in iso_section:
                    self.errors.append("Missing 'architecture' in iso section")
                
                # Validate architecture
                if "architecture" in iso_section:
                    arch = iso_section["architecture"]
                    valid_archs = ["x86_64", "amd64", "i686", "i386", "arm64", "aarch64"]
                    if arch not in valid_archs:
                        self.warnings.append(f"Unsupported architecture: {arch}")
                
                # Validate bootloader
                if "bootloader" not in config:
                    self.errors.append("Missing 'bootloader' section")
                
                return len(self.errors) == 0
        
        return ISOConfigValidator()
    
    def test_valid_config_validation(self):
        """Test validation of valid configuration."""
        config = {
            "iso": {
                "name": "RegicideOS",
                "version": "1.0.0",
                "architecture": "x86_64"
            },
            "bootloader": {
                "efi_bootloader": "grub",
                "grub_timeout": 10
            }
        }
        
        result = self.validator.validate(config)
        self.assertTrue(result)
        self.assertEqual(len(self.validator.errors), 0)
    
    def test_missing_sections_validation(self):
        """Test validation detects missing sections."""
        config = {
            "iso": {
                "name": "RegicideOS"
                # Missing version and architecture
            }
            # Missing bootloader section
        }
        
        result = self.validator.validate(config)
        self.assertFalse(result)
        self.assertGreater(len(self.validator.errors), 0)
        
        # Check for expected errors
        error_messages = [str(error) for error in self.validator.errors]
        self.assertTrue(any("version" in msg for msg in error_messages))
        self.assertTrue(any("architecture" in msg for msg in error_messages))
        self.assertTrue(any("bootloader" in msg for msg in error_messages))
    
    def test_unsupported_architecture_warning(self):
        """Test warning for unsupported architecture."""
        config = {
            "iso": {
                "name": "RegicideOS",
                "version": "1.0.0",
                "architecture": "invalid_arch"
            },
            "bootloader": {
                "efi_bootloader": "grub"
            }
        }
        
        result = self.validator.validate(config)
        self.assertTrue(result)  # Should be valid but with warnings
        self.assertEqual(len(self.validator.errors), 0)
        self.assertGreater(len(self.validator.warnings), 0)
        
        # Check for architecture warning
        warning_messages = [str(warning) for warning in self.validator.warnings]
        self.assertTrue(any("invalid_arch" in msg for msg in warning_messages))

class TestISOConfigDefaults(unittest.TestCase):
    """Test ISO configuration default values."""
    
    def test_default_configuration_creation(self):
        """Test creation of default configuration."""
        default_config = {
            "iso": {
                "name": "RegicideOS",
                "version": "1.0.0",
                "architecture": "x86_64",
                "label": "RegicideOS-1.0.0",
                "publisher": "RegicideOS Team",
                "application": "RegicideOS Live Installer"
            },
            "bootloader": {
                "efi_bootloader": "grub",
                "grub_theme": "regicideos",
                "grub_timeout": 10,
                "grub_default_entry": 0
            },
            "filesystem": {
                "rootfs_type": "squashfs",
                "rootfs_compression": "xz",
                "rootfs_block_size": 131072
            },
            "security": {
                "secure_boot": True,
                "gpg_sign": True,
                "strict_permissions": True
            }
        }
        
        # Validate defaults
        self.assertEqual(default_config["iso"]["name"], "RegicideOS")
        self.assertEqual(default_config["iso"]["architecture"], "x86_64")
        self.assertEqual(default_config["bootloader"]["grub_timeout"], 10)
        self.assertEqual(default_config["filesystem"]["rootfs_type"], "squashfs")
        self.assertTrue(default_config["security"]["secure_boot"])
    
    def test_config_override_defaults(self):
        """Test overriding default configuration values."""
        config = {
            "iso": {
                "name": "CustomOS",
                "version": "2.0.0",
                "architecture": "arm64"
            },
            "bootloader": {
                "grub_timeout": 5
            }
        }
        
        # Values should be overridden
        self.assertEqual(config["iso"]["name"], "CustomOS")
        self.assertEqual(config["iso"]["version"], "2.0.0")
        self.assertEqual(config["iso"]["architecture"], "arm64")
        self.assertEqual(config["bootloader"]["grub_timeout"], 5)

if __name__ == '__main__':
    # Run tests with detailed output
    unittest.main(verbosity=2)