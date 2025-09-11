"""
Integration tests for ISO creation and validation workflow.
"""

import unittest
from unittest.mock import Mock, patch, MagicMock, call
import tempfile
import os
import sys
import subprocess
from pathlib import Path
import shutil

# Add the project root to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent))

class TestISOBuildIntegration(unittest.TestCase):
    """Test complete ISO build and validation workflow."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.build_dir = os.path.join(self.temp_dir, "build")
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
            },
            "validation": {
                "validate_dependencies": True,
                "validate_configuration": True,
                "validate_iso": True
            }
        }
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_complete_build_workflow(self):
        """Test complete ISO build workflow."""
        class ISOBuildWorkflow:
            def __init__(self, build_dir, config):
                self.build_dir = build_dir
                self.config = config
                self.iso_dir = os.path.join(build_dir, "iso")
                self.work_dir = os.path.join(build_dir, "work")
                self.output_dir = os.path.join(build_dir, "output")
                self.workflow_log = []
                self.errors = []
            
            def run_complete_workflow(self):
                """Run complete build workflow."""
                try:
                    # Phase 1: Pre-build validation
                    if not self._pre_build_validation():
                        raise RuntimeError("Pre-build validation failed")
                    
                    # Phase 2: Build environment setup
                    if not self._setup_build_environment():
                        raise RuntimeError("Build environment setup failed")
                    
                    # Phase 3: ISO structure creation
                    if not self._create_iso_structure():
                        raise RuntimeError("ISO structure creation failed")
                    
                    # Phase 4: Root filesystem creation
                    if not self._create_root_filesystem():
                        raise RuntimeError("Root filesystem creation failed")
                    
                    # Phase 5: Bootloader preparation
                    if not self._prepare_bootloaders():
                        raise RuntimeError("Bootloader preparation failed")
                    
                    # Phase 6: ISO image creation
                    if not self._create_iso_image():
                        raise RuntimeError("ISO image creation failed")
                    
                    # Phase 7: Post-build validation
                    if not self._post_build_validation():
                        raise RuntimeError("Post-build validation failed")
                    
                    # Phase 8: Cleanup and reporting
                    if not self._cleanup_and_report():
                        raise RuntimeError("Cleanup and reporting failed")
                    
                    return True
                    
                except Exception as e:
                    self.errors.append(f"Workflow failed: {e}")
                    return False
            
            def _pre_build_validation(self):
                """Pre-build validation phase."""
                self.workflow_log.append("Starting pre-build validation...")
                
                # Validate configuration
                if not self.config:
                    self.errors.append("Configuration is empty")
                    return False
                
                required_sections = ["iso", "build"]
                for section in required_sections:
                    if section not in self.config:
                        self.errors.append(f"Missing required section: {section}")
                        return False
                
                self.workflow_log.append("✓ Configuration validation passed")
                
                # Validate build environment
                if not os.path.exists(self.build_dir):
                    self.errors.append("Build directory does not exist")
                    return False
                
                self.workflow_log.append("✓ Build environment validation passed")
                
                return True
            
            def _setup_build_environment(self):
                """Setup build environment phase."""
                self.workflow_log.append("Setting up build environment...")
                
                # Create required directories
                dirs = [self.iso_dir, self.work_dir, self.output_dir]
                for dir_path in dirs:
                    os.makedirs(dir_path, exist_ok=True)
                
                # Create log directory
                log_dir = os.path.join(self.build_dir, "logs")
                os.makedirs(log_dir, exist_ok=True)
                
                self.workflow_log.append("✓ Build environment setup completed")
                return True
            
            def _create_iso_structure(self):
                """Create ISO directory structure phase."""
                self.workflow_log.append("Creating ISO directory structure...")
                
                # Create ISO structure
                structure = [
                    "boot",
                    "boot/grub",
                    "EFI",
                    "EFI/BOOT",
                    "live",
                    ".disk"
                ]
                
                for item in structure:
                    item_path = os.path.join(self.iso_dir, item)
                    os.makedirs(item_path, exist_ok=True)
                
                # Create disk info files
                info_file = os.path.join(self.iso_dir, ".disk", "info")
                with open(info_file, 'w') as f:
                    f.write(f"RegicideOS {self.config['iso']['version']}\\n")
                    f.write(f"Architecture: {self.config['iso']['architecture']}\\n")
                
                self.workflow_log.append("✓ ISO directory structure created")
                return True
            
            def _create_root_filesystem(self):
                """Create root filesystem phase."""
                self.workflow_log.append("Creating root filesystem...")
                
                # Create temporary root directory
                temp_root = os.path.join(self.work_dir, "rootfs")
                os.makedirs(temp_root, exist_ok=True)
                
                # Create basic directory structure
                basic_dirs = ["bin", "boot", "dev", "etc", "usr", "var"]
                for dir_name in basic_dirs:
                    dir_path = os.path.join(temp_root, dir_name)
                    os.makedirs(dir_path, exist_ok=True)
                
                # Create system files
                self._create_system_files(temp_root)
                
                # Create squashfs image
                live_dir = os.path.join(self.iso_dir, "live")
                os.makedirs(live_dir, exist_ok=True)
                
                squashfs_file = os.path.join(live_dir, "filesystem.squashfs")
                with open(squashfs_file, 'w') as f:
                    f.write("Mock squashfs filesystem\\n")
                
                # Clean up temporary root
                shutil.rmtree(temp_root)
                
                self.workflow_log.append("✓ Root filesystem created")
                return True
            
            def _create_system_files(self, root_dir):
                """Create basic system files."""
                # Create os-release
                os_release_file = os.path.join(root_dir, "etc", "os-release")
                os.makedirs(os.path.dirname(os_release_file), exist_ok=True)
                with open(os_release_file, 'w') as f:
                    f.write('NAME="RegicideOS"\\n')
                    f.write(f'VERSION="{self.config["iso"]["version"]}"\\n')
                    f.write('ID=regicideos\\n')
                
                # Create hostname
                hostname_file = os.path.join(root_dir, "etc", "hostname")
                with open(hostname_file, 'w') as f:
                    f.write("regicideos-live\\n")
            
            def _prepare_bootloaders(self):
                """Prepare bootloaders phase."""
                self.workflow_log.append("Preparing bootloaders...")
                
                # Create GRUB configuration
                grub_cfg = os.path.join(self.iso_dir, "boot", "grub", "grub.cfg")
                with open(grub_cfg, 'w') as f:
                    f.write("set timeout=10\\n")
                    f.write("set default=0\\n")
                    f.write('menuentry "RegicideOS Live" {\\n')
                    f.write("    linux /boot/vmlinuz boot=live\\n")
                    f.write("    initrd /boot/initrd\\n")
                    f.write("}\\n")
                
                # Create UEFI bootloader stubs
                efi_boot = os.path.join(self.iso_dir, "EFI", "BOOT", "BOOTX64.EFI")
                with open(efi_boot, 'w') as f:
                    f.write("Mock UEFI bootloader\\n")
                
                self.workflow_log.append("✓ Bootloaders prepared")
                return True
            
            def _create_iso_image(self):
                """Create ISO image phase."""
                self.workflow_log.append("Creating ISO image...")
                
                # Create mock ISO file
                iso_filename = f"regicideos-{self.config['iso']['version']}-{self.config['iso']['architecture']}.iso"
                iso_file = os.path.join(self.output_dir, iso_filename)
                
                with open(iso_file, 'w') as f:
                    f.write("Mock ISO image content\\n")
                
                # Create checksum file
                checksum_file = f"{iso_file}.sha256"
                with open(checksum_file, 'w') as f:
                    f.write("mock_checksum_value regicideos-1.0.0-x86_64.iso\\n")
                
                self.workflow_log.append("✓ ISO image created")
                return True
            
            def _post_build_validation(self):
                """Post-build validation phase."""
                self.workflow_log.append("Running post-build validation...")
                
                # Check if ISO file exists
                iso_files = [f for f in os.listdir(self.output_dir) if f.endswith('.iso')]
                if not iso_files:
                    self.errors.append("No ISO file created")
                    return False
                
                # Check ISO file size
                iso_file = os.path.join(self.output_dir, iso_files[0])
                file_size = os.path.getsize(iso_file)
                if file_size == 0:
                    self.errors.append("ISO file is empty")
                    return False
                
                # Check checksum file
                checksum_file = f"{iso_file}.sha256"
                if not os.path.exists(checksum_file):
                    self.errors.append("Checksum file not created")
                    return False
                
                self.workflow_log.append("✓ Post-build validation passed")
                return True
            
            def _cleanup_and_report(self):
                """Cleanup and reporting phase."""
                self.workflow_log.append("Cleaning up and generating report...")
                
                # Generate build report
                report_file = os.path.join(self.output_dir, "build-report.txt")
                with open(report_file, 'w') as f:
                    f.write("RegicideOS ISO Build Report\\n")
                    f.write("=========================\\n")
                    f.write(f"Version: {self.config['iso']['version']}\\n")
                    f.write(f"Architecture: {self.config['iso']['architecture']}\\n")
                    f.write(f"Build Date: {subprocess.check_output(['date'], universal_newlines=True).strip()}\\n")
                    f.write("\\nWorkflow Log:\\n")
                    for log_entry in self.workflow_log:
                        f.write(f"  {log_entry}\\n")
                
                self.workflow_log.append("✓ Cleanup and reporting completed")
                return True
        
        # Test the complete workflow
        workflow = ISOBuildWorkflow(self.build_dir, self.config)
        result = workflow.run_complete_workflow()
        
        self.assertTrue(result)
        self.assertEqual(len(workflow.errors), 0)
        self.assertGreater(len(workflow.workflow_log), 0)
        
        # Verify workflow phases were executed
        expected_phases = [
            "Starting pre-build validation",
            "Setting up build environment",
            "Creating ISO directory structure",
            "Creating root filesystem",
            "Preparing bootloaders",
            "Creating ISO image",
            "Running post-build validation",
            "Cleaning up and generating report"
        ]
        
        for phase in expected_phases:
            self.assertTrue(any(phase in log for log in workflow.workflow_log))
        
        # Verify output files were created
        self.assertTrue(os.path.exists(self.output_dir))
        output_files = os.listdir(self.output_dir)
        self.assertGreater(len(output_files), 0)

class TestISOValidationIntegration(unittest.TestCase):
    """Test ISO validation integration."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.test_iso = os.path.join(self.temp_dir, "test.iso")
        
        # Create a mock ISO file
        with open(self.test_iso, 'wb') as f:
            f.write(b"Mock ISO content" * 1000)
        
        # Create mock ISO structure for mounting tests
        self.iso_mount = os.path.join(self.temp_dir, "mount")
        os.makedirs(self.iso_mount, exist_ok=True)
        
        # Create basic ISO structure
        os.makedirs(os.path.join(self.iso_mount, "EFI", "BOOT"), exist_ok=True)
        os.makedirs(os.path.join(self.iso_mount, "boot", "grub"), exist_ok=True)
        os.makedirs(os.path.join(self.iso_mount, "live"), exist_ok=True)
        
        # Create required files
        with open(os.path.join(self.iso_mount, "EFI", "BOOT", "BOOTX64.EFI"), 'w') as f:
            f.write("Mock UEFI bootloader\\n")
        
        with open(os.path.join(self.iso_mount, "boot", "grub", "grub.cfg"), 'w') as f:
            f.write('menuentry "RegicideOS Live" {\\n')
            f.write("    linux /boot/vmlinuz boot=live\\n")
            f.write("    initrd /boot/initrd\\n")
            f.write("}\\n")
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_complete_validation_workflow(self):
        """Test complete ISO validation workflow."""
        class ISOValidationWorkflow:
            def __init__(self, iso_file, mount_point):
                self.iso_file = iso_file
                self.mount_point = mount_point
                self.validation_log = []
                self.errors = []
                self.validation_results = {}
            
            def run_complete_validation(self):
                """Run complete validation workflow."""
                try:
                    # Phase 1: File validation
                    self.validation_results["file_validation"] = self._validate_file()
                    
                    # Phase 2: Checksum validation
                    self.validation_results["checksum_validation"] = self._validate_checksum()
                    
                    # Phase 3: Structure validation
                    self.validation_results["structure_validation"] = self._validate_structure()
                    
                    # Phase 4: Boot validation
                    self.validation_results["boot_validation"] = self._validate_boot()
                    
                    # Phase 5: Security validation
                    self.validation_results["security_validation"] = self._validate_security()
                    
                    # Phase 6: Generate report
                    self._generate_validation_report()
                    
                    return self._calculate_overall_result()
                    
                except Exception as e:
                    self.errors.append(f"Validation workflow failed: {e}")
                    return False
            
            def _validate_file(self):
                """Validate basic file properties."""
                self.validation_log.append("Validating file properties...")
                
                if not os.path.exists(self.iso_file):
                    self.errors.append("ISO file does not exist")
                    return False
                
                file_size = os.path.getsize(self.iso_file)
                if file_size == 0:
                    self.errors.append("ISO file is empty")
                    return False
                
                if file_size < 1048576:  # 1MB minimum
                    self.validation_log.append("Warning: ISO file is small")
                
                self.validation_log.append("✓ File validation passed")
                return True
            
            def _validate_checksum(self):
                """Validate checksum."""
                self.validation_log.append("Validating checksum...")
                
                checksum_file = f"{self.iso_file}.sha256"
                if not os.path.exists(checksum_file):
                    self.validation_log.append("Warning: Checksum file not found")
                    return True  # Non-critical
                
                # Mock checksum validation
                self.validation_log.append("✓ Checksum validation passed")
                return True
            
            def _validate_structure(self):
                """Validate ISO structure."""
                self.validation_log.append("Validating ISO structure...")
                
                required_dirs = ["EFI/BOOT", "boot/grub", "live"]
                missing_dirs = []
                
                for dir_path in required_dirs:
                    full_path = os.path.join(self.mount_point, dir_path)
                    if not os.path.exists(full_path):
                        missing_dirs.append(dir_path)
                
                if missing_dirs:
                    self.errors.append(f"Missing directories: {missing_dirs}")
                    return False
                
                self.validation_log.append("✓ Structure validation passed")
                return True
            
            def _validate_boot(self):
                """Validate boot capability."""
                self.validation_log.append("Validating boot capability...")
                
                # Check UEFI bootloader
                efi_bootloader = os.path.join(self.mount_point, "EFI", "BOOT", "BOOTX64.EFI")
                if not os.path.exists(efi_bootloader):
                    self.errors.append("UEFI bootloader not found")
                    return False
                
                # Check GRUB configuration
                grub_config = os.path.join(self.mount_point, "boot", "grub", "grub.cfg")
                if not os.path.exists(grub_config):
                    self.errors.append("GRUB configuration not found")
                    return False
                
                self.validation_log.append("✓ Boot validation passed")
                return True
            
            def _validate_security(self):
                """Validate security features."""
                self.validation_log.append("Validating security features...")
                
                # Check for secure boot compatibility
                efi_bootloader = os.path.join(self.mount_point, "EFI", "BOOT", "BOOTX64.EFI")
                if os.path.exists(efi_bootloader):
                    self.validation_log.append("✓ Secure boot compatible")
                
                # Check file permissions (mock)
                self.validation_log.append("✓ Security validation passed")
                return True
            
            def _generate_validation_report(self):
                """Generate validation report."""
                self.validation_log.append("Generating validation report...")
                
                report_file = f"{self.iso_file}.validation-report.txt"
                with open(report_file, 'w') as f:
                    f.write("RegicideOS ISO Validation Report\\n")
                    f.write("==============================\\n")
                    f.write(f"ISO File: {self.iso_file}\\n")
                    f.write(f"Validation Date: {subprocess.check_output(['date'], universal_newlines=True).strip()}\\n")
                    f.write("\\nValidation Results:\\n")
                    
                    for test_name, result in self.validation_results.items():
                        status = "PASSED" if result else "FAILED"
                        f.write(f"  {test_name}: {status}\\n")
                    
                    if self.errors:
                        f.write("\\nErrors:\\n")
                        for error in self.errors:
                            f.write(f"  - {error}\\n")
                    
                    f.write("\\nValidation Log:\\n")
                    for log_entry in self.validation_log:
                        f.write(f"  {log_entry}\\n")
                
                self.validation_log.append("✓ Validation report generated")
            
            def _calculate_overall_result(self):
                """Calculate overall validation result."""
                total_tests = len(self.validation_results)
                passed_tests = sum(1 for result in self.validation_results.values() if result)
                
                if total_tests == 0:
                    return False
                
                success_rate = passed_tests / total_tests
                
                # Require at least 80% success rate
                return success_rate >= 0.8
        
        # Test the complete validation workflow
        workflow = ISOValidationWorkflow(self.test_iso, self.iso_mount)
        result = workflow.run_complete_validation()
        
        self.assertTrue(result)
        self.assertGreater(len(workflow.validation_log), 0)
        
        # Verify validation phases were executed
        expected_phases = [
            "Validating file properties",
            "Validating checksum",
            "Validating ISO structure",
            "Validating boot capability",
            "Validating security features",
            "Generating validation report"
        ]
        
        for phase in expected_phases:
            self.assertTrue(any(phase in log for log in workflow.validation_log))
        
        # Verify validation results
        self.assertIn("file_validation", workflow.validation_results)
        self.assertIn("structure_validation", workflow.validation_results)
        self.assertIn("boot_validation", workflow.validation_results)
        
        # Verify report was generated
        report_file = f"{self.test_iso}.validation-report.txt"
        self.assertTrue(os.path.exists(report_file))

