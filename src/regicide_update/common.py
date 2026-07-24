#!/usr/bin/env python3
"""Shared helpers: logging, shell execution, layout constants."""

import os
import subprocess
import sys
from dataclasses import dataclass

PRETEND = False

OVERLAY_DIR = "/overlay"
ROOTS_DIR = "/roots"
# The snapshot store must live on the SAME btrfs filesystem as the snapshotted
# subvolumes (/overlay/etc, /overlay/var): btrfs cannot snapshot across
# filesystems (EXDEV). /roots is on the ROOTS filesystem, so keep only the
# revert flag there; snapshots live on OVERLAY.
SNAPSHOT_DIR = os.path.join(OVERLAY_DIR, ".regicide-snapshots")
REVERT_FLAG = os.path.join(ROOTS_DIR, ".regicide-revert")
CURRENT_FILE = os.path.join(OVERLAY_DIR, ".regicide-current")

# Gentoo RegicideOS mounts /etc and /var from the OVERLAY partition. /usr lives
# on the immutable ROOTS partition (dracut has trouble with a separate /usr
# mount during switch-root), so it is not snapshotted/restored here.
OVERLAY_SUBVOLUMES = ("etc", "var")


@dataclass(frozen=True)
class Colours:
    red: str = "\033[31m"
    endc: str = "\033[m"
    green: str = "\033[32m"
    yellow: str = "\033[33m"
    blue: str = "\033[34m"


def die(message: str) -> None:
    print(f"{Colours.red}[ERROR]{Colours.endc} {message}", file=sys.stderr)
    sys.exit(1)


def info(message: str) -> None:
    print(f"{Colours.blue}[INFO]{Colours.endc} {message}")


def warn(message: str) -> None:
    print(f"{Colours.yellow}[WARN]{Colours.endc} {message}")


def execute(
    program: str, args: list[str], override: bool = False, check: bool = True
) -> str:
    """Run a command with the given arguments and return stdout."""
    cmd = [program, *args]
    if PRETEND and not override:
        info(f"[COMMAND] {' '.join(cmd)}")
        return ""
    result = subprocess.run(cmd, shell=False, capture_output=True, text=True)
    if check and result.returncode != 0:
        die(f"Command failed: {' '.join(cmd)}\n{result.stderr}")
    return result.stdout


def require_root() -> None:
    if os.geteuid() != 0:
        die("This command requires root privileges.")


def is_btrfs(path: str) -> bool:
    result = subprocess.run(
        ["stat", "-f", "--format=%T", path],
        shell=False,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0 and "btrfs" in result.stdout
