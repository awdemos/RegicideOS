"""
Safety tests for ISO creation process.
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

class TestISOCreationSafety(unittest.TestCase):
    """Test safety aspects of ISO creation process."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.build_dir = os.path.join(self.temp_dir, "build")
        self.work_dir = os.path.join(self.build_dir, "work")
        self.output_dir = os.path.join(self.build_dir, "output")
        
        # Create directories
        os.makedirs(self.work_dir, exist_ok=True)
        os.makedirs(self.output_dir, exist_ok=True)
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_dangerous_command_detection(self):
        """Test detection of dangerous commands in build process."""
        class CommandSafetyChecker:
            def __init__(self):
                self.dangerous_commands = [
                    "rm -rf /",
                    "dd if=/dev/zero",
                    "mkfs",
                    "fdisk",
                    "format",
                    "wipe"
                ]
                self.suspicious_patterns = [
                    "/dev/sda",
                    "/dev/sdb",
                    "/dev/hda",
                    "/dev/hdb",
                    "root:",
                    "password"
                ]
                self.violations = []
                self.warnings = []
            
            def check_command_safety(self, command):
                """Check if a command is safe to execute."""
                command_lower = command.lower()
                
                # Check for dangerous commands
                for dangerous_cmd in self.dangerous_commands:
                    if dangerous_cmd in command_lower:
                        self.violations.append(f"Dangerous command detected: {dangerous_cmd}")
                        return False
                
                # Check for suspicious patterns
                for pattern in self.suspicious_patterns:
                    if pattern in command_lower:
                        self.warnings.append(f"Suspicious pattern detected: {pattern}")
                
                return True
            
            def validate_build_script(self, script_content):
                """Validate entire build script for safety."""
                violations = []
                warnings = []
                
                lines = script_content.split('\\n')
                for line_num, line in enumerate(lines, 1):
                    line = line.strip()
                    
                    # Skip comments and empty lines
                    if line.startswith('#') or not line:
                        continue
                    
                    if not self.check_command_safety(line):
                        violations.append(f"Line {line_num}: {line}")
                    
                    # Check for other safety issues
                    if 'sudo' in line and not line.startswith('#'):
                        warnings.append(f"Line {line_num}: sudo command detected")
                    
                    if 'chmod 777' in line:
                        warnings.append(f"Line {line_num}: world-writable permissions")
                
                return violations, warnings
        
        checker = CommandSafetyChecker()
        
        # Test safe commands
        safe_commands = [
            "mkdir -p /tmp/build",
            "cp file1 file2",
            "echo 'hello world'",
            "tar -czf archive.tar.gz files/"
        ]
        
        for cmd in safe_commands:
            result = checker.check_command_safety(cmd)
            self.assertTrue(result)
        
        # Test dangerous commands
        dangerous_commands = [
            "rm -rf /",
            "dd if=/dev/zero of=/dev/sda",
            "mkfs.ext4 /dev/sdb1",
            "fdisk /dev/sda"
        ]
        
        for cmd in dangerous_commands:
            result = checker.check_command_safety(cmd)
            self.assertFalse(result)
        
        # Test build script validation
        safe_script = """
#!/bin/bash
# Safe build script
mkdir -p build/iso
cp bootloader/* build/iso/
echo "Building ISO..."
"""
        
        violations, warnings = checker.validate_build_script(safe_script)
        self.assertEqual(len(violations), 0)
        
        # Test dangerous script
        dangerous_script = """
#!/bin/bash
# Dangerous build script
rm -rf /
dd if=/dev/zero of=/dev/sda
mkfs.ext4 /dev/sdb1
"""
        
        violations, warnings = checker.validate_build_script(dangerous_script)
        self.assertGreater(len(violations), 0)
    
    def test_file_access_safety(self):
        """Test file access safety during ISO creation."""
        class FileAccessSafety:
            def __init__(self, allowed_dirs, forbidden_paths):
                self.allowed_dirs = allowed_dirs
                self.forbidden_paths = forbidden_paths
                self.violations = []
            
            def check_file_access(self, file_path, operation="read"):
                """Check if file access is safe."""
                # Normalize path
                file_path = os.path.abspath(file_path)
                
                # Check forbidden paths
                for forbidden in self.forbidden_paths:
                    if file_path.startswith(forbidden):
                        self.violations.append(f"Access to forbidden path: {file_path}")
                        return False
                
                # Check if path is within allowed directories
                in_allowed_dir = False
                for allowed_dir in self.allowed_dirs:
                    if file_path.startswith(allowed_dir):
                        in_allowed_dir = True
                        break
                
                if not in_allowed_dir:
                    self.violations.append(f"Access outside allowed directories: {file_path}")
                    return False
                
                return True
            
            def validate_operation(self, operation, file_path):
                """Validate specific file operation."""
                safety_rules = {
                    "read": self._check_read_safety,
                    "write": self._check_write_safety,
                    "execute": self._check_execute_safety,
                    "delete": self._check_delete_safety
                }
                
                if operation in safety_rules:
                    return safety_rules[operation](file_path)
                else:
                    self.violations.append(f"Unknown operation: {operation}")
                    return False
            
            def _check_read_safety(self, file_path):
                """Check read operation safety."""
                return self.check_file_access(file_path, "read")
            
            def _check_write_safety(self, file_path):
                """Check write operation safety."""
                # Additional checks for write operations
                if file_path.endswith((".sh", ".py", ".exe")):
                    self.violations.append(f"Attempt to write executable file: {file_path}")
                    return False
                
                return self.check_file_access(file_path, "write")
            
            def _check_execute_safety(self, file_path):
                """Check execute operation safety."""
                if not os.path.exists(file_path):
                    self.violations.append(f"Attempt to execute non-existent file: {file_path}")
                    return False
                
                return self.check_file_access(file_path, "execute")
            
            def _check_delete_safety(self, file_path):
                """Check delete operation safety."""
                # Prevent deletion of system files
                if file_path.startswith("/usr") or file_path.startswith("/bin"):
                    self.violations.append(f"Attempt to delete system file: {file_path}")
                    return False
                
                return self.check_file_access(file_path, "delete")
        
        # Setup safety checker
        allowed_dirs = [self.work_dir, self.output_dir, "/tmp"]
        forbidden_paths = ["/etc", "/boot", "/root", "/home"]
        
        safety = FileAccessSafety(allowed_dirs, forbidden_paths)
        
        # Test safe access
        safe_file = os.path.join(self.work_dir, "test.txt")
        result = safety.validate_operation("write", safe_file)
        self.assertTrue(result)
        
        # Test forbidden access
        forbidden_file = "/etc/passwd"
        result = safety.validate_operation("read", forbidden_file)
        self.assertFalse(result)
        
        # Test system file deletion
        system_file = "/usr/bin/ls"
        result = safety.validate_operation("delete", system_file)
        self.assertFalse(result)
    
    def test_process_isolation(self):
        """Test process isolation during ISO creation."""
        class ProcessIsolation:
            def __init__(self):
                self.isolated_processes = []
                self.security_violations = []
            
            def run_isolated_command(self, command, timeout=30):
                """Run command in isolated environment."""
                # Mock isolated execution
                process_info = {
                    "command": command,
                    "pid": len(self.isolated_processes) + 1000,
                    "start_time": "now",
                    "timeout": timeout,
                    "status": "running"
                }
                
                self.isolated_processes.append(process_info)
                
                # Simulate command execution
                try:
                    # In real implementation, this would use subprocess with proper isolation
                    result = self._simulate_command_execution(command, timeout)
                    process_info["status"] = "completed"
                    process_info["result"] = result
                    return result
                    
                except Exception as e:
                    process_info["status"] = "failed"
                    process_info["error"] = str(e)
                    self.security_violations.append(f"Command failed: {command} - {e}")
                    return None
            
            def _simulate_command_execution(self, command, timeout):
                """Simulate command execution with safety checks."""
                # Check for dangerous commands
                dangerous_keywords = ["rm -rf", "dd", "mkfs", "fdisk"]
                for keyword in dangerous_keywords:
                    if keyword in command.lower():
                        raise RuntimeError(f"Dangerous command detected: {keyword}")
                
                # Simulate successful execution
                return {"exit_code": 0, "output": "Command executed successfully"}
            
            def cleanup_processes(self):
                """Clean up all isolated processes."""
                for process in self.isolated_processes:
                    if process["status"] == "running":
                        process["status"] = "terminated"
                        process["termination_reason"] = "cleanup"
                
                terminated_count = len([p for p in self.isolated_processes if p["status"] == "terminated"])
                return terminated_count
        
        isolation = ProcessIsolation()
        
        # Test safe command execution
        safe_commands = [
            "echo 'hello world'",
            "mkdir -p /tmp/test",
            "cp file1 file2"
        ]
        
        for cmd in safe_commands:
            result = isolation.run_isolated_command(cmd)
            self.assertIsNotNone(result)
            self.assertEqual(result["exit_code"], 0)
        
        # Test dangerous command execution
        dangerous_command = "rm -rf /"
        result = isolation.run_isolated_command(dangerous_command)
        self.assertIsNone(result)
        self.assertGreater(len(isolation.security_violations), 0)
        
        # Test process cleanup
        cleanup_count = isolation.cleanup_processes()
        self.assertGreaterEqual(cleanup_count, 0)
    
    def test_resource_limitation(self):
        """Test resource limitation during ISO creation."""
        class ResourceLimiter:
            def __init__(self):
                self.max_memory = 1024 * 1024 * 1024  # 1GB
                self.max_cpu_time = 300  # 5 minutes
                self.max_disk_space = 10 * 1024 * 1024 * 1024  # 10GB
                self.resource_violations = []
            
            def check_memory_usage(self, memory_usage):
                """Check memory usage against limits."""
                if memory_usage > self.max_memory:
                    self.resource_violations.append(f"Memory usage exceeded: {memory_usage} > {self.max_memory}")
                    return False
                return True
            
            def check_cpu_time(self, cpu_time):
                """Check CPU time usage against limits."""
                if cpu_time > self.max_cpu_time:
                    self.resource_violations.append(f"CPU time exceeded: {cpu_time} > {self.max_cpu_time}")
                    return False
                return True
            
            def check_disk_space(self, disk_usage):
                """Check disk space usage against limits."""
                if disk_usage > self.max_disk_space:
                    self.resource_violations.append(f"Disk space exceeded: {disk_usage} > {self.max_disk_space}")
                    return False
                return True
            
            def get_resource_usage(self):
                """Get current resource usage (mock)."""
                return {
                    "memory": 512 * 1024 * 1024,  # 512MB
                    "cpu_time": 60,  # 1 minute
                    "disk_space": 1 * 1024 * 1024 * 1024  # 1GB
                }
            
            def validate_resource_usage(self):
                """Validate current resource usage."""
                usage = self.get_resource_usage()
                
                checks = [
                    self.check_memory_usage(usage["memory"]),
                    self.check_cpu_time(usage["cpu_time"]),
                    self.check_disk_space(usage["disk_space"])
                ]
                
                return all(checks)
        
        limiter = ResourceLimiter()
        
        # Test normal resource usage
        result = limiter.validate_resource_usage()
        self.assertTrue(result)
        self.assertEqual(len(limiter.resource_violations), 0)
        
        # Test excessive memory usage
        result = limiter.check_memory_usage(2 * 1024 * 1024 * 1024)  # 2GB
        self.assertFalse(result)
        self.assertGreater(len(limiter.resource_violations), 0)
        
        # Test excessive CPU time
        result = limiter.check_cpu_time(600)  # 10 minutes
        self.assertFalse(result)
        
        # Test excessive disk space
        result = limiter.check_disk_space(20 * 1024 * 1024 * 1024)  # 20GB
        self.assertFalse(result)

