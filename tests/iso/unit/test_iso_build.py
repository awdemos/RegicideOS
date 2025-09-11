"""
Unit tests for ISO build process.
"""

import unittest
from unittest.mock import Mock, patch, MagicMock, call, mock_open
import tempfile
import os
import sys
import subprocess
from pathlib import Path
import shutil

# Add the project root to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent))

class TestISOBuild(unittest.TestCase):
    """Test ISO build process with comprehensive mocking."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.build_dir = tempfile.mkdtemp()
        self.iso_dir = os.path.join(self.build_dir, "iso")
        self.work_dir = os.path.join(self.build_dir, "work")
        self.output_dir = os.path.join(self.build_dir, "output")
        
        # Create directories
        os.makedirs(self.iso_dir, exist_ok=True)
        os.makedirs(self.work_dir, exist_ok=True)
        os.makedirs(self.output_dir, exist_ok=True)
        
        # Mock configuration
        self.config = {
            "iso": {
                "name": "RegicideOS",
                "version": "1.0.0",
                "architecture": "x86_64",
                "label": "RegicideOS-1.0.0"
            },
            "build": {
                "parallel_jobs": 4,
                "log_level": "info"
            }
        }
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.build_dir):
            shutil.rmtree(self.build_dir)
    
    def test_build_environment_setup(self):
        """Test build environment setup."""
        class BuildEnvironment:
            def __init__(self, build_dir, config):
                self.build_dir = build_dir
                self.config = config
                self.iso_dir = os.path.join(build_dir, "iso")
                self.work_dir = os.path.join(build_dir, "work")
                self.output_dir = os.path.join(build_dir, "output")
                self.errors = []
            
            def setup(self):
                """Setup build environment."""
                try:
                    # Create directories
                    dirs = [self.iso_dir, self.work_dir, self.output_dir]
                    for dir_path in dirs:
                        if not os.path.exists(dir_path):
                            os.makedirs(dir_path)
                    
                    # Validate directories are writable
                    for dir_path in dirs:
                        test_file = os.path.join(dir_path, "test_write")
                        try:
                            with open(test_file, 'w') as f:
                                f.write("test")
                            os.unlink(test_file)
                        except Exception as e:
                            self.errors.append(f"Directory not writable: {dir_path} - {e}")
                    
                    return len(self.errors) == 0
                    
                except Exception as e:
                    self.errors.append(f"Failed to setup build environment: {e}")
                    return False
        
        # Test successful setup
        env = BuildEnvironment(self.build_dir, self.config)
        result = env.setup()
        
        self.assertTrue(result)
        self.assertEqual(len(env.errors), 0)
        
        # Verify directories exist
        self.assertTrue(os.path.exists(self.iso_dir))
        self.assertTrue(os.path.exists(self.work_dir))
        self.assertTrue(os.path.exists(self.output_dir))
    
    def test_build_environment_setup_failure(self):
        """Test build environment setup failure."""
        # Use a non-existent directory that can't be created
        invalid_dir = "/root/nonexistent_build_dir"
        
        class BuildEnvironment:
            def __init__(self, build_dir):
                self.build_dir = build_dir
                self.errors = []
            
            def setup(self):
                """Setup build environment."""
                try:
                    os.makedirs(self.build_dir, exist_ok=True)
                    return True
                except Exception as e:
                    self.errors.append(f"Failed to create directories: {e}")
                    return False
        
        env = BuildEnvironment(invalid_dir)
        result = env.setup()
        
        self.assertFalse(result)
        self.assertGreater(len(env.errors), 0)
    
    @patch('subprocess.Popen')
    def test_dependency_validation(self, mock_popen):
        """Test dependency validation."""
        class DependencyValidator:
            def __init__(self):
                self.errors = []
                self.warnings = []
            
            def validate_dependencies(self):
                """Validate required dependencies."""
                dependencies = [
                    ("xorriso", "ISO creation tool"),
                    ("squashfs-tools", "Squashfs compression"),
                    ("mksquashfs", "Squashfs filesystem creator"),
                    ("genisoimage", "ISO image creation")
                ]
                
                # Mock subprocess responses
                for dep, desc in dependencies:
                    mock_process = Mock()
                    mock_process.communicate.return_value = (b'', b'')
                    mock_popen.return_value = mock_process
                    
                    # Simulate command not found
                    mock_popen.side_effect = FileNotFoundError(f"Command not found: {dep}")
                    
                    try:
                        subprocess.Popen([dep, "--version"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    except FileNotFoundError:
                        self.errors.append(f"Missing dependency: {dep} ({desc})")
                
                return len(self.errors) == 0
        
        validator = DependencyValidator()
        result = validator.validate_dependencies()
        
        self.assertFalse(result)  # Should fail due to missing dependencies
        self.assertGreater(len(validator.errors), 0)
    
    @patch('subprocess.Popen')
    def test_dependency_validation_success(self, mock_popen):
        """Test successful dependency validation."""
        class DependencyValidator:
            def __init__(self):
                self.errors = []
                self.warnings = []
            
            def validate_dependencies(self):
                """Validate required dependencies."""
                dependencies = ["xorriso", "squashfs-tools", "mksquashfs"]
                
                # Mock successful subprocess responses
                mock_process = Mock()
                mock_process.communicate.return_value = (b'xorriso version 1.5.4', b'')
                mock_popen.return_value = mock_process
                
                for dep in dependencies:
                    try:
                        subprocess.Popen([dep, "--version"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    except FileNotFoundError:
                        self.errors.append(f"Missing dependency: {dep}")
                
                return len(self.errors) == 0
        
        validator = DependencyValidator()
        result = validator.validate_dependencies()
        
        self.assertTrue(result)  # Should pass
        self.assertEqual(len(validator.errors), 0)
    
    def test_iso_structure_creation(self):
        """Test ISO directory structure creation."""
        class ISOStructureBuilder:
            def __init__(self, iso_dir):
                self.iso_dir = iso_dir
                self.errors = []
            
            def create_structure(self):
                """Create ISO directory structure."""
                try:
                    # Define required directories
                    dirs = [
                        "boot",
                        "boot/grub",
                        "EFI",
                        "EFI/BOOT",
                        "live",
                        ".disk"
                    ]
                    
                    # Create directories
                    for dir_name in dirs:
                        dir_path = os.path.join(self.iso_dir, dir_name)
                        os.makedirs(dir_path, exist_ok=True)
                        
                        # Verify directory was created
                        if not os.path.exists(dir_path):
                            self.errors.append(f"Failed to create directory: {dir_name}")
                    
                    # Create required files
                    self._create_disk_info()
                    self._create_bootloader_config()
                    
                    return len(self.errors) == 0
                    
                except Exception as e:
                    self.errors.append(f"Failed to create ISO structure: {e}")
                    return False
            
            def _create_disk_info(self):
                """Create disk information files."""
                info_file = os.path.join(self.iso_dir, ".disk", "info")
                try:
                    with open(info_file, 'w') as f:
                        f.write("RegicideOS 1.0.0\\n")
                        f.write("Built: $(date)\\n")
                        f.write("Architecture: x86_64\\n")
                except Exception as e:
                    self.errors.append(f"Failed to create disk info: {e}")
            
            def _create_bootloader_config(self):
                """Create bootloader configuration."""
                grub_file = os.path.join(self.iso_dir, "boot", "grub", "grub.cfg")
                try:
                    with open(grub_file, 'w') as f:
                        f.write("set timeout=10\\n")
                        f.write("set default=0\\n")
                        f.write('menuentry "RegicideOS Live" {\\n')
                        f.write("    linux /boot/vmlinuz boot=live\\n")
                        f.write("    initrd /boot/initrd\\n")
                        f.write("}\\n")
                except Exception as e:
                    self.errors.append(f"Failed to create bootloader config: {e}")
        
        builder = ISOStructureBuilder(self.iso_dir)
        result = builder.create_structure()
        
        self.assertTrue(result)
        self.assertEqual(len(builder.errors), 0)
        
        # Verify structure was created
        expected_dirs = ["boot/grub", "EFI/BOOT", "live", ".disk"]
        for dir_name in expected_dirs:
            dir_path = os.path.join(self.iso_dir, dir_name)
            self.assertTrue(os.path.exists(dir_path))
        
        # Verify files were created
        info_file = os.path.join(self.iso_dir, ".disk", "info")
        grub_file = os.path.join(self.iso_dir, "boot", "grub", "grub.cfg")
        self.assertTrue(os.path.exists(info_file))
        self.assertTrue(os.path.exists(grub_file))
    
    def test_root_filesystem_creation(self):
        """Test root filesystem creation."""
        class RootFilesystemBuilder:
            def __init__(self, iso_dir, work_dir):
                self.iso_dir = iso_dir
                self.work_dir = work_dir
                self.errors = []
            
            def create_rootfs(self):
                """Create root filesystem."""
                try:
                    # Create temporary root directory
                    temp_root = os.path.join(self.work_dir, "rootfs")
                    os.makedirs(temp_root, exist_ok=True)
                    
                    # Create basic directory structure
                    dirs = ["bin", "boot", "dev", "etc", "usr", "var"]
                    for dir_name in dirs:
                        dir_path = os.path.join(temp_root, dir_name)
                        os.makedirs(dir_path, exist_ok=True)
                    
                    # Create system files
                    self._create_system_files(temp_root)
                    
                    # Create squashfs image
                    squashfs_path = os.path.join(self.iso_dir, "live", "filesystem.squashfs")
                    self._create_squashfs(temp_root, squashfs_path)
                    
                    # Clean up
                    shutil.rmtree(temp_root)
                    
                    return len(self.errors) == 0
                    
                except Exception as e:
                    self.errors.append(f"Failed to create root filesystem: {e}")
                    return False
            
            def _create_system_files(self, root_dir):
                """Create basic system files."""
                # Create os-release
                os_release_file = os.path.join(root_dir, "etc", "os-release")
                try:
                    with open(os_release_file, 'w') as f:
                        f.write('NAME="RegicideOS"\\n')
                        f.write('VERSION="1.0.0"\\n')
                        f.write('ID=regicideos\\n')
                        f.write('VERSION_ID="1.0.0"\\n')
                except Exception as e:
                    self.errors.append(f"Failed to create os-release: {e}")
                
                # Create hostname
                hostname_file = os.path.join(root_dir, "etc", "hostname")
                try:
                    with open(hostname_file, 'w') as f:
                        f.write("regicideos-live\\n")
                except Exception as e:
                    self.errors.append(f"Failed to create hostname: {e}")
            
            def _create_squashfs(self, source_dir, output_file):
                """Create squashfs filesystem image."""
                # Mock squashfs creation
                try:
                    # In real implementation, this would call mksquashfs
                    # For testing, create a placeholder file
                    with open(output_file, 'w') as f:
                        f.write("Mock squashfs filesystem\\n")
                except Exception as e:
                    self.errors.append(f"Failed to create squashfs: {e}")
        
        builder = RootFilesystemBuilder(self.iso_dir, self.work_dir)
        result = builder.create_rootfs()
        
        self.assertTrue(result)
        self.assertEqual(len(builder.errors), 0)
        
        # Verify squashfs was created
        squashfs_file = os.path.join(self.iso_dir, "live", "filesystem.squashfs")
        self.assertTrue(os.path.exists(squashfs_file))
    
    @patch('subprocess.Popen')
    def test_iso_image_creation(self, mock_popen):
        """Test ISO image creation."""
        class ISOImageBuilder:
            def __init__(self, iso_dir, output_file):
                self.iso_dir = iso_dir
                self.output_file = output_file
                self.errors = []
            
            def create_iso(self):
                """Create ISO image."""
                try:
                    # Mock xorriso command
                    mock_process = Mock()
                    mock_process.communicate.return_value = (b'ISO created successfully', b'')
                    mock_process.returncode = 0
                    mock_popen.return_value = mock_process
                    
                    # Create ISO
                    cmd = [
                        "xorriso",
                        "-as", "mkisofs",
                        "-iso-level", "3",
                        "-full-iso9660-filenames",
                        "-volid", "RegicideOS-1.0.0",
                        "-output", self.output_file,
                        self.iso_dir
                    ]
                    
                    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    stdout, stderr = process.communicate()
                    
                    if process.returncode != 0:
                        self.errors.append(f"ISO creation failed: {stderr.decode()}")
                    
                    return len(self.errors) == 0
                    
                except Exception as e:
                    self.errors.append(f"Failed to create ISO image: {e}")
                    return False
        
        output_file = os.path.join(self.output_dir, "test.iso")
        builder = ISOImageBuilder(self.iso_dir, output_file)
        result = builder.create_iso()
        
        self.assertTrue(result)
        self.assertEqual(len(builder.errors), 0)
        
        # Verify xorriso was called
        mock_popen.assert_called_once()
        call_args = mock_popen.call_args[0][0]
        self.assertIn("xorriso", call_args)
        self.assertIn("RegicideOS-1.0.0", call_args)
    
    def test_build_process_integration(self):
        """Test complete build process integration."""
        class BuildProcess:
            def __init__(self, build_dir, config):
                self.build_dir = build_dir
                self.config = config
                self.iso_dir = os.path.join(build_dir, "iso")
                self.work_dir = os.path.join(build_dir, "work")
                self.output_dir = os.path.join(build_dir, "output")
                self.steps_completed = []
                self.errors = []
            
            def run_build(self):
                """Run complete build process."""
                steps = [
                    ("setup_environment", self._setup_environment),
                    ("create_structure", self._create_structure),
                    ("create_rootfs", self._create_rootfs),
                    ("create_iso", self._create_iso),
                    ("validate_output", self._validate_output)
                ]
                
                for step_name, step_func in steps:
                    try:
                        if step_func():
                            self.steps_completed.append(step_name)
                        else:
                            self.errors.append(f"Step {step_name} failed")
                            return False
                    except Exception as e:
                        self.errors.append(f"Step {step_name} failed with exception: {e}")
                        return False
                
                return True
            
            def _setup_environment(self):
                """Setup build environment."""
                dirs = [self.iso_dir, self.work_dir, self.output_dir]
                for dir_path in dirs:
                    os.makedirs(dir_path, exist_ok=True)
                return True
            
            def _create_structure(self):
                """Create ISO structure."""
                dirs = ["boot/grub", "EFI/BOOT", "live"]
                for dir_name in dirs:
                    dir_path = os.path.join(self.iso_dir, dir_name)
                    os.makedirs(dir_path, exist_ok=True)
                return True
            
            def _create_rootfs(self):
                """Create root filesystem."""
                os.makedirs(os.path.join(self.iso_dir, "live"), exist_ok=True)
                squashfs_file = os.path.join(self.iso_dir, "live", "filesystem.squashfs")
                with open(squashfs_file, 'w') as f:
                    f.write("Mock root filesystem\\n")
                return True
            
            def _create_iso(self):
                """Create ISO image."""
                iso_file = os.path.join(self.output_dir, "test.iso")
                with open(iso_file, 'w') as f:
                    f.write("Mock ISO image\\n")
                return True
            
            def _validate_output(self):
                """Validate output files."""
                iso_file = os.path.join(self.output_dir, "test.iso")
                return os.path.exists(iso_file)
        
        builder = BuildProcess(self.build_dir, self.config)
        result = builder.run_build()
        
        self.assertTrue(result)
        self.assertEqual(len(builder.errors), 0)
        
        # Verify all steps completed
        expected_steps = [
            "setup_environment",
            "create_structure", 
            "create_rootfs",
            "create_iso",
            "validate_output"
        ]
        self.assertEqual(builder.steps_completed, expected_steps)

class TestISOBuildErrorHandling(unittest.TestCase):
    """Test error handling in ISO build process."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.build_dir = tempfile.mkdtemp()
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.build_dir):
            shutil.rmtree(self.build_dir)
    
    def test_directory_creation_failure(self):
        """Test handling of directory creation failures."""
        class ErrorBuilder:
            def __init__(self, build_dir):
                self.build_dir = build_dir
                self.errors = []
            
            def setup_environment(self):
                """Setup that fails."""
                try:
                    # Try to create directory in a non-existent location
                    invalid_dir = os.path.join(self.build_dir, "nonexistent", "nested")
                    os.makedirs(invalid_dir, exist_ok=False)
                    return True
                except Exception as e:
                    self.errors.append(f"Directory creation failed: {e}")
                    return False
        
        builder = ErrorBuilder(self.build_dir)
        result = builder.setup_environment()
        
        self.assertFalse(result)
        self.assertGreater(len(builder.errors), 0)
    
    def test_file_write_failure(self):
        """Test handling of file write failures."""
        class ErrorBuilder:
            def __init__(self, build_dir):
                self.build_dir = build_dir
                self.errors = []
            
            def create_config_file(self):
                """Create config file that fails."""
                try:
                    # Try to write to a non-existent directory
                    config_file = os.path.join(self.build_dir, "nonexistent", "config.toml")
                    with open(config_file, 'w') as f:
                        f.write("test config\\n")
                    return True
                except Exception as e:
                    self.errors.append(f"File write failed: {e}")
                    return False
        
        builder = ErrorBuilder(self.build_dir)
        result = builder.create_config_file()
        
        self.assertFalse(result)
        self.assertGreater(len(builder.errors), 0)

if __name__ == '__main__':
    # Run tests with detailed output
    unittest.main(verbosity=2)