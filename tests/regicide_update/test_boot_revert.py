import os
import shutil
import tempfile
import unittest
from unittest import mock

from regicide_update import boot_revert, snapshots, common as rc


class BootRevertTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        overlay_dir = self.tmpdir.name
        roots_dir = os.path.join(self.tmpdir.name, "roots")
        snapshot_dir = os.path.join(overlay_dir, "snapshots")
        os.makedirs(snapshot_dir)
        os.makedirs(roots_dir)

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

    def _create_fake_snapshot_set(self, tag: str) -> str:
        name = snapshots._make_name(tag)
        target = os.path.join(rc.SNAPSHOT_DIR, name)
        os.makedirs(target)
        for subvol in rc.OVERLAY_SUBVOLUMES:
            os.makedirs(os.path.join(target, subvol))
        return name

    def _create_live_subvolumes(self):
        for subvol in rc.OVERLAY_SUBVOLUMES:
            os.makedirs(os.path.join(rc.OVERLAY_DIR, subvol))

    def test_apply_revert_no_flag_returns_false(self):
        self.assertFalse(boot_revert.apply_revert())

    def test_apply_revert_missing_target_cancels_flag(self):
        with open(rc.REVERT_FLAG, "w") as f:
            f.write("missing-snapshot")
        with mock.patch.object(rc, "warn"):
            self.assertFalse(boot_revert.apply_revert())
        self.assertFalse(os.path.exists(rc.REVERT_FLAG))

    def test_apply_revert_mounts_overlay_when_not_mounted(self):
        name = self._create_fake_snapshot_set("pre_update")
        self._create_live_subvolumes()
        with open(rc.REVERT_FLAG, "w") as f:
            f.write(name)

        with mock.patch.object(rc, "execute") as mock_execute:
            with mock.patch.object(rc, "info"):
                with mock.patch("os.path.ismount", return_value=False):
                    self.assertTrue(boot_revert.apply_revert())

        calls = [call.args for call in mock_execute.call_args_list]
        self.assertIn(("mount", ["LABEL=OVERLAY", rc.OVERLAY_DIR]), calls)

    def _fake_execute(self, program, args, **kwargs):
        """Create temp/backup directories and perform mocked mv/delete on disk."""
        if program == "btrfs" and args[:2] == ["subvolume", "snapshot"]:
            dst = args[-1]
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

    def test_apply_revert_creates_temp_then_swaps_into_place(self):
        name = self._create_fake_snapshot_set("pre_update")
        self._create_live_subvolumes()
        with open(rc.REVERT_FLAG, "w") as f:
            f.write(name)

        with mock.patch.object(rc, "execute", side_effect=self._fake_execute) as mock_execute:
            with mock.patch.object(rc, "info"):
                with mock.patch("os.path.ismount", return_value=True):
                    self.assertTrue(boot_revert.apply_revert())

        calls = [call.args for call in mock_execute.call_args_list]
        for subvol in rc.OVERLAY_SUBVOLUMES:
            live = os.path.join(rc.OVERLAY_DIR, subvol)
            snap = os.path.join(rc.SNAPSHOT_DIR, name, subvol)
            temp = f"{live}.regicide-revert-tmp"
            backup = f"{live}.regicide-revert-backup"
            self.assertIn(("btrfs", ["subvolume", "snapshot", snap, temp]), calls)
            self.assertIn(("mv", [live, backup]), calls)
            self.assertIn(("mv", [temp, live]), calls)
            self.assertIn(("btrfs", ["subvolume", "delete", backup]), calls)
            self.assertNotIn(("btrfs", ["subvolume", "delete", live]), calls)

    def test_apply_revert_keeps_live_path_valid_during_swap(self):
        """The live path must never be deleted before the replacement exists."""
        name = self._create_fake_snapshot_set("pre_update")
        self._create_live_subvolumes()
        with open(rc.REVERT_FLAG, "w") as f:
            f.write(name)

        with mock.patch.object(rc, "execute", side_effect=self._fake_execute) as mock_execute:
            with mock.patch.object(rc, "info"):
                with mock.patch("os.path.ismount", return_value=True):
                    boot_revert.apply_revert()

        calls = [call.args for call in mock_execute.call_args_list]
        for subvol in rc.OVERLAY_SUBVOLUMES:
            live = os.path.join(rc.OVERLAY_DIR, subvol)
            self.assertNotIn(("btrfs", ["subvolume", "delete", live]), calls)
            self.assertTrue(os.path.isdir(live))

    def test_apply_revert_removes_flag_and_writes_current(self):
        name = self._create_fake_snapshot_set("pre_update")
        self._create_live_subvolumes()
        with open(rc.REVERT_FLAG, "w") as f:
            f.write(name)

        with mock.patch.object(rc, "execute", side_effect=self._fake_execute):
            with mock.patch.object(rc, "info"):
                with mock.patch("os.path.ismount", return_value=True):
                    boot_revert.apply_revert()

        self.assertFalse(os.path.exists(rc.REVERT_FLAG))
        self.assertEqual(snapshots.read_current(), name)

    def test_apply_revert_skips_missing_snapshot_subvolume(self):
        name = self._create_fake_snapshot_set("pre_update")
        self._create_live_subvolumes()
        missing_subvol = rc.OVERLAY_SUBVOLUMES[0]
        os.rmdir(os.path.join(rc.SNAPSHOT_DIR, name, missing_subvol))

        with open(rc.REVERT_FLAG, "w") as f:
            f.write(name)

        with mock.patch.object(rc, "execute") as mock_execute:
            with mock.patch.object(rc, "info"):
                with mock.patch.object(rc, "warn"):
                    with mock.patch("os.path.ismount", return_value=True):
                        self.assertTrue(boot_revert.apply_revert())

        calls = [call.args for call in mock_execute.call_args_list]
        live = os.path.join(rc.OVERLAY_DIR, missing_subvol)
        self.assertNotIn(("btrfs", ["subvolume", "delete", live]), calls)

    def test_apply_revert_cleans_stale_temp_and_backup(self):
        name = self._create_fake_snapshot_set("pre_update")
        self._create_live_subvolumes()
        for subvol in rc.OVERLAY_SUBVOLUMES:
            live = os.path.join(rc.OVERLAY_DIR, subvol)
            os.makedirs(f"{live}.regicide-revert-tmp")
            os.makedirs(f"{live}.regicide-revert-backup")

        with open(rc.REVERT_FLAG, "w") as f:
            f.write(name)

        with mock.patch.object(rc, "execute", side_effect=self._fake_execute) as mock_execute:
            with mock.patch.object(rc, "info"):
                with mock.patch("os.path.ismount", return_value=True):
                    self.assertTrue(boot_revert.apply_revert())

        calls = [call.args for call in mock_execute.call_args_list]
        for subvol in rc.OVERLAY_SUBVOLUMES:
            live = os.path.join(rc.OVERLAY_DIR, subvol)
            self.assertIn(("btrfs", ["subvolume", "delete", f"{live}.regicide-revert-tmp"]), calls)
            self.assertIn(("btrfs", ["subvolume", "delete", f"{live}.regicide-revert-backup"]), calls)

    def test_main_leaves_flag_on_failure(self):
        """If apply_revert raises, main must leave the flag for the next boot."""
        with open(rc.REVERT_FLAG, "w") as f:
            f.write("pre_update")

        with mock.patch.object(boot_revert, "apply_revert", side_effect=RuntimeError("boom")):
            with self.assertRaises(SystemExit):
                boot_revert.main()

        self.assertTrue(os.path.exists(rc.REVERT_FLAG))


if __name__ == "__main__":
    unittest.main()
