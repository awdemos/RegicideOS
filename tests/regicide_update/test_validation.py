import os
import tempfile
import unittest
from pathlib import Path

from regicide_update import validation


class ValidationTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)

    def test_safe_url_allows_https(self):
        self.assertEqual(validation.safe_url("https://example.com/img.tar.xz"), "https://example.com/img.tar.xz")

    def test_safe_url_rejects_file_scheme(self):
        with self.assertRaises(SystemExit):
            validation.safe_url("file:///etc/passwd")

    def test_safe_url_rejects_missing_scheme(self):
        with self.assertRaises(SystemExit):
            validation.safe_url("example.com/img.tar.xz")

    def test_safe_url_rejects_newline(self):
        with self.assertRaises(SystemExit):
            validation.safe_url("https://example.com/img\n/etc/passwd")

    def test_safe_path_requires_absolute(self):
        with self.assertRaises(SystemExit):
            validation.safe_path("relative/path")

    def test_safe_path_enforces_prefix(self):
        root = Path(self.tmpdir.name) / "roots"
        root.mkdir()
        good = root / "slot"
        good.mkdir()
        self.assertEqual(
            validation.safe_path(str(good), allowed_prefixes=(str(root),)),
            good.resolve(),
        )

    def test_safe_path_rejects_traversal(self):
        root = Path(self.tmpdir.name) / "roots"
        root.mkdir()
        with self.assertRaises(SystemExit):
            validation.safe_path(str(root / ".." / "etc"), allowed_prefixes=(str(root),))

    def test_safe_package_name_accepts_atom(self):
        self.assertEqual(validation.safe_package_name("app-misc/foo"), "app-misc/foo")

    def test_safe_package_name_accepts_set(self):
        self.assertEqual(validation.safe_package_name("@world"), "@world")

    def test_safe_package_name_rejects_option(self):
        with self.assertRaises(SystemExit):
            validation.safe_package_name("--config-root=/etc")

    def test_safe_package_name_rejects_traversal(self):
        with self.assertRaises(SystemExit):
            validation.safe_package_name("../foo")

    def test_safe_snapshot_name_accepts_valid(self):
        self.assertEqual(validation.safe_snapshot_name("pre_upgrade"), "pre_upgrade")

    def test_safe_snapshot_name_rejects_empty(self):
        with self.assertRaises(SystemExit):
            validation.safe_snapshot_name("")

    def test_safe_snapshot_name_rejects_path_traversal(self):
        with self.assertRaises(SystemExit):
            validation.safe_snapshot_name("../etc")

    def test_safe_slot_accepts_a_and_b(self):
        self.assertEqual(validation.safe_slot("A"), "a")
        self.assertEqual(validation.safe_slot("b"), "b")

    def test_safe_slot_rejects_invalid(self):
        with self.assertRaises(SystemExit):
            validation.safe_slot("c")

    def test_safe_shell_arg_rejects_metacharacters(self):
        with self.assertRaises(SystemExit):
            validation.safe_shell_arg("foo; rm -rf /")

    def test_safe_shell_arg_accepts_plain(self):
        self.assertEqual(validation.safe_shell_arg("firefox"), "firefox")


if __name__ == "__main__":
    unittest.main()
