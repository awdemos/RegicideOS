import hashlib
import os
import tempfile
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

    def test_ensure_cache_creates_directory(self):
        image.ensure_cache()
        self.assertTrue(image.CACHE_DIR.is_dir())

    @mock.patch("urllib.request.urlretrieve")
    def test_fetch_downloads_to_cache(self, mock_retrieve):
        mock_retrieve.return_value = (str(image.CACHE_DIR / "release.tar.xz"), None)
        dest = image.fetch("https://example.com/regicide/release.tar.xz")
        self.assertEqual(dest, image.CACHE_DIR / "release.tar.xz")
        mock_retrieve.assert_called_once_with(
            "https://example.com/regicide/release.tar.xz", dest, timeout=300
        )

    def test_fetch_rejects_url_without_filename(self):
        with self.assertRaises(SystemExit):
            image.fetch("https://example.com/regicide/")

    @mock.patch("urllib.request.urlretrieve")
    def test_verify_checksum_downloads_checksum_file(self, mock_retrieve):
        image_file = image.CACHE_DIR / "release.tar.xz"
        image_file.write_text("image data")
        expected = hashlib.sha256(image_file.read_bytes()).hexdigest()
        checksum_file = image.CACHE_DIR / "checksums-release.tar.xz.sha256"
        checksum_file.write_text(f"{expected}  {image_file.name}\n")
        result = image.verify_checksum(image_file, "https://example.com/checksums.sha256")
        self.assertTrue(result)
        mock_retrieve.assert_called_once_with(
            "https://example.com/checksums.sha256", checksum_file, timeout=60
        )

    @mock.patch("urllib.request.urlretrieve")
    def test_verify_checksum_mismatch_dies(self, mock_retrieve):
        image_file = image.CACHE_DIR / "release.tar.xz"
        image_file.write_text("image data")
        checksum_file = image.CACHE_DIR / "checksums-release.tar.xz.sha256"
        checksum_file.write_text(f"{'0' * 64}  {image_file.name}\n")
        with self.assertRaises(SystemExit):
            image.verify_checksum(image_file, "https://example.com/checksums.sha256")

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
            "tar", ["-C", roots_mount, "-x", "-p", "-J", "-f", str(image_file)]
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
        mock_execute.assert_any_call("bash", [seed_script, roots_mount, "/overlay"])


if __name__ == "__main__":
    unittest.main()
