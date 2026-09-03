#!/usr/bin/env python3
"""CLI for fetching and installing release images."""

import argparse
from pathlib import Path
from regicide_update import common as rc
from regicide_update import image
from regicide_update import validation


def cmd_fetch(args: argparse.Namespace) -> None:
    path = image.fetch(validation.safe_url(args.url))
    if args.checksum_url:
        image.verify_checksum(path, validation.safe_url(args.checksum_url))
    rc.info(f"Image cached at {path}")


def cmd_install(args: argparse.Namespace) -> None:
    path = validation.safe_path(args.path, must_exist=True)
    roots_mount = validation.safe_path(args.roots_mount, must_be_absolute=True)
    if args.ab:
        from regicide_update import boot_entry
        slot = boot_entry.install_and_sync(path)
        rc.info(f"A/B root update staged in slot {slot}; reboot to boot into it.")
    else:
        image.install_tarball(path, str(roots_mount), args.reseed)


def cmd_verify(args: argparse.Namespace) -> None:
    image.verify_checksum(validation.safe_path(args.path, must_exist=True), args.checksum_url)


def cmd_rollback(_args: argparse.Namespace) -> None:
    from regicide_update import boot_entry
    slot = boot_entry.rollback_and_sync()
    rc.info(f"Rollback staged to slot {slot}; reboot to activate.")


def main() -> None:
    rc.require_root()
    parser = argparse.ArgumentParser(prog="regicide-image")
    sub = parser.add_subparsers(dest="action", required=True)

    fetch = sub.add_parser("fetch", help="Download a release image")
    fetch.add_argument("url")
    fetch.add_argument("--checksum-url")

    install = sub.add_parser("install", help="Install a tarball into ROOTS")
    install.add_argument("path")
    install.add_argument("--roots-mount", default="/roots")
    install.add_argument("--reseed", action="store_true", default=True)
    install.add_argument(
        "--ab", action="store_true", help="Install into the inactive A/B root slot"
    )

    verify = sub.add_parser("verify", help="Verify a downloaded image checksum")
    verify.add_argument("path")
    verify.add_argument("--checksum-url", required=True)

    sub.add_parser("rollback", help="Rollback to the previous A/B root slot")

    args = parser.parse_args()
    match args.action:
        case "fetch":
            cmd_fetch(args)
        case "install":
            cmd_install(args)
        case "verify":
            cmd_verify(args)
        case "rollback":
            cmd_rollback(args)


if __name__ == "__main__":
    main()
