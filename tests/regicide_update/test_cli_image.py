import os
import sys
import tempfile
import unittest
import http.server
import threading
from pathlib import Path
from unittest import mock

from regicide_update import cli_image, common as rc, image


class _QuietHandler(http.server.BaseHTTPRequestHandler):
    _root: str = ""

    def log_message(self, *args, **kwargs):
        pass

    def do_GET(self):
        root = _QuietHandler._root
        local = Path(root) / Path(self.path.lstrip("/")).name
        if local.exists():
            self.send_response(200)
            self.send_header("Content-Length", str(local.stat().st_size))
            self.end_headers()
            self.wfile.write(local.read_bytes())
        else:
            self.send_response(404)
            self.end_headers()


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

        root = Path(self.tmpdir.name) / "remote"
        root.mkdir()
        _QuietHandler._root = str(root)
        self.server = http.server.HTTPServer(("127.0.0.1", 0), _QuietHandler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.addCleanup(self._stop_server)
        self.base_url = f"http://127.0.0.1:{self.server.server_port}"

    def _stop_server(self):
        self.server.shutdown()
        self.server.server_close()

    def _write_remote(self, name, data):
        path = Path(self.tmpdir.name) / "remote" / name
        path.write_bytes(data)
        return f"{self.base_url}/{name}"

    @mock.patch("os.geteuid", return_value=0)
    def test_fetch_command_downloads_and_caches(self, _mock_root):
        url = self._write_remote("release.tar.xz", b"image data")
        with mock.patch.object(sys, "argv", ["regicide-image", "fetch", url]):
            cli_image.main()
        cached = image.CACHE_DIR / "release.tar.xz"
        self.assertEqual(cached.read_bytes(), b"image data")

    @mock.patch("os.geteuid", return_value=0)
    @mock.patch("regicide_update.image.install_tarball")
    def test_install_command_passes_path_to_install_tarball(self, mock_install, _mock_root):
        image_path = Path(self.tmpdir.name) / "release.tar.xz"
        image_path.write_text("")
        with mock.patch.object(sys, "argv", ["regicide-image", "install", str(image_path)]):
            cli_image.main()
        mock_install.assert_called_once_with(image_path.resolve(), "/roots", True)

    @mock.patch("os.geteuid", return_value=0)
    @mock.patch("regicide_update.boot_entry.install_and_sync")
    def test_install_ab_command_uses_boot_entry_install_and_sync(self, mock_install_and_sync, _mock_root):
        image_path = Path(self.tmpdir.name) / "release.tar.xz"
        image_path.write_text("")
        mock_install_and_sync.return_value = "b"
        with mock.patch.object(sys, "argv", ["regicide-image", "install", "--ab", str(image_path)]):
            cli_image.main()
        mock_install_and_sync.assert_called_once_with(image_path.resolve())

    @mock.patch("os.geteuid", return_value=0)
    @mock.patch("regicide_update.boot_entry.rollback_and_sync")
    def test_rollback_command_calls_rollback_and_sync(self, mock_rollback, _mock_root):
        mock_rollback.return_value = "a"
        with mock.patch.object(sys, "argv", ["regicide-image", "rollback"]):
            cli_image.main()
        mock_rollback.assert_called_once_with()

    @mock.patch("os.geteuid", return_value=0)
    def test_verify_command_checks_checksum(self, _mock_root):
        image_path = image.CACHE_DIR / "release.tar.xz"
        image_path.write_bytes(b"image data")
        expected = __import__("hashlib").sha256(image_path.read_bytes()).hexdigest()
        checksum_name = "checksums-release.tar.xz.sha256"
        checksum_url = self._write_remote(checksum_name, f"{expected}  {image_path.name}\n".encode())
        with mock.patch.object(sys, "argv", ["regicide-image", "verify", str(image_path), "--checksum-url", checksum_url]):
            cli_image.main()


if __name__ == "__main__":
    unittest.main()
