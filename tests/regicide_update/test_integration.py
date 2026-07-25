"""Integration test for a full RegicideOS update/boot/rollback cycle.

This test can run in two modes:

1. Real Btrfs mode (preferred): if `mkfs.btrfs`, `btrfs`, and `losetup` are
   available and the test is run as root, it creates a loop-mounted Btrfs
   image, creates live subvolumes, snapshots them, mutates the live state,
   applies a boot-time revert, and verifies the restored state.

2. Pretend mode: if Btrfs tools are missing or the test is not run as root,
   the test uses rc.PRETEND=True and mocks `rc.execute` to verify that the
   regicide-update orchestration emits the correct btrfs command sequence for
   a full update/rollback cycle.

Run with real Btrfs:
    sudo python -m pytest tests/regicide_update/test_integration.py -v
"""

import os
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

import pytest

from regicide_update import boot_revert, snapshots, common as rc


def _btrfs_tools_available() -> bool:
    return shutil.which("btrfs") is not None and shutil.which("mkfs.btrfs") is not None


class FullCycleIntegrationTest(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        overlay_dir = self.tmpdir.name
        roots_dir = os.path.join(self.tmpdir.name, "roots")
        snapshot_dir = os.path.join(overlay_dir, "snapshots")
        os.makedirs(snapshot_dir)
        os.makedirs(roots_dir)

        self._orig_pretend = rc.PRETEND
        rc.PRETEND = False
        self.addCleanup(setattr, rc, "PRETEND", self._orig_pretend)

        self.patch_overlay = mock.patch.object(rc, "OVERLAY_DIR", overlay_dir)
        self.patch_roots = mock.patch.object(rc, "ROOTS_DIR", roots_dir)
        self.patch_snapshots = mock.patch.object(rc, "SNAPSHOT_DIR", snapshot_dir)
        self.patch_current = mock.patch.object(
            rc, "CURRENT_FILE", os.path.join(overlay_dir, ".regicide-current")
        )
        self.patch_revert = mock.patch.object(
            rc, "REVERT_FLAG", os.path.join(roots_dir, ".regicide-revert")
        )
        self.patch_overlay.start()
        self.patch_roots.start()
        self.patch_snapshots.start()
        self.patch_current.start()
        self.patch_revert.start()
        self.addCleanup(self.patch_overlay.stop)
        self.addCleanup(self.patch_roots.stop)
        self.addCleanup(self.patch_snapshots.stop)
        self.addCleanup(self.patch_current.stop)
        self.addCleanup(self.patch_revert.stop)

    def _create_live_subvolumes(self):
        for subvol in rc.OVERLAY_SUBVOLUMES:
            os.makedirs(os.path.join(rc.OVERLAY_DIR, subvol))

    def _write_state(self, subvol: str, filename: str, content: str):
        path = os.path.join(rc.OVERLAY_DIR, subvol, filename)
        with open(path, "w") as f:
            f.write(content)

    def _read_state(self, subvol: str, filename: str) -> str:
        path = os.path.join(rc.OVERLAY_DIR, subvol, filename)
        with open(path) as f:
            return f.read()

    @pytest.mark.skipif(not _btrfs_tools_available(), reason="Btrfs tools not available")
    @pytest.mark.skipif(os.geteuid() != 0, reason="Requires root for loop device mounts")
    def test_full_update_and_rollback_on_real_btrfs(self):
        image = os.path.join(self.tmpdir.name, "overlay.img")
        subprocess.run(["truncate", "-s", "256M", image], check=True)
        subprocess.run(["mkfs.btrfs", "-f", image], check=True)

        mount_point = os.path.join(self.tmpdir.name, "mnt")
        os.makedirs(mount_point)
        subprocess.run(["mount", "-o", "loop", image, mount_point], check=True)
        self.addCleanup(lambda: subprocess.run(["umount", mount_point], check=False))

        # Re-point OVERLAY_DIR at the mounted real Btrfs filesystem.
        rc.OVERLAY_DIR = mount_point
        rc.SNAPSHOT_DIR = os.path.join(mount_point, ".regicide-snapshots")
        rc.CURRENT_FILE = os.path.join(mount_point, ".regicide-current")

        # 1. Create initial Btrfs subvolumes for etc and var.
        for subvol in rc.OVERLAY_SUBVOLUMES:
            rc.execute("btrfs", ["subvolume", "create", os.path.join(mount_point, subvol)])

        # 2. Write initial system state.
        self._write_state("etc", "hostname", "regicide-initial")
        self._write_state("var", "marker", "initial")

        # 3. Create pre-update snapshot.
        pre = snapshots.create_snapshot_set("pre_upgrade")

        # 4. Simulate an update mutating the live state.
        self._write_state("etc", "hostname", "regicide-updated")
        self._write_state("var", "marker", "updated")

        # 5. Create post-update snapshot.
        post = snapshots.create_snapshot_set("post_upgrade")

        # 6. Verify live state is the updated state.
        self.assertEqual(self._read_state("etc", "hostname"), "regicide-updated")
        self.assertEqual(self._read_state("var", "marker"), "updated")

        # 7. Schedule a revert to the pre-update snapshot.
        snapshots.set_revert(pre)
        self.assertTrue(os.path.isfile(rc.REVERT_FLAG))

        # 8. Apply the revert (simulating boot-time revert service).
        boot_revert.apply_revert()

        # 9. Verify the live state is back to the pre-update snapshot.
        self.assertEqual(self._read_state("etc", "hostname"), "regicide-initial")
        self.assertEqual(self._read_state("var", "marker"), "initial")

        # 10. Verify the revert flag is gone and current points to pre.
        self.assertFalse(os.path.exists(rc.REVERT_FLAG))
        self.assertEqual(snapshots.read_current(), pre)

        # 11. Roll forward again by restoring the post-update snapshot.
        snapshots.set_revert(post)
        boot_revert.apply_revert()

        # 12. Verify live state is the updated state again.
        self.assertEqual(self._read_state("etc", "hostname"), "regicide-updated")
        self.assertEqual(self._read_state("var", "marker"), "updated")
        self.assertEqual(snapshots.read_current(), post)

    def test_full_update_and_rollback_orchestration_in_pretend_mode(self):
        """Verify the exact btrfs command orchestration for a full cycle."""
        orig_pretend = rc.PRETEND
        rc.PRETEND = True
        self.addCleanup(setattr, rc, "PRETEND", orig_pretend)
        commands = []

        def fake_execute(program, args, **kwargs):
            commands.append((program, args))
            # Mirror the real Btrfs sequence on disk so later commands succeed.
            if program == "btrfs" and args[:2] == ["subvolume", "snapshot"]:
                dst = args[-1]
                if dst.endswith(".regicide-revert-tmp") or dst.endswith(".regicide-revert-backup"):
                    os.makedirs(dst, exist_ok=True)
            elif program == "btrfs" and args[:2] == ["subvolume", "delete"]:
                target = args[-1]
                if os.path.isdir(target):
                    shutil.rmtree(target)
            elif program == "mv" and len(args) == 2:
                src, dst = args
                if os.path.isdir(src):
                    if os.path.isdir(dst):
                        shutil.rmtree(dst)
                    os.rename(src, dst)
            return ""

        with mock.patch.object(rc, "execute", side_effect=fake_execute):
            with mock.patch("os.path.ismount", return_value=True):
                self._create_live_subvolumes()
                self._write_state("etc", "hostname", "regicide-initial")
                self._write_state("var", "marker", "initial")

                pre = snapshots.create_snapshot_set("pre_upgrade")
                # Create the fake on-disk snapshot structure so set_revert passes.
                for subvol in rc.OVERLAY_SUBVOLUMES:
                    os.makedirs(os.path.join(rc.SNAPSHOT_DIR, pre, subvol), exist_ok=True)

                self._write_state("etc", "hostname", "regicide-updated")
                self._write_state("var", "marker", "updated")
                post = snapshots.create_snapshot_set("post_upgrade")
                for subvol in rc.OVERLAY_SUBVOLUMES:
                    os.makedirs(os.path.join(rc.SNAPSHOT_DIR, post, subvol), exist_ok=True)

                snapshots.set_revert(pre)
                boot_revert.apply_revert()

        # Expect snapshot commands for pre and post, plus revert restore commands.
        btrfs_commands = [args for prog, args in commands if prog == "btrfs"]
        self.assertIn(["subvolume", "snapshot", "-r", os.path.join(rc.OVERLAY_DIR, "etc"), os.path.join(rc.SNAPSHOT_DIR, pre, "etc")], btrfs_commands)
        self.assertIn(["subvolume", "snapshot", "-r", os.path.join(rc.OVERLAY_DIR, "var"), os.path.join(rc.SNAPSHOT_DIR, pre, "var")], btrfs_commands)
        self.assertIn(["subvolume", "snapshot", "-r", os.path.join(rc.OVERLAY_DIR, "etc"), os.path.join(rc.SNAPSHOT_DIR, post, "etc")], btrfs_commands)
        self.assertIn(["subvolume", "snapshot", "-r", os.path.join(rc.OVERLAY_DIR, "var"), os.path.join(rc.SNAPSHOT_DIR, post, "var")], btrfs_commands)

        for subvol in rc.OVERLAY_SUBVOLUMES:
            live = os.path.join(rc.OVERLAY_DIR, subvol)
            snap = os.path.join(rc.SNAPSHOT_DIR, pre, subvol)
            temp = f"{live}.regicide-revert-tmp"
            backup = f"{live}.regicide-revert-backup"
            self.assertIn(("btrfs", ["subvolume", "snapshot", snap, temp]), commands)
            self.assertIn(("mv", [live, backup]), commands)
            self.assertIn(("mv", [temp, live]), commands)
            self.assertNotIn(("btrfs", ["subvolume", "delete", live]), commands)
            self.assertIn(("btrfs", ["subvolume", "delete", backup]), commands)

        self.assertFalse(os.path.exists(rc.REVERT_FLAG))
        self.assertEqual(snapshots.read_current(), pre)


if __name__ == "__main__":
    unittest.main()
