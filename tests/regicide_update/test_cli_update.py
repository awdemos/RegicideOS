import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from regicide_update import boot_entry, cli_update, common as rc


class CliUpdateTransactionTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmpdir.cleanup)
        overlay_dir = self.tmpdir.name
        snapshot_dir = os.path.join(overlay_dir, "snapshots")
        roots_dir = os.path.join(self.tmpdir.name, "roots")
        os.makedirs(snapshot_dir)
        os.makedirs(roots_dir)

        self.patch_overlay = mock.patch.object(rc, "OVERLAY_DIR", overlay_dir)
        self.patch_roots = mock.patch.object(rc, "ROOTS_DIR", roots_dir)
        self.patch_snapshots = mock.patch.object(rc, "SNAPSHOT_DIR", snapshot_dir)
        self.patch_current = mock.patch.object(
            rc, "CURRENT_FILE", os.path.join(overlay_dir, ".regicide-current")
        )
        self.patch_revert = mock.patch.object(
            rc, "REVERT_FLAG", os.path.join(roots_dir, ".regicide-revert")
        )
        self.patch_overlay.start()
        self.patch_roots.start()
        self.patch_snapshots.start()
        self.patch_current.start()
        self.patch_revert.start()
        self.addCleanup(self.patch_overlay.stop)
        self.addCleanup(self.patch_roots.stop)
        self.addCleanup(self.patch_snapshots.stop)
        self.addCleanup(self.patch_current.stop)
        self.addCleanup(self.patch_revert.stop)

        rc.PRETEND = True
        self.execute_patch = mock.patch.object(rc, "execute")
        self.mock_execute = self.execute_patch.start()
        self.addCleanup(self.execute_patch.stop)
        self._fake_counter = 0

    def _create_live_subvolumes(self):
        for subvol in rc.OVERLAY_SUBVOLUMES:
            os.makedirs(os.path.join(rc.OVERLAY_DIR, subvol))

    def _fake_name(self, tag: str) -> str:
        self._fake_counter += 1
        return f"2026-07-24_{self._fake_counter:04d}_{tag}"

    def _make_namespace(self, action: str, **kwargs) -> mock.Mock:
        ns = mock.Mock()
        ns.action = action
        for k, v in kwargs.items():
            setattr(ns, k, v)
        return ns

    @mock.patch("regicide_update.cli_update.subprocess.call")
    def test_run_emerge_rejects_long_option_injection(self, mock_call):
        mock_call.return_value = 0
        rc.PRETEND = False
        with self.assertRaises(SystemExit):
            cli_update.run_emerge("--config-root=/etc")
        mock_call.assert_not_called()

    @mock.patch("regicide_update.cli_update.subprocess.call")
    def test_cmd_install_rejects_option_injection_in_packages(self, mock_call):
        mock_call.return_value = 0
        args = self._make_namespace("install", packages=["--config-root=/etc"], no_rollback=True)
        with self.assertRaises(SystemExit) as ctx:
            cli_update.cmd_install(args)
        self.assertEqual(ctx.exception.code, 1)
        mock_call.assert_not_called()

    @mock.patch("regicide_update.cli_update.subprocess.call")
    def test_cmd_remove_rejects_option_injection_in_packages(self, mock_call):
        mock_call.return_value = 0
        args = self._make_namespace("remove", packages=["--config-root=/etc"], no_rollback=True)
        with self.assertRaises(SystemExit) as ctx:
            cli_update.cmd_remove(args)
        self.assertEqual(ctx.exception.code, 1)
        mock_call.assert_not_called()

    @mock.patch("regicide_update.cli_update.subprocess.call")
    @mock.patch("regicide_update.snapshots.create_snapshot_set")
    @mock.patch("regicide_update.boot_entry.ensure_source_stub")
    def test_transaction_creates_pre_and_post_snapshots_on_success(
        self, _mock_stub, mock_create, mock_call
    ):
        mock_call.return_value = 0
        mock_create.side_effect = [self._fake_name("pre_upgrade"), self._fake_name("post_upgrade")]
        args = self._make_namespace("upgrade", no_rollback=False)
        with self.assertRaises(SystemExit) as ctx:
            cli_update.cmd_upgrade(args)
        self.assertEqual(ctx.exception.code, 0)
        self.assertEqual(mock_create.call_count, 2)

    @mock.patch("regicide_update.cli_update.subprocess.call")
    @mock.patch("regicide_update.snapshots.create_snapshot_set")
    @mock.patch("regicide_update.snapshots.set_revert")
    def test_transaction_schedules_revert_on_failure(
        self, mock_set_revert, mock_create, mock_call
    ):
        pre = self._fake_name("pre_upgrade")
        mock_call.return_value = 1
        mock_create.return_value = pre
        args = self._make_namespace("install", packages=["app-misc/foo"], no_rollback=False)
        with self.assertRaises(SystemExit) as ctx:
            cli_update.cmd_install(args)
        self.assertEqual(ctx.exception.code, 1)
        mock_set_revert.assert_called_once_with(pre)

    @mock.patch("regicide_update.cli_update.subprocess.call")
    @mock.patch("regicide_update.snapshots.create_snapshot_set")
    @mock.patch("regicide_update.snapshots.set_revert")
    def test_transaction_skips_revert_when_no_rollback_flag_set(
        self, mock_set_revert, mock_create, mock_call
    ):
        mock_call.return_value = 1
        mock_create.return_value = self._fake_name("pre_upgrade")
        args = self._make_namespace("remove", packages=["app-misc/foo"], no_rollback=True)
        with self.assertRaises(SystemExit) as ctx:
            cli_update.cmd_remove(args)
        self.assertEqual(ctx.exception.code, 1)
        mock_set_revert.assert_not_called()

    @mock.patch("regicide_update.cli_update.subprocess.call")
    def test_cmd_sync_runs_emerge_sync(self, mock_call):
        mock_call.return_value = 0
        args = self._make_namespace("sync")
        with self.assertRaises(SystemExit) as ctx:
            cli_update.cmd_sync(args)
        self.assertEqual(ctx.exception.code, 0)
        mock_call.assert_called_once_with(["emerge", "--sync"])

    @mock.patch("regicide_update.cli_update.subprocess.call")
    def test_cmd_search_runs_emerge_search(self, mock_call):
        mock_call.return_value = 0
        args = self._make_namespace("search", query="firefox")
        with self.assertRaises(SystemExit) as ctx:
            cli_update.cmd_search(args)
        self.assertEqual(ctx.exception.code, 0)
        mock_call.assert_called_once_with(["emerge", "-s", "firefox"])

    @mock.patch("regicide_update.cli_update.subprocess.call")
    def test_cmd_search_rejects_shell_metacharacters(self, mock_call):
        mock_call.return_value = 0
        args = self._make_namespace("search", query="foo; rm -rf /")
        with self.assertRaises(SystemExit) as ctx:
            cli_update.cmd_search(args)
        self.assertEqual(ctx.exception.code, 1)
        mock_call.assert_not_called()

    @mock.patch("regicide_update.cli_update.subprocess.call")
    def test_run_emerge_accepts_allowlisted_options(self, mock_call):
        mock_call.return_value = 0
        rc.PRETEND = False
        code = cli_update.run_emerge("-uDU", "@world")
        self.assertEqual(code, 0)
        mock_call.assert_called_once_with(["emerge", "-uDU", "@world"])

    @mock.patch("regicide_update.cli_update.subprocess.call")
    def test_run_emerge_rejects_single_dash_option_injection(self, mock_call):
        mock_call.return_value = 0
        rc.PRETEND = False
        with self.assertRaises(SystemExit):
            cli_update.run_emerge("-C", "app-misc/foo")
        mock_call.assert_not_called()

    @mock.patch("regicide_update.cli_update.subprocess.call")
    @mock.patch("regicide_update.boot_entry.ensure_source_stub")
    def test_cmd_install_passes_validated_packages_to_emerge(self, _mock_stub, mock_call):
        mock_call.return_value = 0
        args = self._make_namespace("install", packages=["app-misc/foo"], no_rollback=True)
        with self.assertRaises(SystemExit):
            cli_update.cmd_install(args)
        mock_call.assert_called_once_with(["emerge", "app-misc/foo"])

    @mock.patch("regicide_update.cli_update.subprocess.call")
    @mock.patch("os.geteuid", return_value=0)
    def test_main_requires_arguments(self, _mock_root, mock_call):
        mock_call.return_value = 0
        with self.assertRaises(SystemExit) as ctx:
            cli_update.main()
        self.assertEqual(ctx.exception.code, 2)

    def test_grub_mkconfig_and_ab_dispatch_do_not_stomp_each_other(self):
        """Interleave maybe_refresh_bootloader and ensure_grub_cfg; both survive."""
        boot_dir = Path(self.tmpdir.name) / "boot"
        with mock.patch.object(boot_entry, "ESP_BOOT_DIR", boot_dir):
            grub_cfg = boot_entry._grub_cfg()
            grub_cfg.parent.mkdir(parents=True, exist_ok=True)

            # 1. A/B update writes the dispatch and stubs grub.cfg.
            boot_entry.ensure_grub_cfg()
            dispatch = boot_entry._grub_dispatch()
            self.assertTrue(dispatch.is_file())
            self.assertIn("source $prefix/regicide.cfg", grub_cfg.read_text())

            # 2. A kernel transaction runs grub-mkconfig, which rewrites
            #    grub.cfg from scratch (stub line destroyed).
            def fake_mkconfig(program, args, **kwargs):
                if program == "grub-mkconfig":
                    grub_cfg.write_text("set default=0\n# generated by grub-mkconfig\n")

            with mock.patch.object(rc, "execute", side_effect=fake_mkconfig):
                with mock.patch.object(
                    cli_update, "_latest_kernel_version", return_value="6.8.0"
                ):
                    cli_update.maybe_refresh_bootloader(["sys-kernel/foo"])
            text = grub_cfg.read_text()
            self.assertIn("# generated by grub-mkconfig", text)
            self.assertIn("source $prefix/regicide.cfg", text)
            self.assertIn(
                boot_entry._DISPATCH_MARKER, dispatch.read_text()
            )

            # 3. The next A/B sync sees the mkconfig output and the stub and
            #    must keep both.
            boot_entry.ensure_grub_cfg()
            text = grub_cfg.read_text()
            self.assertIn("# generated by grub-mkconfig", text)
            self.assertIn("source $prefix/regicide.cfg", text)


if __name__ == "__main__":
    unittest.main()
