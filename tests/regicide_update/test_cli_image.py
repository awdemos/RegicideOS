import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from regicide_update import cli_image, common as rc, image


class CliImageTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        self.cache_patch = mock.patch.object(image, "CACHE_DIR", Path(self.tmpdir.name))
        self.cache_patch.start()
        self.addCleanup(self.cache_patch.stop)

        self._orig_pretend = rc.PRETEND
        rc.PRETEND = True
        self.addCleanup(setattr, rc, "PRETEND", self._orig_pretend)

    @mock.patch("os.geteuid", return_value=0)
    @mock.patch("urllib.request.urlretrieve")
    def test_fetch_command_downloads_and_caches(self, mock_retrieve, _mock_root):
        mock_retrieve.return_value = (str(image.CACHE_DIR / "release.tar.xz"), None)
        with mock.patch.object(sys, "argv", ["regicide-image", "fetch", "https://example.com/release.tar.xz"]):
            cli_image.main()
        mock_retrieve.assert_called_once()

    @mock.patch("os.geteuid", return_value=0)
    @mock.patch("regicide_update.image.install_tarball")
    def test_install_command_passes_path_to_install_tarball(self, mock_install, _mock_root):
        image_path = Path(self.tmpdir.name) / "release.tar.xz"
        image_path.write_text("")
        with mock.patch.object(sys, "argv", ["regicide-image", "install", str(image_path)]):
            cli_image.main()
        mock_install.assert_called_once_with(image_path, "/roots", True)

    @mock.patch("os.geteuid", return_value=0)
    @mock.patch("regicide_update.boot_entry.install_and_sync")
    def test_install_ab_command_uses_boot_entry_install_and_sync(self, mock_install_and_sync, _mock_root):
        image_path = Path(self.tmpdir.name) / "release.tar.xz"
        image_path.write_text("")
        mock_install_and_sync.return_value = "b"
        with mock.patch.object(sys, "argv", ["regicide-image", "install", "--ab", str(image_path)]):
            cli_image.main()
        mock_install_and_sync.assert_called_once_with(image_path)

    @mock.patch("os.geteuid", return_value=0)
    @mock.patch("regicide_update.boot_entry.rollback_and_sync")
    def test_rollback_command_calls_rollback_and_sync(self, mock_rollback, _mock_root):
        mock_rollback.return_value = "a"
        with mock.patch.object(sys, "argv", ["regicide-image", "rollback"]):
            cli_image.main()
        mock_rollback.assert_called_once_with()

    @mock.patch("os.geteuid", return_value=0)
    @mock.patch("urllib.request.urlretrieve")
    def test_verify_command_checks_checksum(self, mock_retrieve, _mock_root):
        image_path = image.CACHE_DIR / "release.tar.xz"
        image_path.write_text("image data")
        expected = __import__("hashlib").sha256(image_path.read_bytes()).hexdigest()
        checksum_file = image.CACHE_DIR / "checksums-release.tar.xz.sha256"
        checksum_file.write_text(f"{expected}  {image_path.name}\n")
        mock_retrieve.return_value = (str(checksum_file), None)
        with mock.patch.object(sys, "argv", ["regicide-image", "verify", str(image_path), "--checksum-url", "https://example.com/checksums.sha256"]):
            cli_image.main()


if __name__ == "__main__":
    unittest.main()
