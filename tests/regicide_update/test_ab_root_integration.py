"""Real-Btrfs integration test for A/B root updates.

Requires root and btrfs/mkfs.btrfs/losetup. Skips cleanly otherwise.

The test creates a loop-mounted Btrfs filesystem representing the ROOTS
partition, installs a fake root image into the inactive A/B slot, verifies it,
activates the new slot, then rolls back to the previous slot.
"""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import pytest

from regicide_update import boot_entry, common as rc, root_ab


def _btrfs_tools_available() -> bool:
    return (
        shutil.which("btrfs") is not None
        and shutil.which("mkfs.btrfs") is not None
    )


class RootAbRealBtrfsIntegrationTest(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        roots_dir = os.path.join(self.tmpdir.name, "roots")
        os.makedirs(roots_dir)

        self._orig_pretend = rc.PRETEND
        rc.PRETEND = False
        self.addCleanup(setattr, rc, "PRETEND", self._orig_pretend)

        self.patch_roots = mock.patch.object(rc, "ROOTS_DIR", roots_dir)
        self.patch_roots.start()
        self.addCleanup(self.patch_roots.stop)
        root_ab.CURRENT_FILE = Path(roots_dir) / ".regicide-root-current"

        self.boot_dir = Path(self.tmpdir.name) / "boot"
        self.boot_patch = mock.patch.object(boot_entry, "ESP_BOOT_DIR", self.boot_dir)
        self.boot_patch.start()
        self.addCleanup(self.boot_patch.stop)

    @pytest.mark.skipif(not _btrfs_tools_available(), reason="Btrfs tools not available")
    @pytest.mark.skipif(os.geteuid() != 0, reason="Requires root for loop device mounts")
    def test_full_ab_root_update_and_rollback_on_real_btrfs(self):
        image = os.path.join(self.tmpdir.name, "roots.img")
        subprocess.run(["truncate", "-s", "512M", image], check=True)
        subprocess.run(["mkfs.btrfs", "-f", image], check=True)

        mount_point = os.path.join(self.tmpdir.name, "mnt")
        os.makedirs(mount_point)
        subprocess.run(["mount", "-o", "loop", image, mount_point], check=True)
        self.addCleanup(lambda: subprocess.run(["umount", mount_point], check=False))

        # Re-point ROOTS_DIR at the mounted real Btrfs filesystem.
        rc.ROOTS_DIR = mount_point
        root_ab.CURRENT_FILE = Path(mount_point) / ".regicide-root-current"

        # Seed the current (top-level) root with a minimal /boot so the
        # snapshot created for slot A during the first update is bootable.
        current_boot_dir = os.path.join(mount_point, "boot")
        os.makedirs(current_boot_dir, exist_ok=True)
        Path(os.path.join(current_boot_dir, "vmlinuz")).touch()
        Path(os.path.join(current_boot_dir, "initramfs.img")).touch()

        # Create a fake stage4 tarball containing a minimal bootable root.
        new_root_dir = os.path.join(self.tmpdir.name, "new-root")
        for d in ("usr", "bin", "lib", "etc", "var", "boot"):
            os.makedirs(os.path.join(new_root_dir, d))
        Path(os.path.join(new_root_dir, "boot", "vmlinuz")).touch()
        Path(os.path.join(new_root_dir, "boot", "initramfs.img")).touch()
        tarball = os.path.join(self.tmpdir.name, "new-root.tar.xz")
        subprocess.run(
            ["tar", "-C", new_root_dir, "-cJf", tarball, "."],
            check=True,
        )

        # Initially the top-level ROOTS partition is the active root.
        self.assertEqual(root_ab.read_active_slot(), "a")

        # Install the new root into the inactive slot and activate it.
        slot = root_ab.install_and_activate(Path(tarball))
        self.assertEqual(slot, "b")
        self.assertTrue(os.path.isdir(os.path.join(mount_point, "roots_b")))
        self.assertEqual(root_ab.read_active_slot(), "b")
        self.assertTrue(
            os.path.isfile(os.path.join(mount_point, "roots_b", "boot", "vmlinuz"))
        )

        # Verify boot entries point at the actual files in each slot.
        boot_entry.sync_entries()
        b_entry = (boot_entry.ESP_BOOT_DIR / "loader" / "entries" / "regicide-b.conf").read_text()
        self.assertIn("linux /vmlinuz", b_entry)
        self.assertIn("initrd /initramfs.img", b_entry)
        self.assertIn("root=LABEL=ROOTS ro rootflags=subvol=roots_b", b_entry)
        loader_conf = (boot_entry.ESP_BOOT_DIR / "loader" / "loader.conf").read_text()
        self.assertIn("default regicide-b", loader_conf)

        # Roll back to the previous slot.
        previous = root_ab.rollback()
        self.assertEqual(previous, "a")
        boot_entry.sync_entries()
        self.assertTrue(os.path.isdir(os.path.join(mount_point, "roots_a")))
        self.assertEqual(root_ab.read_active_slot(), "a")
        loader_conf = (boot_entry.ESP_BOOT_DIR / "loader" / "loader.conf").read_text()
        self.assertIn("default regicide-a", loader_conf)


if __name__ == "__main__":
    unittest.main()
