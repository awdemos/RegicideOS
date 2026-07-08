#!/usr/bin/env python3
"""CLI for fetching and installing release images."""

import argparse
from pathlib import Path
from regicide_update import common as rc
from regicide_update import image


def cmd_fetch(args: argparse.Namespace) -> None:
    path = image.fetch(args.url)
    if args.checksum_url:
        image.verify_checksum(path, args.checksum_url)
    rc.info(f"Image cached at {path}")


def cmd_install(args: argparse.Namespace) -> None:
    path = Path(args.path)
    if not path.is_file():
        rc.die(f"Image not found: {path}")
    image.install_tarball(path, args.roots_mount, args.reseed)


def cmd_verify(args: argparse.Namespace) -> None:
    image.verify_checksum(Path(args.path), args.checksum_url)


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

    verify = sub.add_parser("verify", help="Verify a downloaded image checksum")
    verify.add_argument("path")
    verify.add_argument("--checksum-url", required=True)

    args = parser.parse_args()
    match args.action:
        case "fetch":
            cmd_fetch(args)
        case "install":
            cmd_install(args)
        case "verify":
            cmd_verify(args)


if __name__ == "__main__":
    main()
