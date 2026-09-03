import glob
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from regicide_update import boot_entry, common as rc, root_ab


class BootEntryTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        self.boot_patch = mock.patch.object(boot_entry, "ESP_BOOT_DIR", Path(self.tmpdir.name))
        self.boot_patch.start()
        self.addCleanup(self.boot_patch.stop)

        roots_dir = os.path.join(self.tmpdir.name, "roots")
        os.makedirs(roots_dir)
        self.roots_patch = mock.patch.object(rc, "ROOTS_DIR", roots_dir)
        self.roots_patch.start()
        self.addCleanup(self.roots_patch.stop)
        root_ab.CURRENT_FILE = Path(roots_dir) / ".regicide-root-current"

        self._orig_pretend = rc.PRETEND
        rc.PRETEND = True
        self.addCleanup(setattr, rc, "PRETEND", self._orig_pretend)

    def _make_slot_boot_dir(self, slot: str, kernel: str, initrd: str):
        boot_dir = os.path.join(rc.ROOTS_DIR, f"roots_{slot}", "boot")
        os.makedirs(boot_dir, exist_ok=True)
        Path(os.path.join(boot_dir, kernel)).touch()
        Path(os.path.join(boot_dir, initrd)).touch()

    def _slot_kernel_path(self, slot: str, name: str) -> str:
        return f"/roots/roots_{slot}/boot/{name}"

    def test_ensure_grub_cfg_creates_grub_cfg(self):
        boot_entry.ensure_grub_cfg()
        cfg = boot_entry._grub_cfg()
        self.assertTrue(cfg.is_file())
        text = cfg.read_text()
        self.assertIn("regicide_slot", text)
        self.assertIn('menuentry "RegicideOS ($regicide_slot)"', text)

    def test_ensure_grub_cfg_is_idempotent(self):
        boot_entry.ensure_grub_cfg()
        cfg = boot_entry._grub_cfg()
        first = cfg.read_text()
        # Manually append noise; ensure_grub_cfg should leave it alone.
        with open(cfg, "a") as f:
            f.write("# extra\n")
        boot_entry.ensure_grub_cfg()
        self.assertIn("# extra", cfg.read_text())

    def test_discover_kernel_initrd_finds_versioned_names(self):
        self._make_slot_boot_dir("a", "vmlinuz-6.8.0-gentoo", "initramfs-6.8.0-gentoo.img")
        kernel, initrd = boot_entry.discover_kernel_initrd("a")
        self.assertEqual(kernel, "vmlinuz-6.8.0-gentoo")
        self.assertEqual(initrd, "initramfs-6.8.0-gentoo.img")

    def test_discover_kernel_initrd_prefers_stable_names(self):
        boot_dir = os.path.join(rc.ROOTS_DIR, "roots_a", "boot")
        os.makedirs(boot_dir, exist_ok=True)
        Path(os.path.join(boot_dir, "vmlinuz-6.8.0-gentoo")).touch()
        Path(os.path.join(boot_dir, "vmlinuz")).touch()
        Path(os.path.join(boot_dir, "initramfs-6.8.0-gentoo.img")).touch()
        Path(os.path.join(boot_dir, "initramfs.img")).touch()
        kernel, initrd = boot_entry.discover_kernel_initrd("a")
        self.assertEqual(kernel, "vmlinuz")
        self.assertEqual(initrd, "initramfs.img")

    def test_discover_kernel_initrd_is_deterministic_with_many_files(self):
        boot_dir = os.path.join(rc.ROOTS_DIR, "roots_a", "boot")
        os.makedirs(boot_dir, exist_ok=True)
        for name in ["vmlinuz-6.10.0-gentoo", "vmlinuz-6.8.0-gentoo", "vmlinuz-6.9.0-gentoo"]:
            Path(os.path.join(boot_dir, name)).touch()
        for name in ["initramfs-6.10.0-gentoo.img", "initramfs-6.8.0-gentoo.img", "initramfs-6.9.0-gentoo.img"]:
            Path(os.path.join(boot_dir, name)).touch()
        kernel, initrd = boot_entry.discover_kernel_initrd("a")
        self.assertEqual(kernel, "vmlinuz-6.10.0-gentoo")
        self.assertEqual(initrd, "initramfs-6.10.0-gentoo.img")

    def test_write_slot_to_grubenv_rejects_invalid_slot(self):
        with self.assertRaises(SystemExit):
            boot_entry.write_slot_to_grubenv("c")

    @mock.patch.object(boot_entry, "_grub_editenv")
    def test_write_slot_to_grubenv_sets_slot(self, mock_editenv):
        boot_entry._init_grubenv()
        boot_entry.write_slot_to_grubenv("b")
        mock_editenv.assert_called_once_with(["set", "regicide_slot=b"])

    def test_write_slot_to_grubenv_restores_backup_on_failure(self):
        boot_entry._init_grubenv()
        env_path = boot_entry._grubenv_path()
        env_path.write_text("# existing block\n")

        def boom(*args, **kwargs):
            raise RuntimeError("disk full")

        with mock.patch.object(boot_entry, "_grub_editenv", side_effect=boom):
            with self.assertRaises(RuntimeError):
                boot_entry.write_slot_to_grubenv("b")
        self.assertIn("# existing block", env_path.read_text())

    @mock.patch.object(boot_entry, "_grub_editenv")
    def test_sync_entries_sets_active_slot(self, mock_editenv):
        root_ab.write_active_slot("b")
        self._make_slot_boot_dir("a", "vmlinuz-a", "initramfs-a.img")
        self._make_slot_boot_dir("b", "vmlinuz-b", "initramfs-b.img")
        boot_entry.sync_entries()
        mock_editenv.assert_called_once_with(["set", "regicide_slot=b"])

    def test_sync_entries_discovers_both_slots(self):
        root_ab.write_active_slot("b")
        self._make_slot_boot_dir("a", "vmlinuz-a", "initramfs-a.img")
        self._make_slot_boot_dir("b", "vmlinuz-b", "initramfs-b.img")
        boot_entry.ensure_grub_cfg()
        # Real discovery; should not raise.
        boot_entry.sync_entries()

    def test_install_and_sync_activates_slot_and_updates_grub(self):
        from regicide_update import boot_entry as be

        image_path = Path(self.tmpdir.name) / "new-root.tar.xz"
        image_path.write_text("")

        def fake_install_and_activate(image):
            root_ab.CURRENT_FILE.write_text("b\n")
            return "b"

        with mock.patch.object(root_ab, "install_and_activate", side_effect=fake_install_and_activate):
            with mock.patch.object(be, "sync_entries") as mock_sync:
                slot = be.install_and_sync(image_path)
        self.assertEqual(slot, "b")
        mock_sync.assert_called_once_with()

    def test_rollback_and_sync_switches_default(self):
        from regicide_update import boot_entry as be

        root_ab.CURRENT_FILE.write_text("b\n")
        with mock.patch.object(root_ab, "rollback", return_value="a"):
            with mock.patch.object(be, "sync_entries") as mock_sync:
                slot = be.rollback_and_sync()
        self.assertEqual(slot, "a")
        mock_sync.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
