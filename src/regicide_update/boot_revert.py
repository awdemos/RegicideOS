#!/usr/bin/env python3
"""Boot-time snapshot revert logic.

This script is intended to run early in boot, before /etc, /var, and /usr
overlay mounts are established. It reads /roots/.regicide-revert and restores
the named snapshot set into /overlay/{etc,var,usr}.
"""

import os
import sys
from regicide_update import common as rc
from regicide_update import snapshots


def apply_revert() -> bool:
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

    overlay_mounted = os.path.ismount(rc.OVERLAY_DIR)
    if not overlay_mounted:
        rc.execute(f"mount LABEL=OVERLAY {rc.OVERLAY_DIR}")

    for subvol in rc.OVERLAY_SUBVOLUMES:
        live_path = os.path.join(rc.OVERLAY_DIR, subvol)
        snap_path = os.path.join(target, subvol)
        if not os.path.isdir(snap_path):
            rc.warn(f"Missing snapshot subvolume {subvol}; skipping")
            continue
        if os.path.isdir(live_path):
            rc.execute(f"btrfs subvolume delete {live_path}")
        rc.execute(f"btrfs subvolume create {live_path}")
        rc.execute(f"cp -aT --reflink=auto {snap_path} {live_path}")

    os.remove(rc.REVERT_FLAG)
    snapshots.write_current(target_name)
    rc.info("Revert applied.")
    return True


def main() -> None:
    try:
        apply_revert()
    except Exception as exc:
        rc.warn(f"Revert failed: {exc}")
        if os.path.exists(rc.REVERT_FLAG):
            os.remove(rc.REVERT_FLAG)
        sys.exit(1)


if __name__ == "__main__":
    main()
