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
        os.makedirs(snapshot_dir)
        self.patch_overlay = mock.patch.object(rc, "OVERLAY_DIR", overlay_dir)
        self.patch_snapshots = mock.patch.object(rc, "SNAPSHOT_DIR", snapshot_dir)
        self.patch_current = mock.patch.object(
            rc, "CURRENT_FILE", os.path.join(overlay_dir, ".regicide-current")
        )
        self.patch_overlay.start()
        self.patch_snapshots.start()
        self.patch_current.start()
        self.addCleanup(self.patch_overlay.stop)
        self.addCleanup(self.patch_snapshots.stop)
        self.addCleanup(self.patch_current.stop)
        rc.PRETEND = True

    def test_ensure_snapshot_dir_is_idempotent_when_pretend(self):
        snapshots.ensure_snapshot_dir()
        self.assertTrue(os.path.isdir(rc.SNAPSHOT_DIR))

    def test_list_snapshot_sets_empty(self):
        self.assertEqual(snapshots.list_snapshot_sets(), [])

    def test_write_and_read_current(self):
        snapshots.write_current("initial")
        self.assertEqual(snapshots.read_current(), "initial")


if __name__ == "__main__":
    unittest.main()
