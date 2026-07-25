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

    def test_ensure_dirs_creates_loader_entries(self):
        boot_entry.ensure_dirs()
        self.assertTrue((boot_entry.ESP_BOOT_DIR / "loader" / "entries").is_dir())

    def _make_slot_boot_dir(self, slot: str, kernel: str, initrd: str):
        boot_dir = os.path.join(rc.ROOTS_DIR, f"roots_{slot}", "boot")
        os.makedirs(boot_dir, exist_ok=True)
        Path(os.path.join(boot_dir, kernel)).touch()
        Path(os.path.join(boot_dir, initrd)).touch()

    def test_write_entry_creates_conf_file(self):
        boot_entry.write_entry("a", "/vmlinuz", "/initramfs.img")
        entry = boot_entry.ESP_BOOT_DIR / "loader" / "entries" / "regicide-a.conf"
        self.assertTrue(entry.is_file())
        text = entry.read_text()
        self.assertIn("title RegicideOS A", text)
        self.assertIn("linux /vmlinuz", text)
        self.assertIn("initrd /initramfs.img", text)
        self.assertIn("root=LABEL=ROOTS ro rootflags=subvol=roots_a", text)

    def test_set_default_writes_loader_conf(self):
        boot_entry.write_entry("a", "/vmlinuz", "/initramfs.img")
        boot_entry.write_entry("b", "/vmlinuz", "/initramfs.img")
        boot_entry.set_default("b")
        loader_conf = boot_entry.ESP_BOOT_DIR / "loader" / "loader.conf"
        text = loader_conf.read_text()
        self.assertIn("default regicide-b", text)
        self.assertIn("timeout 5", text)

    def test_discover_kernel_initrd_finds_versioned_names(self):
        self._make_slot_boot_dir("a", "vmlinuz-6.8.0-gentoo", "initramfs-6.8.0-gentoo.img")
        kernel, initrd = boot_entry.discover_kernel_initrd("a")
        self.assertEqual(kernel, "/vmlinuz-6.8.0-gentoo")
        self.assertEqual(initrd, "/initramfs-6.8.0-gentoo.img")

    def test_discover_kernel_initrd_prefers_stable_names(self):
        boot_dir = os.path.join(rc.ROOTS_DIR, "roots_a", "boot")
        os.makedirs(boot_dir, exist_ok=True)
        Path(os.path.join(boot_dir, "vmlinuz-6.8.0-gentoo")).touch()
        Path(os.path.join(boot_dir, "vmlinuz")).touch()
        Path(os.path.join(boot_dir, "initramfs-6.8.0-gentoo.img")).touch()
        Path(os.path.join(boot_dir, "initramfs.img")).touch()
        kernel, initrd = boot_entry.discover_kernel_initrd("a")
        self.assertEqual(kernel, "/vmlinuz")
        self.assertEqual(initrd, "/initramfs.img")

    def test_discover_kernel_initrd_is_deterministic_with_many_files(self):
        boot_dir = os.path.join(rc.ROOTS_DIR, "roots_a", "boot")
        os.makedirs(boot_dir, exist_ok=True)
        for name in ["vmlinuz-6.10.0-gentoo", "vmlinuz-6.8.0-gentoo", "vmlinuz-6.9.0-gentoo"]:
            Path(os.path.join(boot_dir, name)).touch()
        for name in ["initramfs-6.10.0-gentoo.img", "initramfs-6.8.0-gentoo.img", "initramfs-6.9.0-gentoo.img"]:
            Path(os.path.join(boot_dir, name)).touch()
        kernel, initrd = boot_entry.discover_kernel_initrd("a")
        self.assertEqual(kernel, "/vmlinuz-6.8.0-gentoo")
        self.assertEqual(initrd, "/initramfs-6.8.0-gentoo.img")

    def test_write_entry_preserves_old_entry_on_failure(self):
        self._make_slot_boot_dir("a", "vmlinuz-a", "initramfs-a.img")
        boot_entry.write_entry("a", "/vmlinuz-a", "/initramfs-a.img")
        entry = boot_entry.ESP_BOOT_DIR / "loader" / "entries" / "regicide-a.conf"
        original = entry.read_text()

        def boom(*args, **kwargs):
            raise RuntimeError("disk full")

        with mock.patch.object(boot_entry.Path, "write_text", side_effect=boom):
            with self.assertRaises(RuntimeError):
                boot_entry.write_entry("a", "/vmlinuz-b", "/initramfs-b.img")
        self.assertEqual(entry.read_text(), original)
        backup = entry.parent / (entry.name + boot_entry._BACKUP_SUFFIX)
        self.assertFalse(backup.exists())

    def test_set_default_preserves_old_loader_conf_on_failure(self):
        boot_entry.write_entry("a", "/vmlinuz-a", "/initramfs-a.img")
        boot_entry.write_entry("b", "/vmlinuz-b", "/initramfs-b.img")
        boot_entry.set_default("a")
        loader_conf = boot_entry.ESP_BOOT_DIR / "loader" / "loader.conf"
        original = loader_conf.read_text()

        def boom(*args, **kwargs):
            raise RuntimeError("disk full")

        with mock.patch.object(boot_entry.Path, "write_text", side_effect=boom):
            with self.assertRaises(RuntimeError):
                boot_entry.set_default("b")
        self.assertEqual(loader_conf.read_text(), original)
        backup = loader_conf.parent / (loader_conf.name + boot_entry._BACKUP_SUFFIX)
        self.assertFalse(backup.exists())

    def test_sync_entries_defaults_to_active_slot(self):
        root_ab.write_active_slot("b")
        self._make_slot_boot_dir("a", "vmlinuz-a", "initramfs-a.img")
        self._make_slot_boot_dir("b", "vmlinuz-b", "initramfs-b.img")
        boot_entry.sync_entries()
        a_entry = (boot_entry.ESP_BOOT_DIR / "loader" / "entries" / "regicide-a.conf").read_text()
        b_entry = (boot_entry.ESP_BOOT_DIR / "loader" / "entries" / "regicide-b.conf").read_text()
        self.assertIn("linux /vmlinuz-a", a_entry)
        self.assertIn("initrd /initramfs-a.img", a_entry)
        self.assertIn("linux /vmlinuz-b", b_entry)
        self.assertIn("initrd /initramfs-b.img", b_entry)
        loader_conf = (boot_entry.ESP_BOOT_DIR / "loader" / "loader.conf").read_text()
        self.assertIn("default regicide-b", loader_conf)

    def test_install_and_sync_activates_slot_and_writes_entries(self):
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
