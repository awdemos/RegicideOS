import hashlib
import http.server
import os
import tempfile
import threading
import unittest
from pathlib import Path
from unittest import mock

from regicide_update import common as rc, image


class ImageTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        self.cache_patch = mock.patch.object(image, "CACHE_DIR", Path(self.tmpdir.name))
        self.cache_patch.start()
        self.addCleanup(self.cache_patch.stop)
        rc.PRETEND = True

    def _serve(self, path_map: dict[str, bytes]) -> str:
        """Start a thread-local HTTP server and return its base URL."""
        root = Path(self.tmpdir.name)
        for name, data in path_map.items():
            (root / name).write_bytes(data)

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self2):
                # Serve from root; require exact path match
                rel = Path(self2.path.lstrip("/"))
                target = root / rel
                if not target.is_file():
                    self2.send_error(404)
                    return
                self2.send_response(200)
                self2.send_header("Content-Length", str(target.stat().st_size))
                self2.end_headers()
                with open(target, "rb") as f:
                    while True:
                        chunk = f.read(65536)
                        if not chunk:
                            break
                        self2.wfile.write(chunk)

            def log_message(self2, *args, **kwargs):
                pass

        server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.shutdown)
        return f"http://127.0.0.1:{server.server_port}"

    def test_ensure_cache_creates_directory(self):
        image.ensure_cache()
        self.assertTrue(image.CACHE_DIR.is_dir())

    def test_fetch_downloads_to_cache(self):
        url_root = self._serve({"release.tar.xz": b"image payload"})
        dest = image.fetch(f"{url_root}/release.tar.xz")
        self.assertEqual(dest, image.CACHE_DIR / "release.tar.xz")
        self.assertEqual(dest.read_bytes(), b"image payload")

    def test_fetch_rejects_url_without_filename(self):
        with self.assertRaises(SystemExit):
            image.fetch("https://example.com/regicide/")

    def test_verify_checksum_downloads_checksum_file(self):
        image_file = image.CACHE_DIR / "release.tar.xz"
        image_file.write_text("image data")
        expected = hashlib.sha256(image_file.read_bytes()).hexdigest()
        url_root = self._serve(
            {"checksums.sha256": f"{expected}  {image_file.name}\n".encode()}
        )
        result = image.verify_checksum(image_file, f"{url_root}/checksums.sha256")
        self.assertTrue(result)

    def test_verify_checksum_mismatch_dies(self):
        image_file = image.CACHE_DIR / "release.tar.xz"
        image_file.write_text("image data")
        url_root = self._serve(
            {"checksums.sha256": f"{'0' * 64}  {image_file.name}\n".encode()}
        )
        with self.assertRaises(SystemExit):
            image.verify_checksum(image_file, f"{url_root}/checksums.sha256")

    def test_verify_checksum_skips_when_url_none(self):
        image_file = image.CACHE_DIR / "release.tar.xz"
        self.assertTrue(image.verify_checksum(image_file, None))

    def test_install_tarball_dies_when_not_btrfs(self):
        roots_mount = self.tmpdir.name
        image_file = image.CACHE_DIR / "release.tar.xz"
        image_file.write_text("")
        with mock.patch.object(rc, "is_btrfs", return_value=False):
            with self.assertRaises(SystemExit):
                image.install_tarball(image_file, roots_mount)

    def test_install_tarball_extracts_xz_tarball(self):
        roots_mount = self.tmpdir.name
        image_file = image.CACHE_DIR / "release.tar.xz"
        image_file.write_text("")
        with mock.patch.object(rc, "is_btrfs", return_value=True):
            with mock.patch.object(rc, "execute") as mock_execute:
                image.install_tarball(image_file, roots_mount, reseed=False)
        mock_execute.assert_any_call(
            "tar", ["-C", str(Path(roots_mount).resolve()), "-x", "-p", "-J", "-f", str(image_file)]
        )

    def test_install_tarball_runs_seed_script_when_present(self):
        roots_mount = self.tmpdir.name
        seed_dir = os.path.join(roots_mount, "usr", "lib", "regicide-update")
        os.makedirs(seed_dir)
        seed_script = os.path.join(seed_dir, "seed-overlays.sh")
        Path(seed_script).touch()
        image_file = image.CACHE_DIR / "release.tar"
        image_file.write_text("")
        with mock.patch.object(rc, "is_btrfs", return_value=True):
            with mock.patch.object(rc, "execute") as mock_execute:
                image.install_tarball(image_file, roots_mount, reseed=True)
        seed_script_real = str(Path(seed_script).resolve())
        mock_execute.assert_any_call("bash", [seed_script_real, str(Path(roots_mount).resolve()), "/overlay"])


if __name__ == "__main__":
    unittest.main()
