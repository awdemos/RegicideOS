#!/usr/bin/env python3
"""Snapshot set creation, listing, deletion, and revert scheduling."""

import os
import time
from datetime import datetime
from regicide_update import common as rc


def _make_name(tag: str) -> str:
    return f"{datetime.now().strftime('%Y-%m-%d_%H:%M:%S')}_{tag}"


def ensure_snapshot_dir() -> None:
    if not os.path.isdir(rc.SNAPSHOT_DIR):
        rc.execute(f"btrfs subvolume create {rc.SNAPSHOT_DIR}")


def list_snapshot_sets() -> list[tuple[str, str]]:
    if not os.path.isdir(rc.SNAPSHOT_DIR):
        return []
    names: list[tuple[str, str]] = []
    for name in os.listdir(rc.SNAPSHOT_DIR):
        path = os.path.join(rc.SNAPSHOT_DIR, name)
        if os.path.isdir(path) and all(
            os.path.isdir(os.path.join(path, subvol)) for subvol in rc.OVERLAY_SUBVOLUMES
        ):
            names.append((name, time.ctime(os.stat(path).st_mtime)))
    return sorted(names, key=lambda x: x[0])


def write_current(name: str) -> None:
    os.makedirs(rc.OVERLAY_DIR, exist_ok=True)
    with open(rc.CURRENT_FILE, "w") as f:
        f.write(f"{name}\n{datetime.now().isoformat()}\n")


def read_current() -> str:
    if not os.path.isfile(rc.CURRENT_FILE):
        return "unknown"
    with open(rc.CURRENT_FILE) as f:
        return f.readline().strip()


def create_snapshot_set(tag: str = "manual") -> str:
    ensure_snapshot_dir()
    name = _make_name(tag)
    target = os.path.join(rc.SNAPSHOT_DIR, name)
    os.makedirs(target, exist_ok=False)
    for subvol in rc.OVERLAY_SUBVOLUMES:
        src = os.path.join(rc.OVERLAY_DIR, subvol)
        dst = os.path.join(target, subvol)
        if os.path.isdir(src):
            rc.execute(f"btrfs subvolume snapshot -r {src} {dst}")
    write_current(name)
    return name


def delete_snapshot_set(name: str) -> None:
    target = os.path.join(rc.SNAPSHOT_DIR, name)
    if not os.path.isdir(target):
        rc.die(f"Snapshot set '{name}' not found.")
    if name == "initial":
        rc.die("Refusing to delete the 'initial' snapshot set.")
    for subvol in rc.OVERLAY_SUBVOLUMES:
        path = os.path.join(target, subvol)
        if os.path.isdir(path):
            rc.execute(f"btrfs subvolume delete {path}")
    os.rmdir(target)


def set_revert(name: str) -> None:
    target = os.path.join(rc.SNAPSHOT_DIR, name)
    if not os.path.isdir(target):
        rc.die(f"Cannot revert to '{name}': snapshot set does not exist.")
    with open(rc.REVERT_FLAG, "w") as f:
        f.write(name)
    rc.info(f"Revert to '{name}' scheduled. Reboot to apply.")


def cancel_revert() -> None:
    if os.path.exists(rc.REVERT_FLAG):
        os.remove(rc.REVERT_FLAG)
        rc.info("Revert cancelled.")
    else:
        rc.warn("No revert is pending.")


def apply_retention(keep_count: int = 5) -> None:
    sets = list_snapshot_sets()
    protected = {"initial", read_current()}
    candidates = [name for name, _ in sets if name not in protected]
    to_remove = candidates[:-keep_count] if len(candidates) > keep_count else []
    for name in to_remove:
        try:
            delete_snapshot_set(name)
            rc.info(f"Pruned old snapshot set: {name}")
        except SystemExit:
            rc.warn(f"Failed to prune snapshot set: {name}")
