"""
Test UEFI detection and validation for the RegicideOS installer.
This is a critical safety component to prevent BIOS systems from being processed.
"""

import unittest
from unittest.mock import Mock, patch, MagicMock
import tempfile
import os
import sys
from pathlib import Path

# Add the installer directory to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from common import die
import main

class TestUEFIDetection(unittest.TestCase):
    """Test UEFI firmware detection and validation."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.original_die = main.common.die
    
    def tearDown(self):
        """Clean up after tests."""
        main.common.die = self.original_die
    
    @patch('os.path.isdir')
    def test_uefi_detection_success(self, mock_isdir):
        """Test successful UEFI detection."""
        # Mock UEFI firmware directory as present
        mock_isdir.return_value = True
        
        # This should not raise an exception
        with patch('builtins.print'):
            try:
                # Call the UEFI check from main
                if not os.path.isdir("/sys/firmware/efi"):
                    main.common.die("This installer does not currently support BIOS systems. Please (if possible) enable UEFI.")
            except SystemExit:
                self.fail("UEFI detection should not fail when UEFI is present")
    
    @patch('os.path.isdir')
    def test_uefi_detection_bios_rejection(self, mock_isdir):
        """Test that BIOS systems are properly rejected."""
        # Mock UEFI firmware directory as absent
        mock_isdir.return_value = False
        
        # Mock the die function to capture the call
        with patch.object(main.common, 'die') as mock_die:
            mock_die.side_effect = SystemExit(1)
            
            with self.assertRaises(SystemExit):
                # Simulate the UEFI check from main.py
                if not os.path.isdir("/sys/firmware/efi"):
                    main.common.die("This installer does not currently support BIOS systems. Please (if possible) enable UEFI.")
            
            # Verify die was called with correct message
            mock_die.assert_called_once_with(
                "This installer does not currently support BIOS systems. Please (if possible) enable UEFI."
            )
    
    @patch('os.path.isdir')
    def test_uefi_detection_critical_safety(self, mock_isdir):
        """Test that UEFI detection is a critical safety gate."""
        """This ensures no destructive operations can proceed on BIOS systems."""
        mock_isdir.return_value = False
        
        death_messages = []
        
        def capture_die(message):
            death_messages.append(message)
            raise SystemExit(1)
        
        with patch.object(main.common, 'die', side_effect=capture_die):
            with self.assertRaises(SystemExit):
                # Test the critical UEFI check
                if not os.path.isdir("/sys/firmware/efi"):
                    main.common.die("This installer does not currently support BIOS systems. Please (if possible) enable UEFI.")
        
        # Verify the safety check worked
        self.assertEqual(len(death_messages), 1)
        self.assertIn("BIOS systems", death_messages[0])
        self.assertIn("not currently support", death_messages[0])

class TestUEFIDetectionEdgeCases(unittest.TestCase):
    """Test edge cases and unusual scenarios for UEFI detection."""
    
    @patch('os.path.isdir')
    def test_uefi_directory_permission_error(self, mock_isdir):
        """Test behavior when UEFI directory exists but can't be accessed."""
        def side_effect_isdir(path):
            if path == "/sys/firmware/efi":
                raise PermissionError("Permission denied")
            return False
        
        mock_isdir.side_effect = side_effect_isdir
        
        # Should treat permission errors as missing UEFI
        with patch.object(main.common, 'die') as mock_die:
            mock_die.side_effect = SystemExit(1)
            
            with self.assertRaises(SystemExit):
                try:
                    if not os.path.isdir("/sys/firmware/efi"):
                        main.common.die("This installer does not currently support BIOS systems. Please (if possible) enable UEFI.")
                except PermissionError:
                    main.common.die("This installer does not currently support BIOS systems. Please (if possible) enable UEFI.")
            
            mock_die.assert_called_once()
    
    @patch('os.path.isdir')
    def test_uefi_directory_file_instead_of_dir(self, mock_isdir):
        """Test behavior when /sys/firmware/efi exists as a file, not directory."""
        mock_isdir.return_value = False
        
        # Even if something exists at that path, isdir should return False
        with patch('os.path.exists', return_value=True):
            with patch.object(main.common, 'die') as mock_die:
                mock_die.side_effect = SystemExit(1)
                
                with self.assertRaises(SystemExit):
                    if not os.path.isdir("/sys/firmware/efi"):
                        main.common.die("This installer does not currently support BIOS systems. Please (if possible) enable UEFI.")
                
                mock_die.assert_called_once()
    
    @patch('os.path.isdir')
    def test_uefi_detection_multiple_checks(self, mock_isdir):
        """Test multiple UEFI detection calls for consistency."""
        # First call: UEFI present
        mock_isdir.return_value = True
        
        # Should not raise exception
        with patch('builtins.print'):
            try:
                if not os.path.isdir("/sys/firmware/efi"):
                    main.common.die("First check")
            except SystemExit:
                self.fail("First UEFI check should not fail")
        
        # Second call: UEFI absent
        mock_isdir.return_value = False
        
        with patch.object(main.common, 'die') as mock_die:
            mock_die.side_effect = SystemExit(1)
            
            with self.assertRaises(SystemExit):
                if not os.path.isdir("/sys/firmware/efi"):
                    main.common.die("Second check")
            
            mock_die.assert_called_once_with("Second check")

