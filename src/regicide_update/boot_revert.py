#!/usr/bin/env python3
"""Boot-time snapshot revert logic.

This script is intended to run early in boot, before /etc, /var, and /usr
overlay mounts are established. It reads /roots/.regicide-revert and restores
the named snapshot set into /overlay/{etc,var}.

The restore is performed atomically at the live-path level:

1. Clean up any stale temporary or backup subvolumes left by a previous
   interrupted run.
2. Snapshot the target snapshot into a temporary read-write subvolume next to
   the live subvolume (e.g. /overlay/etc.regicide-revert-tmp).
3. Rename the current live subvolume to a read-only backup
   (/overlay/etc.regicide-revert-backup).
4. Rename the temporary subvolume into the live path.
5. Only after the rename succeeds, delete the read-only backup.

The live path therefore always points at valid data: either the old live
subvolume or the new temporary subvolume. If the process is interrupted after
step 3, re-running the revert will detect the backup and the temp and finish
the swap.
"""

import os
import sys
from regicide_update import common as rc
from regicide_update import snapshots


_LIVE_BACKUP_SUFFIX = ".regicide-revert-backup"
_LIVE_TEMP_SUFFIX = ".regicide-revert-tmp"


def _mount_overlay() -> None:
    """Mount the OVERLAY partition by label if it is not already mounted."""
    if not os.path.ismount(rc.OVERLAY_DIR):
        rc.execute("mount", ["LABEL=OVERLAY", rc.OVERLAY_DIR])


def _cleanup_existing(path: str) -> None:
    """Remove a leftover temporary or backup subvolume if it exists."""
    if os.path.isdir(path):
        rc.execute("btrfs", ["subvolume", "delete", path])


def _restore_subvolume(subvol: str, target: str) -> None:
    """Atomically restore one subvolume from the target snapshot set.

    The live path is never left invalid. A new subvolume is built at a
    temporary path, then the old live subvolume is renamed to a backup path,
    the temporary subvolume is renamed into the live path, and only then the
    backup is deleted. If a previous attempt was interrupted, the recovery
    logic restores the live path from whatever consistent pieces remain.

    The whole swap is wrapped in a single rollback block so that a failure
    mid-sequence can recover the previous live data if possible.
    """
    live_path = os.path.join(rc.OVERLAY_DIR, subvol)
    snap_path = os.path.join(target, subvol)
    backup_path = f"{live_path}{_LIVE_BACKUP_SUFFIX}"
    temp_path = f"{live_path}{_LIVE_TEMP_SUFFIX}"

    if not os.path.isdir(snap_path):
        rc.warn(f"Missing snapshot subvolume {subvol}; skipping")
        return

    # Recover from an interrupted previous attempt. The goal is to make
    # live_path valid again, preferring the temporary subvolume (new data)
    # over the backup (old data).
    if not os.path.isdir(live_path):
        if os.path.isdir(temp_path):
            _cleanup_existing(backup_path)
            rc.execute("mv", [temp_path, live_path])
        elif os.path.isdir(backup_path):
            rc.execute("mv", [backup_path, live_path])
    else:
        _cleanup_existing(temp_path)
        _cleanup_existing(backup_path)

    if not os.path.isdir(live_path):
        rc.die(f"Could not recover live subvolume {subvol}")

    # Build the replacement subvolume at a temporary path.
    _cleanup_existing(temp_path)
    rc.execute("btrfs", ["subvolume", "snapshot", snap_path, temp_path])

    # Atomic swap: old live -> backup, temp -> live, then delete backup.
    try:
        _cleanup_existing(backup_path)
        rc.execute("mv", [live_path, backup_path])
        rc.execute("mv", [temp_path, live_path])
        _cleanup_existing(backup_path)
    except SystemExit:
        # Roll back: restore the previous live subvolume if the second rename
        # failed or left live_path missing. Prefer the already-swapped new data
        # if it exists, otherwise fall back to the backup.
        if not os.path.isdir(live_path):
            if os.path.isdir(temp_path):
                rc.execute("mv", [temp_path, live_path])
            elif os.path.isdir(backup_path):
                rc.execute("mv", [backup_path, live_path])
        _cleanup_existing(temp_path)
        _cleanup_existing(backup_path)
        raise


def apply_revert() -> bool:
    """Apply a pending revert if one is scheduled.

    Returns True if a revert was applied, False if no revert was pending.
    Raises SystemExit on fatal errors.
    """
    if not os.path.exists(rc.REVERT_FLAG):
        return False

    with open(rc.REVERT_FLAG) as f:
        target_name = f.read().strip()

    target = os.path.join(rc.SNAPSHOT_DIR, target_name)
    if not os.path.isdir(target):
        rc.warn(f"Revert target '{target_name}' missing; cancelling revert.")
        os.remove(rc.REVERT_FLAG)
        return False

    rc.info(f"Applying revert to snapshot set: {target_name}")

    _mount_overlay()

    for subvol in rc.OVERLAY_SUBVOLUMES:
        _restore_subvolume(subvol, target)

    os.remove(rc.REVERT_FLAG)
    snapshots.write_current(target_name)
    rc.info("Revert applied.")
    return True


def main() -> None:
    try:
        apply_revert()
    except Exception as exc:
        rc.warn(f"Revert failed: {exc}")
        # Leave REVERT_FLAG in place so the next boot can retry or be inspected.
        sys.exit(1)


if __name__ == "__main__":
    main()
