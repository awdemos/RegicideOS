#!/usr/bin/env python3
"""A/B root subvolume management for atomic system updates.

RegicideOS keeps the active root on the ROOTS Btrfs partition. This module
maintains two root subvolumes, A and B, and atomically switches the default
root between them. The inactive root is the update target; after a new image
is installed into it and verified, it is promoted to the active root. If the
new root fails to boot or verify, the previous root can be rolled back.

Layout on ROOTS:
    /roots_a       active or standby root subvolume
    /roots_b       active or standby root subvolume
    /.regicide-root-current   name of the active slot ('a' or 'b')

The actual root subvolume is selected at boot by the kernel `rootflags`
option `subvol=roots_a` or `subvol=roots_b`, or by Btrfs default subvolume.
"""

import os
from pathlib import Path
from regicide_update import common as rc


SLOT_A = "a"
SLOT_B = "b"
ROOT_SLOT_SUBVOL = "roots_{slot}"
CURRENT_FILE = Path(rc.ROOTS_DIR) / ".regicide-root-current"


def _slot_path(slot: str) -> str:
    return os.path.join(rc.ROOTS_DIR, ROOT_SLOT_SUBVOL.format(slot=slot))


def _other_slot(slot: str) -> str:
    return SLOT_B if slot == SLOT_A else SLOT_A


def read_active_slot() -> str:
    """Return the currently active root slot, defaulting to 'a'."""
    if not CURRENT_FILE.is_file():
        return SLOT_A
    text = CURRENT_FILE.read_text().strip().splitlines()[0].strip().lower()
    return text if text in (SLOT_A, SLOT_B) else SLOT_A


def write_active_slot(slot: str) -> None:
    if slot not in (SLOT_A, SLOT_B):
        rc.die(f"Invalid root slot: {slot}")
    CURRENT_FILE.write_text(f"{slot}\n")


def _ensure_slots_exist() -> None:
    """Create the A/B root subvolumes if they are missing.

    If the active root is currently the top-level ROOTS partition, it is
    snapshotted into slot A so the running system is preserved, and slot B is
    created as an empty subvolume ready to receive updates.
    """
    if not rc.is_btrfs(rc.ROOTS_DIR):
        rc.die(f"{rc.ROOTS_DIR} is not a btrfs filesystem")

    active = read_active_slot()
    active_path = _slot_path(active)

    if not os.path.isdir(active_path):
        # The existing root lives at the top-level of ROOTS. Snapshot it into
        # the active slot so we can later swap without data loss.
        rc.execute(
            "btrfs",
            ["subvolume", "snapshot", "-r", rc.ROOTS_DIR, active_path],
        )

    other = _other_slot(active)
    other_path = _slot_path(other)
    if not os.path.isdir(other_path):
        rc.execute("btrfs", ["subvolume", "create", other_path])


def prepare_update_slot() -> str:
    """Return the inactive slot path ready to receive a new root image."""
    _ensure_slots_exist()
    inactive = _other_slot(read_active_slot())
    inactive_path = _slot_path(inactive)
    if os.path.isdir(inactive_path):
        # Wipe any prior contents so the new image starts clean.
        rc.execute("btrfs", ["subvolume", "delete", inactive_path])
    rc.execute("btrfs", ["subvolume", "create", inactive_path])
    return inactive_path


def verify_root(path: str) -> bool:
    """Verify that a root subvolume looks bootable.

    Checks for the existence of key directories needed by a Linux system and
    a kernel/initramfs under /boot. Kernel and initramfs names are matched with
    globs so Gentoo-style vmlinuz-<kver>/initramfs-<kver>.img names work too.
    Returns True if verification passes.
    """
    import glob

    required = ["usr", "bin", "lib", "etc", "var"]
    for entry in required:
        if not os.path.isdir(os.path.join(path, entry)):
            rc.warn(f"Verification failed: missing {entry}/ in {path}")
            return False
    boot_dir = os.path.join(path, "boot")
    if not os.path.isdir(boot_dir):
        rc.warn(f"Verification failed: no /boot in {path}")
        return False
    # Match common kernel and initramfs naming conventions, including versioned
    # Gentoo filenames like vmlinuz-6.1.0 and initramfs-6.1.0.img.
    has_kernel = any(
        glob.glob(os.path.join(boot_dir, pattern))
        for pattern in (
            "vmlinuz*",
            "vmlinuz-linux*",
            "Image*",
            "bzImage*",
            "kernel*",
        )
    )
    has_initrd = any(
        glob.glob(os.path.join(boot_dir, pattern))
        for pattern in (
            "initramfs*",
            "initrd*",
        )
    )
    if not (has_kernel and has_initrd):
        rc.warn(f"Verification failed: missing kernel/initramfs in {boot_dir}")
        return False
    rc.info(f"Root verification passed for {path}")
    return True


def activate_slot(slot: str) -> None:
    """Promote the named slot to the active root.

    This writes the active-slot marker. The actual boot switch is performed
    by updating the bootloader entry (see boot_entry.py).
    """
    path = _slot_path(slot)
    if not os.path.isdir(path):
        rc.die(f"Cannot activate {slot}: subvolume {path} does not exist")
    write_active_slot(slot)
    rc.info(f"Activated root slot {slot}")


def rollback() -> str:
    """Switch back to the previously active slot and return its name."""
    current = read_active_slot()
    previous = _other_slot(current)
    previous_path = _slot_path(previous)
    if not os.path.isdir(previous_path):
        rc.die(f"Cannot rollback to slot {previous}: subvolume missing")
    activate_slot(previous)
    return previous


def install_and_activate(image: Path) -> str:
    """Install a tarball into the inactive slot, verify it, and activate it.

    Returns the newly active slot name. The previous slot remains intact and
    can be rolled back to later.
    """
    from regicide_update import image as img

    if not rc.is_btrfs(rc.ROOTS_DIR):
        rc.die(f"{rc.ROOTS_DIR} is not a btrfs filesystem")
    inactive_path = prepare_update_slot()
    img.install_tarball(image, inactive_path)
    if not verify_root(inactive_path):
        rc.die("New root failed verification; keeping current root active")
    slot = os.path.basename(inactive_path).replace("roots_", "")
    activate_slot(slot)
    return slot