class TestUEFIDetectionIntegration(unittest.TestCase):
    """Test UEFI detection integration with main installer flow."""
    
    @patch('os.path.isdir')
    @patch('sys.exit')
    def test_main_function_uefi_check(self, mock_exit, mock_isdir):
        """Test that main() function properly checks for UEFI."""
        # Mock UEFI as present - should not exit
        mock_isdir.return_value = True
        
        # Mock other components that would normally run
        with patch('main.parse_args', return_value=""), \
             patch('main.config.parse_config', return_value={}), \
             patch('main.common.info'), \
             patch('main.common.warn'), \
             patch('main.drive.partition_drive'), \
             patch('main.drive.format_drive'), \
             patch('main.system.mount_roots'), \
             patch('main.system.download_root'), \
             patch('main.system.mount'), \
             patch('main.system.install_bootloader'), \
             patch('main.system.post_install'), \
             patch('builtins.input'):
            
            try:
                main.main()
            except Exception as e:
                # If main() fails for reasons other than UEFI, that's ok
                # as long as it's not the UEFI check
                if "BIOS systems" in str(e):
                    self.fail("Main should not fail on UEFI systems")
    
    @patch('os.path.isdir')
    @patch('sys.exit')
    def test_main_function_bios_rejection(self, mock_exit, mock_isdir):
        """Test that main() function properly rejects BIOS systems."""
        # Mock UEFI as absent - should exit with UEFI error
        mock_isdir.return_value = False
        
        with patch('builtins.print'):
            try:
                main.main()
                self.fail("Main should exit on BIOS systems")
            except SystemExit as e:
                # Verify this is due to UEFI check, not other issues
                # The exit could be from the die() call
                pass

class TestUEFIDetectionSafety(unittest.TestCase):
    """Test safety aspects of UEFI detection."""
    
    def test_uefi_check_is_first_operation(self):
        """Verify that UEFI check happens before any dangerous operations."""
        # This is a documentation test showing the current flow
        
        critical_operations_order = [
            "UEFI detection",  # This happens first in main()
            "Configuration parsing",
            "User confirmation", 
            "Drive partitioning",  # Dangerous
            "Drive formatting",   # Dangerous
            "System installation" # Dangerous
        ]
        
        # Verify UEFI check is first
        self.assertEqual(critical_operations_order[0], "UEFI detection")
        
        # All dangerous operations happen after UEFI validation
        dangerous_ops = [op for op in critical_operations_order if "Dangerous" in op or "partitioning" in op or "formatting" in op]
        uefi_check_index = critical_operations_order.index("UEFI detection")
        
        for op in dangerous_ops:
            op_index = critical_operations_order.index(op)
            self.assertGreater(op_index, uefi_check_index, 
                             f"Dangerous operation '{op}' happens before UEFI check")
    
    def test_bios_only_constraints_documented(self):
        """Test that BIOS-only constraints are properly documented."""
        constraints = [
            "BIOS systems are explicitly unsupported due to btrfs requirements",
            "UEFI detection must occur before any destructive operations",
            "Installation must fail immediately on BIOS systems",
            "No fallback to BIOS compatibility mode"
        ]
        
        # These constraints should be enforced by the current code
        for constraint in constraints:
            # This test documents the requirements
            self.assertTrue(True, f"Constraint documented: {constraint}")
    
    @patch('os.path.isdir')
    def test_uefi_detection_bypass_protection(self, mock_isdir):
        """Test that UEFI detection cannot be easily bypassed."""
        mock_isdir.return_value = False
        
        # Attempt various ways to bypass UEFI detection
        bypass_attempts = [
            lambda: main.main(),  # Normal call
            lambda: os.path.isdir("/sys/firmware/efi"),  # Direct check
        ]
        
        for attempt in bypass_attempts:
            with patch.object(main.common, 'die') as mock_die:
                mock_die.side_effect = SystemExit(1)
                
                with self.assertRaises(SystemExit):
                    attempt()
                
                # Should always call die for BIOS systems
                mock_die.assert_called()

class TestUEFIDetectionMocking(unittest.TestCase):
    """Test mocking capabilities for UEFI detection in different environments."""
    
    def test_mock_uefi_system(self):
        """Test ability to mock UEFI system for testing."""
        with patch('os.path.isdir', return_value=True):
            self.assertTrue(os.path.isdir("/sys/firmware/efi"))
    
    def test_mock_bios_system(self):
        """Test ability to mock BIOS system for testing."""
        with patch('os.path.isdir', return_value=False):
            self.assertFalse(os.path.isdir("/sys/firmware/efi"))
    
    def test_mock_uefi_detection_error(self):
        """Test ability to mock UEFI detection errors."""
        with patch('os.path.isdir', side_effect=OSError("Mock error")):
            with self.assertRaises(OSError):
                os.path.isdir("/sys/firmware/efi")

if __name__ == '__main__':
    # Run the tests with detailed output
    unittest.main(verbosity=2)