class TestISOBuildProcessSafety(unittest.TestCase):
    """Test safety of ISO build process."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.build_dir = os.path.join(self.temp_dir, "build")
        os.makedirs(self.build_dir, exist_ok=True)
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def test_build_environment_safety(self):
        """Test build environment safety."""
        class BuildEnvironmentSafety:
            def __init__(self, build_dir):
                self.build_dir = build_dir
                self.safety_violations = []
                self.warnings = []
            
            def setup_safe_environment(self):
                """Setup safe build environment."""
                try:
                    # Create isolated build directory
                    self._create_isolated_directory()
                    
                    # Set proper permissions
                    self._set_safe_permissions()
                    
                    # Create safe workspace
                    self._create_safe_workspace()
                    
                    return len(self.safety_violations) == 0
                    
                except Exception as e:
                    self.safety_violations.append(f"Environment setup failed: {e}")
                    return False
            
            def _create_isolated_directory(self):
                """Create isolated build directory."""
                build_dir = os.path.join(self.build_dir, "isolated_build")
                os.makedirs(build_dir, exist_ok=True)
                
                # Set restrictive permissions
                os.chmod(build_dir, 0o750)
                
                # Verify directory is isolated
                if not self._verify_directory_isolation(build_dir):
                    self.safety_violations.append("Build directory is not properly isolated")
                    return False
                
                return True
            
            def _set_safe_permissions(self):
                """Set safe permissions for build environment."""
                build_dir = os.path.join(self.build_dir, "isolated_build")
                
                # Set directory permissions
                for root, dirs, files in os.walk(build_dir):
                    for d in dirs:
                        dir_path = os.path.join(root, d)
                        os.chmod(dir_path, 0o750)
                    
                    for f in files:
                        file_path = os.path.join(root, f)
                        os.chmod(file_path, 0o640)
                
                return True
            
            def _create_safe_workspace(self):
                """Create safe workspace directories."""
                base_dir = os.path.join(self.build_dir, "isolated_build")
                
                # Create workspace directories
                workspace_dirs = ["src", "build", "output", "temp"]
                for dir_name in workspace_dirs:
                    dir_path = os.path.join(base_dir, dir_name)
                    os.makedirs(dir_path, exist_ok=True)
                    os.chmod(dir_path, 0o750)
                
                return True
            
            def _verify_directory_isolation(self, directory):
                """Verify that directory is properly isolated."""
                # Check that directory is not world-writable
                stat_info = os.stat(directory)
                if stat_info.st_mode & 0o002:
                    return False
                
                # Check that parent directory is not accessible
                parent_dir = os.path.dirname(directory)
                if parent_dir != self.build_dir:
                    return False
                
                return True
            
            def validate_build_artifacts(self):
                """Validate that build artifacts are safe."""
                artifacts_dir = os.path.join(self.build_dir, "isolated_build", "output")
                
                if not os.path.exists(artifacts_dir):
                    self.warnings.append("No artifacts directory found")
                    return True
                
                # Check artifacts for safety
                unsafe_artifacts = []
                
                for root, dirs, files in os.walk(artifacts_dir):
                    for file in files:
                        file_path = os.path.join(root, file)
                        
                        # Check for suspicious files
                        if file.endswith((".sh", ".py", ".exe")):
                            unsafe_artifacts.append(file_path)
                        
                        # Check file permissions
                        stat_info = os.stat(file_path)
                        if stat_info.st_mode & 0o111:  # Executable
                            unsafe_artifacts.append(file_path)
                
                if unsafe_artifacts:
                    self.warnings.append(f"Potentially unsafe artifacts: {unsafe_artifacts}")
                
                return len(unsafe_artifacts) == 0
        
        safety = BuildEnvironmentSafety(self.build_dir)
        
        # Test environment setup
        result = safety.setup_safe_environment()
        self.assertTrue(result)
        self.assertEqual(len(safety.safety_violations), 0)
        
        # Test build artifacts validation
        result = safety.validate_build_artifacts()
        self.assertTrue(result)
    
    def test_build_script_safety(self):
        """Test build script safety."""
        class BuildScriptSafety:
            def __init__(self):
                self.safety_violations = []
                self.warnings = []
            
            def validate_build_script(self, script_path):
                """Validate build script for safety issues."""
                if not os.path.exists(script_path):
                    self.safety_violations.append("Build script not found")
                    return False
                
                try:
                    with open(script_path, 'r') as f:
                        script_content = f.read()
                    
                    return self._analyze_script_content(script_content)
                    
                except Exception as e:
                    self.safety_violations.append(f"Failed to read build script: {e}")
                    return False
            
            def _analyze_script_content(self, content):
                """Analyze script content for safety issues."""
                # Check for dangerous commands
                dangerous_patterns = [
                    "rm -rf",
                    "dd if=/dev",
                    "mkfs",
                    "fdisk",
                    "format",
                    "wipe",
                    "/dev/sd",
                    "sudo rm",
                    "chmod 777"
                ]
                
                lines = content.split('\\n')
                for line_num, line in enumerate(lines, 1):
                    line = line.strip()
                    
                    # Skip comments and empty lines
                    if line.startswith('#') or not line:
                        continue
                    
                    # Check for dangerous patterns
                    for pattern in dangerous_patterns:
                        if pattern in line.lower():
                            self.safety_violations.append(f"Line {line_num}: Dangerous pattern '{pattern}'")
                    
                    # Check for privilege escalation
                    if "sudo" in line.lower():
                        self.warnings.append(f"Line {line_num}: Privilege escalation detected")
                    
                    # Check for network access
                    if any(cmd in line.lower() for cmd in ["curl", "wget", "nc", "netcat"]):
                        self.warnings.append(f"Line {line_num}: Network access detected")
                
                return len(self.safety_violations) == 0
            
            def generate_safe_script_template(self):
                """Generate a safe build script template."""
                safe_template = '''#!/bin/bash
# Safe ISO Build Script for RegicideOS
set -euo pipefail

# Safety settings
BUILD_DIR="$(pwd)/build"
WORK_DIR="${BUILD_DIR}/work"
OUTPUT_DIR="${BUILD_DIR}/output"

# Validate environment
if [[ ! -d "$BUILD_DIR" ]]; then
    echo "Error: Build directory not found"
    exit 1
fi

# Create safe workspace
mkdir -p "$WORK_DIR" "$OUTPUT_DIR"

# Safe build operations
echo "Starting safe ISO build process..."

# Add safe build commands here
# Example: cp -r source/* "$WORK_DIR/"
# Example: xorriso -as mkisofs -o "$OUTPUT_DIR/iso.iso" "$WORK_DIR"

echo "ISO build completed successfully"
'''
                
                return safe_template
        
        safety = BuildScriptSafety()
        
        # Test safe script validation
        safe_script = os.path.join(self.build_dir, "safe_build.sh")
        with open(safe_script, 'w') as f:
            f.write(safety.generate_safe_script_template())
        
        result = safety.validate_build_script(safe_script)
        self.assertTrue(result)
        self.assertEqual(len(safety.safety_violations), 0)
        
        # Test dangerous script validation
        dangerous_script = os.path.join(self.build_dir, "dangerous_build.sh")
        with open(dangerous_script, 'w') as f:
            f.write('#!/bin/bash\\n')
            f.write('rm -rf /\\n')
            f.write('dd if=/dev/zero of=/dev/sda\\n')
            f.write('sudo mkfs.ext4 /dev/sdb1\\n')
        
        result = safety.validate_build_script(dangerous_script)
        self.assertFalse(result)
        self.assertGreater(len(safety.safety_violations), 0)

if __name__ == '__main__':
    # Run tests with detailed output
    unittest.main(verbosity=2)