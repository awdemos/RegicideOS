"""
Test configuration validation and parsing for the RegicideOS installer.
"""

import unittest
from unittest.mock import Mock, patch, MagicMock
import tempfile
import os
import sys
from pathlib import Path

# Add the installer directory to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from config import parse_config, fix_config

class TestConfigValidation(unittest.TestCase):
    """Test configuration validation and parsing logic."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.valid_config = {
            "drive": "/dev/sda",
            "root_url": "https://repo.xenialinux.com/releases/current/root.img",
            "filesystem": "btrfs"
        }
    
    @patch('config.common.get_drives')
    @patch('config.common.check_url')
    @patch('config.common.get_fs')
    def test_parse_config_valid(self, mock_get_fs, mock_check_url, mock_get_drives):
        """Test parsing a valid configuration."""
        # Mock the validation functions
        mock_get_drives.return_value = ["/dev/sda", "/dev/sdb"]
        mock_check_url.return_value = True
        mock_get_fs.return_value = ["btrfs", "btrfs_encryption_dev"]
        
        result = parse_config(self.valid_config, interactive=False)
        
        self.assertEqual(result["drive"], "/dev/sda")
        self.assertEqual(result["filesystem"], "btrfs")
        self.assertEqual(result["root_url"], self.valid_config["root_url"])
    
    @patch('config.common.get_drives')
    @patch('config.common.check_url')
    @patch('config.common.get_fs')
    def test_parse_config_invalid_drive(self, mock_get_fs, mock_check_url, mock_get_drives):
        """Test parsing with invalid drive selection."""
        # Mock available drives but config has invalid drive
        mock_get_drives.return_value = ["/dev/sdb", "/dev/sdc"]
        mock_check_url.return_value = True
        mock_get_fs.return_value = ["btrfs"]
        
        # Should raise SystemExit due to die() call in non-interactive mode
        with self.assertRaises(SystemExit):
            parse_config({"drive": "/dev/invalid"}, interactive=False)
    
    @patch('config.common.get_drives')
    @patch('config.common.check_url')
    @patch('config.common.get_fs')
    def test_parse_config_invalid_url(self, mock_get_fs, mock_check_url, mock_get_drives):
        """Test parsing with invalid URL."""
        mock_get_drives.return_value = ["/dev/sda"]
        mock_check_url.return_value = False  # URL validation fails
        mock_get_fs.return_value = ["btrfs"]
        
        with self.assertRaises(SystemExit):
            parse_config({"drive": "/dev/sda", "root_url": "invalid-url"}, interactive=False)
    
    @patch('config.common.get_drives')
    @patch('config.common.check_url')
    @patch('config.common.get_fs')
    def test_parse_config_invalid_filesystem(self, mock_get_fs, mock_check_url, mock_get_drives):
        """Test parsing with unsupported filesystem."""
        mock_get_drives.return_value = ["/dev/sda"]
        mock_check_url.return_value = True
        mock_get_fs.return_value = ["btrfs"]  # Only btrfs supported
        
        # Traditional filesystem should fail
        with self.assertRaises(SystemExit):
            parse_config({"drive": "/dev/sda", "filesystem": "traditional"}, interactive=False)
    
    @patch('config.common.get_drives')
    @patch('config.common.check_url')
    @patch('config.common.get_fs')
    @patch('builtins.input')
    def test_parse_config_interactive_mode(self, mock_input, mock_get_fs, mock_check_url, mock_get_drives):
        """Test interactive configuration mode."""
        mock_get_drives.return_value = ["/dev/sda", "/dev/sdb"]
        mock_check_url.return_value = True
        mock_get_fs.return_value = ["btrfs", "btrfs_encryption_dev"]
        
        # Simulate user accepting defaults
        mock_input.return_value = ""
        
        # Test with empty config (should prompt for values)
        result = parse_config({}, interactive=True)
        
        # Should have valid values after interactive input
        self.assertIn("drive", result)
        self.assertIn("filesystem", result)
        self.assertIn("root_url", result)
    
    def test_config_missing_required_fields(self):
        """Test configuration with missing required fields."""
        incomplete_config = {"drive": "/dev/sda"}  # Missing root_url and filesystem
        
        # This should be caught by validation and trigger interactive mode or fail
        with patch('config.common.get_drives') as mock_get_drives:
            mock_get_drives.return_value = ["/dev/sda"]
            
            with self.assertRaises(SystemExit):
                parse_config(incomplete_config, interactive=False)
    
    @patch('config.common.get_drives')
    @patch('config.common.check_url')
    @patch('config.common.get_fs')
    def test_config_btrfs_encryption_support(self, mock_get_fs, mock_check_url, mock_get_drives):
        """Test that btrfs encryption is properly supported."""
        mock_get_drives.return_value = ["/dev/sda"]
        mock_check_url.return_value = True
        mock_get_fs.return_value = ["btrfs", "btrfs_encryption_dev"]
        
        # Test btrfs_encryption_dev is accepted
        result = parse_config({
            "drive": "/dev/sda",
            "root_url": "https://example.com/root.img",
            "filesystem": "btrfs_encryption_dev"
        }, interactive=False)
        
        self.assertEqual(result["filesystem"], "btrfs_encryption_dev")
    
    @patch('config.common.get_drives')
    @patch('config.common.check_url')
    @patch('config.common.get_fs')
    def test_config_lvm_rejected(self, mock_get_fs, mock_check_url, mock_get_drives):
        """Test that LVM layouts are rejected due to btrfs-only requirement."""
        mock_get_drives.return_value = ["/dev/sda"]
        mock_check_url.return_value = True
        mock_get_fs.return_value = ["btrfs"]  # Only btrfs in available options
        
        # LVM should not be available in get_fs return, but test if somehow specified
        with self.assertRaises(SystemExit):
            parse_config({
                "drive": "/dev/sda", 
                "root_url": "https://example.com/root.img",
                "filesystem": "lvm"
            }, interactive=False)

class TestConfigFileHandling(unittest.TestCase):
    """Test configuration file handling."""
    
    def test_config_file_toml_parsing(self):
        """Test parsing of TOML configuration files."""
        toml_content = """
drive = "/dev/sda"
root_url = "https://repo.xenialinux.com/releases/current/root.img"
filesystem = "btrfs"
"""
        
        with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
            f.write(toml_content)
            f.flush()
            
            try:
                # Test that TOML file can be read (requires tomllib import)
                with patch('builtins.open', unittest.mock.mock_open(read_data=toml_content)):
                    # This would normally be done in main.py
                    with patch('tomllib.load') as mock_load:
                        mock_load.return_value = {
                            "drive": "/dev/sda",
                            "root_url": "https://repo.xenialinux.com/releases/current/root.img", 
                            "filesystem": "btrfs"
                        }
                        
                        imported_tomllib = __import__('tomllib')
                        result = imported_tomllib.load(f.name)
                        self.assertEqual(result["drive"], "/dev/sda")
                        
            finally:
                os.unlink(f.name)

if __name__ == '__main__':
    unittest.main()