import os
import tempfile
import unittest
from unittest import mock

from regicide_update import snapshots, common as rc


class SnapshotNameTests(unittest.TestCase):
    def test_make_name_contains_tag(self):
        name = snapshots._make_name("test")
        self.assertIn("test", name)
        self.assertEqual(name.count("_"), 2)


class SnapshotSetTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        overlay_dir = self.tmpdir.name
        snapshot_dir = os.path.join(overlay_dir, "snapshots")
        roots_dir = os.path.join(self.tmpdir.name, "roots")
        os.makedirs(snapshot_dir)
        os.makedirs(roots_dir)

        self.patch_overlay = mock.patch.object(rc, "OVERLAY_DIR", overlay_dir)
        self.patch_snapshots = mock.patch.object(rc, "SNAPSHOT_DIR", snapshot_dir)
        self.patch_current = mock.patch.object(
            rc, "CURRENT_FILE", os.path.join(overlay_dir, ".regicide-current")
        )
        self.patch_revert = mock.patch.object(
            rc, "REVERT_FLAG", os.path.join(roots_dir, ".regicide-revert")
        )
        self.patch_overlay.start()
        self.patch_snapshots.start()
        self.patch_current.start()
        self.patch_revert.start()
        self.addCleanup(self.patch_overlay.stop)
        self.addCleanup(self.patch_snapshots.stop)
        self.addCleanup(self.patch_current.stop)
        self.addCleanup(self.patch_revert.stop)

        self.execute_patch = mock.patch.object(rc, "execute")
        self.mock_execute = self.execute_patch.start()
        self.addCleanup(self.execute_patch.stop)

        # Pretend mode prevents real btrfs commands, but we track them.
        self._orig_pretend = rc.PRETEND
        rc.PRETEND = True
        self.addCleanup(setattr, rc, "PRETEND", self._orig_pretend)

        self._fake_counter = 0

    def _create_live_subvolumes(self):
        for subvol in rc.OVERLAY_SUBVOLUMES:
            os.makedirs(os.path.join(rc.OVERLAY_DIR, subvol))

    def _create_fake_snapshot_set(self, tag: str) -> str:
        """Create the on-disk structure of a snapshot set without btrfs.

        The name is guaranteed unique even when called multiple times in the
        same second, avoiding collisions in tests.
        """
        self._fake_counter += 1
        name = f"{snapshots._make_name(tag)}_{self._fake_counter:04d}"
        target = os.path.join(rc.SNAPSHOT_DIR, name)
        os.makedirs(target)
        for subvol in rc.OVERLAY_SUBVOLUMES:
            os.makedirs(os.path.join(target, subvol))
        return name

    def test_ensure_snapshot_dir_is_idempotent_when_pretend(self):
        snapshots.ensure_snapshot_dir()
        self.assertTrue(os.path.isdir(rc.SNAPSHOT_DIR))

    def test_list_snapshot_sets_empty(self):
        self.assertEqual(snapshots.list_snapshot_sets(), [])

    def test_write_and_read_current(self):
        snapshots.write_current("initial")
        self.assertEqual(snapshots.read_current(), "initial")

    def test_create_snapshot_set_records_current_and_returns_name(self):
        self._create_live_subvolumes()
        name = snapshots.create_snapshot_set("manual")
        self.assertIn("manual", name)
        self.assertEqual(snapshots.read_current(), name)
        self.assertEqual(self.mock_execute.call_count, len(rc.OVERLAY_SUBVOLUMES))
        for subvol in rc.OVERLAY_SUBVOLUMES:
            self.mock_execute.assert_any_call(
                "btrfs", ["subvolume", "snapshot", "-r", os.path.join(rc.OVERLAY_DIR, subvol), os.path.join(rc.SNAPSHOT_DIR, name, subvol)]
            )

    def test_list_snapshot_sets_sorts_by_name(self):
        first = self._create_fake_snapshot_set("manual")
        second = self._create_fake_snapshot_set("manual")
        sets = snapshots.list_snapshot_sets()
        self.assertEqual([s[0] for s in sets], sorted([first, second]))

    def test_delete_snapshot_set_invokes_btrfs_delete(self):
        name = self._create_fake_snapshot_set("manual")

        def fake_execute(program, args, **kwargs):
            # Simulate btrfs deleting the fake snapshot subvolume directory.
            if program == "btrfs" and args[:2] == ["subvolume", "delete"]:
                path = args[2]
                if os.path.isdir(path):
                    os.rmdir(path)

        rc.PRETEND = False
        with mock.patch.object(rc, "execute", side_effect=fake_execute) as mock_execute:
            snapshots.delete_snapshot_set(name)
            calls = [call.args for call in mock_execute.call_args_list]
            for subvol in rc.OVERLAY_SUBVOLUMES:
                self.assertIn(
                    ("btrfs", ["subvolume", "delete", os.path.join(rc.SNAPSHOT_DIR, name, subvol)]),
                    calls,
                )
        self.assertFalse(os.path.isdir(os.path.join(rc.SNAPSHOT_DIR, name)))

    def test_delete_snapshot_set_refuses_initial(self):
        with self.assertRaises(SystemExit):
            snapshots.delete_snapshot_set("initial")

    def test_set_revert_writes_flag(self):
        name = self._create_fake_snapshot_set("manual")
        with mock.patch.object(rc, "info"):
            snapshots.set_revert(name)
        self.assertTrue(os.path.isfile(rc.REVERT_FLAG))
        with open(rc.REVERT_FLAG) as f:
            self.assertEqual(f.read().strip(), name)

    def test_set_revert_dies_when_snapshot_missing(self):
        with self.assertRaises(SystemExit):
            snapshots.set_revert("does-not-exist")

    def test_cancel_revert_removes_flag(self):
        name = self._create_fake_snapshot_set("manual")
        with mock.patch.object(rc, "info"):
            snapshots.set_revert(name)
            snapshots.cancel_revert()
        self.assertFalse(os.path.exists(rc.REVERT_FLAG))

    def test_apply_retention_keeps_current_and_prunes_older(self):
        names = [self._create_fake_snapshot_set("manual") for _ in range(7)]
        current = names[-1]
        snapshots.write_current(current)
        rc.PRETEND = False
        with mock.patch.object(snapshots, "delete_snapshot_set") as mock_delete:
            with mock.patch.object(rc, "info"):
                with mock.patch.object(rc, "warn"):
                    snapshots.apply_retention(keep_count=3)
        # With 7 sets and current as the newest, candidates are the 6 non-current
        # sets. Keeping 3 means pruning the oldest 3.
        expected_removed = set(names[:3])
        actual_removed = {call.args[0] for call in mock_delete.call_args_list}
        self.assertEqual(actual_removed, expected_removed)
        self.assertNotIn(current, actual_removed)


if __name__ == "__main__":
    unittest.main()