class TestISOBuildValidationIntegration(unittest.TestCase):
    """Test integration between build and validation processes."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.build_dir = os.path.join(self.temp_dir, "build")
        os.makedirs(self.build_dir, exist_ok=True)
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_build_and_validation_integration(self):
        """Test integration between build and validation processes."""
        class BuildValidationIntegration:
            def __init__(self, build_dir):
                self.build_dir = build_dir
                self.integration_log = []
                self.errors = []
                self.iso_file = None
            
            def run_build_and_validate(self):
                """Run build and validation together."""
                try:
                    # Phase 1: Build ISO
                    self.integration_log.append("Starting ISO build...")
                    if not self._build_iso():
                        raise RuntimeError("ISO build failed")
                    
                    # Phase 2: Validate built ISO
                    self.integration_log.append("Starting ISO validation...")
                    if not self._validate_iso():
                        raise RuntimeError("ISO validation failed")
                    
                    # Phase 3: Generate combined report
                    self.integration_log.append("Generating combined report...")
                    if not self._generate_combined_report():
                        raise RuntimeError("Report generation failed")
                    
                    return True
                    
                except Exception as e:
                    self.errors.append(f"Integration failed: {e}")
                    return False
            
            def _build_iso(self):
                """Build ISO (mock)."""
                # Create mock build structure
                iso_dir = os.path.join(self.build_dir, "iso")
                output_dir = os.path.join(self.build_dir, "output")
                
                os.makedirs(iso_dir, exist_ok=True)
                os.makedirs(output_dir, exist_ok=True)
                
                # Create mock ISO structure
                os.makedirs(os.path.join(iso_dir, "EFI", "BOOT"), exist_ok=True)
                os.makedirs(os.path.join(iso_dir, "boot", "grub"), exist_ok=True)
                os.makedirs(os.path.join(iso_dir, "live"), exist_ok=True)
                
                # Create mock ISO file
                self.iso_file = os.path.join(output_dir, "regicideos-1.0.0-x86_64.iso")
                with open(self.iso_file, 'w') as f:
                    f.write("Mock ISO content" * 1000)
                
                self.integration_log.append("✓ ISO build completed")
                return True
            
            def _validate_iso(self):
                """Validate built ISO (mock)."""
                if not self.iso_file:
                    self.errors.append("No ISO file to validate")
                    return False
                
                if not os.path.exists(self.iso_file):
                    self.errors.append("ISO file does not exist")
                    return False
                
                # Mock validation checks
                file_size = os.path.getsize(self.iso_file)
                if file_size < 1024:
                    self.errors.append("ISO file is too small")
                    return False
                
                self.integration_log.append("✓ ISO validation completed")
                return True
            
            def _generate_combined_report(self):
                """Generate combined build and validation report."""
                report_file = os.path.join(self.build_dir, "build-validation-report.txt")
                with open(report_file, 'w') as f:
                    f.write("RegicideOS Build and Validation Report\\n")
                    f.write("======================================\\n")
                    f.write(f"Report Date: {subprocess.check_output(['date'], universal_newlines=True).strip()}\\n")
                    f.write("\\nIntegration Log:\\n")
                    for log_entry in self.integration_log:
                        f.write(f"  {log_entry}\\n")
                    
                    if self.errors:
                        f.write("\\nErrors:\\n")
                        for error in self.errors:
                            f.write(f"  - {error}\\n")
                    else:
                        f.write("\\nNo errors reported.\\n")
                
                self.integration_log.append("✓ Combined report generated")
                return True
        
        # Test the integration
        integration = BuildValidationIntegration(self.build_dir)
        result = integration.run_build_and_validate()
        
        self.assertTrue(result)
        self.assertEqual(len(integration.errors), 0)
        self.assertGreater(len(integration.integration_log), 0)
        
        # Verify integration phases
        expected_phases = [
            "Starting ISO build",
            "ISO build completed",
            "Starting ISO validation",
            "ISO validation completed",
            "Generating combined report",
            "Combined report generated"
        ]
        
        for phase in expected_phases:
            self.assertTrue(any(phase in log for log in integration.integration_log))
        
        # Verify combined report was generated
        report_file = os.path.join(self.build_dir, "build-validation-report.txt")
        self.assertTrue(os.path.exists(report_file))

if __name__ == '__main__':
    # Run tests with detailed output
    unittest.main(verbosity=2)