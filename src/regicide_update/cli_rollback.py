#!/usr/bin/env python3
"""CLI for snapshot/rollback operations."""

import argparse
from regicide_update import snapshots, common as rc


def cmd_list(_args: argparse.Namespace) -> None:
    sets = snapshots.list_snapshot_sets()
    current = snapshots.read_current()
    print("Snapshot sets:")
    for name, mtime in sets:
        marker = " <-- current" if name == current else ""
        print(f"  {name} ({mtime}){marker}")


def cmd_create(args: argparse.Namespace) -> None:
    name = snapshots.create_snapshot_set(args.tag)
    rc.info(f"Created snapshot set: {name}")


def cmd_delete(args: argparse.Namespace) -> None:
    snapshots.delete_snapshot_set(args.name)
    rc.info(f"Deleted snapshot set: {args.name}")


def cmd_revert(args: argparse.Namespace) -> None:
    if args.cancel:
        snapshots.cancel_revert()
        return
    if not args.name:
        rc.die("Please specify a snapshot set name or --cancel.")
    snapshots.set_revert(args.name)


def cmd_current(_args: argparse.Namespace) -> None:
    print(snapshots.read_current())


def main() -> None:
    rc.require_root()
    parser = argparse.ArgumentParser(prog="regicide-rollback")
    sub = parser.add_subparsers(dest="action", required=True)

    sub.add_parser("list", help="List snapshot sets")

    create = sub.add_parser("create", help="Create a manual snapshot set")
    create.add_argument("--tag", default="manual")

    delete = sub.add_parser("delete", help="Delete a snapshot set")
    delete.add_argument("name")

    revert = sub.add_parser("revert", help="Revert at next boot")
    revert.add_argument("name", nargs="?")
    revert.add_argument("--cancel", action="store_true")

    sub.add_parser("current", help="Show current snapshot set")

    args = parser.parse_args()
    match args.action:
        case "list":
            cmd_list(args)
        case "create":
            cmd_create(args)
        case "delete":
            cmd_delete(args)
        case "revert":
            cmd_revert(args)
        case "current":
            cmd_current(args)


if __name__ == "__main__":
    main()
