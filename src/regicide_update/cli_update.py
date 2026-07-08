#!/usr/bin/env python3
"""CLI for Portage-based update operations with snapshot safety."""

import argparse
import subprocess
import sys
from regicide_update import snapshots, common as rc


def run_emerge(*args: str) -> int:
    cmd = ["emerge"] + list(args)
    rc.info("Running: " + " ".join(cmd))
    return subprocess.call(cmd)


def cmd_sync(_args: argparse.Namespace) -> None:
    sys.exit(run_emerge("--sync"))


def cmd_search(args: argparse.Namespace) -> None:
    sys.exit(run_emerge("-s", args.query))


def _transaction(args: argparse.Namespace, tag_prefix: str, emerge_args: list[str]) -> None:
    pre = snapshots.create_snapshot_set(f"pre_{tag_prefix}")
    rc.info(f"Pre-transaction snapshot: {pre}")
    code = run_emerge(*emerge_args)
    if code == 0:
        post = snapshots.create_snapshot_set(f"post_{tag_prefix}")
        rc.info(f"Post-transaction snapshot: {post}")
        snapshots.apply_retention()
    else:
        rc.warn(f"{tag_prefix.capitalize()} failed.")
        if not args.no_rollback:
            snapshots.set_revert(pre)
            rc.warn(f"Reboot to roll back to {pre}.")
    sys.exit(code)


def cmd_upgrade(args: argparse.Namespace) -> None:
    _transaction(args, "upgrade", ["-uDU", "@world"])


def cmd_install(args: argparse.Namespace) -> None:
    _transaction(args, "install", args.packages)


def cmd_remove(args: argparse.Namespace) -> None:
    _transaction(args, "remove", ["--unmerge", *args.packages])


def main() -> None:
    rc.require_root()
    parser = argparse.ArgumentParser(prog="regicide-update")
    sub = parser.add_subparsers(dest="action", required=True)

    sub.add_parser("sync", help="Sync Portage repositories")

    search = sub.add_parser("search", help="Search packages")
    search.add_argument("query")

    upgrade = sub.add_parser("upgrade", help="Upgrade installed packages")
    upgrade.add_argument("--no-rollback", action="store_true")

    install = sub.add_parser("install", help="Install packages")
    install.add_argument("packages", nargs="+")
    install.add_argument("--no-rollback", action="store_true")

    remove = sub.add_parser("remove", help="Remove packages")
    remove.add_argument("packages", nargs="+")
    remove.add_argument("--no-rollback", action="store_true")

    args = parser.parse_args()
    match args.action:
        case "sync":
            cmd_sync(args)
        case "search":
            cmd_search(args)
        case "upgrade":
            cmd_upgrade(args)
        case "install":
            cmd_install(args)
        case "remove":
            cmd_remove(args)


if __name__ == "__main__":
    main()
