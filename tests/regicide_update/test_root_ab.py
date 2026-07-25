import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from regicide_update import common as rc, root_ab


class RootAbTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        roots_dir = os.path.join(self.tmpdir.name, "roots")
        os.makedirs(roots_dir)
        self.roots_patch = mock.patch.object(rc, "ROOTS_DIR", roots_dir)
        self.roots_patch.start()
        self.addCleanup(self.roots_patch.stop)
        root_ab.CURRENT_FILE = Path(roots_dir) / ".regicide-root-current"

        self._orig_pretend = rc.PRETEND
        rc.PRETEND = True
        self.addCleanup(setattr, rc, "PRETEND", self._orig_pretend)

        self.execute_patch = mock.patch.object(rc, "execute")
        self.mock_execute = self.execute_patch.start()
        self.addCleanup(self.execute_patch.stop)

        self.is_btrfs_patch = mock.patch.object(rc, "is_btrfs", return_value=True)
        self.mock_is_btrfs = self.is_btrfs_patch.start()
        self.addCleanup(self.is_btrfs_patch.stop)

    def _fake_execute_makedirs(self, program, args, **kwargs):
        import shutil
        if program == "btrfs" and args[:2] == ["subvolume", "create"]:
            os.makedirs(args[-1], exist_ok=True)
        elif program == "btrfs" and args[:2] == ["subvolume", "snapshot"]:
            os.makedirs(args[-1], exist_ok=True)
        elif program == "btrfs" and args[:2] == ["subvolume", "delete"]:
            target = args[-1]
            if os.path.isdir(target):
                shutil.rmtree(target)

    def test_read_active_slot_defaults_to_a(self):
        self.assertEqual(root_ab.read_active_slot(), "a")

    def test_write_active_slot_persists_choice(self):
        root_ab.write_active_slot("b")
        self.assertEqual(root_ab.read_active_slot(), "b")

    def test_write_active_slot_rejects_invalid(self):
        with self.assertRaises(SystemExit):
            root_ab.write_active_slot("c")

    def test_prepare_update_slot_returns_inactive_slot(self):
        root_ab.CURRENT_FILE.write_text("a\n")
        with mock.patch.object(rc, "execute", side_effect=self._fake_execute_makedirs):
            inactive = root_ab.prepare_update_slot()
        self.assertEqual(inactive, os.path.join(rc.ROOTS_DIR, "roots_b"))
        self.assertTrue(os.path.isdir(inactive))

    def test_prepare_update_slot_wipes_existing_inactive(self):
        root_ab.CURRENT_FILE.write_text("a\n")
        old_b = os.path.join(rc.ROOTS_DIR, "roots_b")
        os.makedirs(old_b)
        with mock.patch.object(rc, "execute", side_effect=self._fake_execute_makedirs):
            new_b = root_ab.prepare_update_slot()
        self.assertEqual(new_b, old_b)

    def test_verify_root_passes_with_required_dirs_and_boot_files(self):
        path = os.path.join(self.tmpdir.name, "roots_b")
        for d in ("usr", "bin", "lib", "etc", "var", "boot"):
            os.makedirs(os.path.join(path, d))
        Path(os.path.join(path, "boot", "vmlinuz")).touch()
        Path(os.path.join(path, "boot", "initramfs.img")).touch()
        self.assertTrue(root_ab.verify_root(path))

    def test_verify_root_fails_when_required_dir_missing(self):
        path = os.path.join(self.tmpdir.name, "roots_b")
        for d in ("usr", "etc", "var"):
            os.makedirs(os.path.join(path, d))
        self.assertFalse(root_ab.verify_root(path))

    def test_verify_root_fails_when_boot_files_missing(self):
        path = os.path.join(self.tmpdir.name, "roots_b")
        for d in ("usr", "bin", "lib", "etc", "var", "boot"):
            os.makedirs(os.path.join(path, d))
        self.assertFalse(root_ab.verify_root(path))

    def test_verify_root_accepts_versioned_kernel_names(self):
        path = os.path.join(self.tmpdir.name, "roots_b")
        for d in ("usr", "bin", "lib", "etc", "var", "boot"):
            os.makedirs(os.path.join(path, d))
        Path(os.path.join(path, "boot", "vmlinuz-6.8.0-gentoo")).touch()
        Path(os.path.join(path, "boot", "initramfs-6.8.0-gentoo.img")).touch()
        self.assertTrue(root_ab.verify_root(path))

    def test_activate_slot_writes_current(self):
        os.makedirs(os.path.join(rc.ROOTS_DIR, "roots_a"))
        root_ab.activate_slot("a")
        self.assertEqual(root_ab.read_active_slot(), "a")

    def test_activate_slot_dies_when_subvolume_missing(self):
        with self.assertRaises(SystemExit):
            root_ab.activate_slot("a")

    def test_rollback_switches_to_other_slot(self):
        for slot in ("roots_a", "roots_b"):
            os.makedirs(os.path.join(rc.ROOTS_DIR, slot))
        root_ab.write_active_slot("a")
        previous = root_ab.rollback()
        self.assertEqual(previous, "b")
        self.assertEqual(root_ab.read_active_slot(), "b")

    def test_install_and_activate_full_cycle(self):
        """End-to-end update: prepare slot, install image, verify, activate."""
        image_file = Path(self.tmpdir.name) / "new-root.tar.xz"
        image_file.write_text("")

        def fake_execute(program, args, **kwargs):
            if program == "btrfs" and args[:2] == ["subvolume", "create"]:
                os.makedirs(args[-1], exist_ok=True)
            elif program == "tar":
                target = args[1]
                for d in ("usr", "bin", "lib", "etc", "var", "boot"):
                    os.makedirs(os.path.join(target, d), exist_ok=True)
                Path(os.path.join(target, "boot", "vmlinuz")).touch()
                Path(os.path.join(target, "boot", "initramfs.img")).touch()

        root_ab.CURRENT_FILE.write_text("a\n")
        with mock.patch.object(rc, "execute", side_effect=fake_execute):
            slot = root_ab.install_and_activate(image_file)
        self.assertEqual(slot, "b")
        self.assertEqual(root_ab.read_active_slot(), "b")

    def test_install_and_activate_dies_when_verification_fails(self):
        image_file = Path(self.tmpdir.name) / "new-root.tar.xz"
        image_file.write_text("")

        def fake_execute(program, args, **kwargs):
            if program == "btrfs" and args[:2] == ["subvolume", "create"]:
                os.makedirs(args[-1], exist_ok=True)
            elif program == "tar":
                target = args[1]
                os.makedirs(os.path.join(target, "usr"), exist_ok=True)

        root_ab.CURRENT_FILE.write_text("a\n")
        with mock.patch.object(rc, "execute", side_effect=fake_execute):
            with self.assertRaises(SystemExit):
                root_ab.install_and_activate(image_file)


if __name__ == "__main__":
    unittest.main()
