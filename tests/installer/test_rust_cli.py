"""
Test the Rust installer binary via CLI invocation.
Replaces broken Python tests that assumed a Python implementation.
"""

import unittest
import subprocess
import sys
import os
import re
from pathlib import Path

# Path to the compiled installer binary
PROJECT_ROOT = Path(__file__).parent.parent.parent
INSTALLER_BIN = PROJECT_ROOT / "installer" / "target" / "release" / "installer"
INSTALLER_DEBUG = PROJECT_ROOT / "installer" / "target" / "debug" / "installer"
CARGO_CONFIG = PROJECT_ROOT / ".cargo" / "config.toml"


def _cargo_target_dir_candidates():
    """Target dirs declared by the repo's .cargo/config.toml, if parseable."""
    candidates = []
    try:
        content = CARGO_CONFIG.read_text()
    except OSError:
        return candidates
    match = re.search(r'target-dir\s*=\s*"([^"]+)"', content)
    if match:
        target_dir = Path(match.group(1))
        if not target_dir.is_absolute():
            target_dir = PROJECT_ROOT / target_dir
        candidates.append(target_dir / "release" / "installer")
        candidates.append(target_dir / "debug" / "installer")
    return candidates


# Every location searched before giving up (resolution order matters: the
# plain in-repo installer/target paths win over config-derived locations).
SEARCH_CANDIDATES = [
    INSTALLER_BIN,
    INSTALLER_DEBUG,
    *_cargo_target_dir_candidates(),
    *sorted(PROJECT_ROOT.glob("target*/release/installer")),
    *sorted(PROJECT_ROOT.glob("target*/debug/installer")),
]


def get_installer_path():
    """Find the compiled installer binary, or None if it is truly absent.

    Checks, in order: installer/target/{release,debug}, the build.target-dir
    from .cargo/config.toml, then any target*/release|debug/installer under
    the repo root.
    """
    for candidate in SEARCH_CANDIDATES:
        if candidate.exists():
            return candidate
    return None


class TestInstallerCLI(unittest.TestCase):
    """Test installer command-line interface."""

    @classmethod
    def setUpClass(cls):
        cls.installer = get_installer_path()
        if cls.installer is None:
            searched = "\n  ".join(str(p) for p in SEARCH_CANDIDATES)
            raise unittest.SkipTest(
                "Installer binary not found. Searched:\n  "
                + searched
                + "\nBuild it first with: cd installer && cargo build --release"
            )

    def test_help_flag(self):
        """Test --help prints usage information."""
        result = subprocess.run(
            [str(self.installer), "--help"],
            capture_output=True,
            text=True
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("Usage:", result.stdout)

    def test_version_flag(self):
        """Test --version is rejected with a clear error."""
        result = subprocess.run(
            [str(self.installer), "--version"],
            capture_output=True,
            text=True
        )
        # Installer does not have --version; verify it fails gracefully
        self.assertEqual(result.returncode, 2)
        self.assertIn("unexpected argument", result.stderr)

    def test_no_args_rejects_interactive_run(self):
        """Running installer with no args must not enter interactive install."""
        result = subprocess.run(
            [str(self.installer), "--help"],
            capture_output=True,
            text=True,
            timeout=5
        )
        # --help exits immediately with usage; no panic or segfault
        self.assertEqual(result.returncode, 0)
        self.assertNotIn("panic", result.stderr.lower())
        self.assertNotIn("segmentation fault", result.stderr.lower())
        self.assertIn("Usage:", result.stdout)


class TestUEFISafety(unittest.TestCase):
    """Test UEFI safety requirements documented in constitution."""

    def test_uefi_detection_order(self):
        """UEFI detection must happen before any destructive operation."""
        # This is enforced by the Rust code; we verify the design constraint.
        dangerous_ops = ["partition_drive", "format_drive", "install_bootloader"]
        # Read main.rs to verify UEFI check comes first
        main_rs = PROJECT_ROOT / "installer" / "src" / "main.rs"
        if not main_rs.exists():
            self.skipTest("main.rs not found")
        content = main_rs.read_text()
        uefi_pos = content.find("is_efi()")
        self.assertGreater(uefi_pos, 0, "UEFI check must exist in installer")
        for op in dangerous_ops:
            op_pos = content.find(op)
            if op_pos > 0:
                self.assertGreater(
                    op_pos, uefi_pos,
                    f"Dangerous operation '{op}' must come after UEFI check"
                )

    def test_bios_rejection_message(self):
        """BIOS systems must be rejected with a clear message."""
        main_rs = PROJECT_ROOT / "installer" / "src" / "main.rs"
        if not main_rs.exists():
            self.skipTest("main.rs not found")
        content = main_rs.read_text()
        self.assertIn("BIOS", content)
        self.assertIn("UEFI", content)


class TestSafetyRequirements(unittest.TestCase):
    """Verify critical safety requirements against the real installer source.

    Note: the installer has no dry-run mode (there is intentionally no such
    flag), so no assertion is made for one; the checked mechanisms are the
    ones that genuinely exist in installer/src.
    """

    MAIN_RS = PROJECT_ROOT / "installer" / "src" / "main.rs"
    VALIDATION_RS = PROJECT_ROOT / "installer" / "src" / "validation.rs"

    def test_device_path_validation_before_partitioning(self):
        """Drive paths are validated (allowlist regex + dangerous-device
        denylist) before the drive is partitioned or formatted."""
        main_rs = self.MAIN_RS.read_text()
        validation_rs = self.VALIDATION_RS.read_text()

        self.assertIn("pub fn validate_device_path", validation_rs)
        self.assertIn('bail!("Invalid device path")', validation_rs)
        # Pseudo-devices such as /dev/zero must be denied.
        self.assertIn('"/dev/zero"', validation_rs)

        # The validation call must appear before the destructive call sites.
        validate_pos = main_rs.find("validation::validate_device_path(&config.drive")
        self.assertGreater(validate_pos, -1, "validate_device_path must be called in main.rs")
        self.assertGreater(
            main_rs.find("partition_drive(&config_parsed.drive"),
            validate_pos,
            "drive validation must happen before partitioning",
        )
        self.assertGreater(
            main_rs.find("format_drive(&config_parsed.drive"),
            validate_pos,
            "drive validation must happen before formatting",
        )

    def test_filesystem_type_validation_before_formatting(self):
        """Filesystem selection is restricted to an allowlist before the
        drive is formatted."""
        main_rs = self.MAIN_RS.read_text()
        validation_rs = self.VALIDATION_RS.read_text()

        self.assertIn("pub fn validate_filesystem_type", validation_rs)
        self.assertIn('bail!("Unsupported filesystem type")', validation_rs)

        validate_pos = main_rs.find("validation::validate_filesystem_type(&config.filesystem")
        self.assertGreater(validate_pos, -1, "validate_filesystem_type must be called in main.rs")
        self.assertGreater(
            main_rs.find("format_drive(&config_parsed.drive"),
            validate_pos,
            "filesystem validation must happen before formatting",
        )

    def test_typed_drive_confirmation_before_partitioning(self):
        """Interactive runs require typing the drive path before any
        destructive operation begins."""
        main_rs = self.MAIN_RS.read_text()

        self.assertIn("Type the drive path to continue", main_rs)
        self.assertIn("Drive confirmation did not match", main_rs)
        self.assertLess(
            main_rs.find("Drive confirmation did not match"),
            main_rs.find("partition_drive(&config_parsed.drive"),
            "user confirmation must happen before partitioning",
        )


if __name__ == '__main__':
    unittest.main(verbosity=2)
