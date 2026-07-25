#!/usr/bin/env python3
"""systemd-boot entry management for A/B root slots.

This module reads/writes bootloader configuration files under the EFI system
partition (ESP) so the next boot selects the active root slot. It assumes the
ESP is mounted at /boot or /efi; /boot is the default used by RegicideOS.
"""

import glob
import os
import shutil
import tempfile
from pathlib import Path
from regicide_update import common as rc
from regicide_update import root_ab


ESP_BOOT_DIR = Path("/boot")
_BACKUP_SUFFIX = ".regicide-backup"


_ROOT_SLOT_OPTION = "rootflags=subvol=roots_{slot}"
KERNEL_PATTERNS = ("vmlinuz*", "vmlinuz-linux*", "Image*", "bzImage*", "kernel*")
INITRD_PATTERNS = ("initramfs*", "initrd*")


def _loader_dir() -> Path:
    return ESP_BOOT_DIR / "loader"


def _entries_dir() -> Path:
    return _loader_dir() / "entries"


def _loader_conf() -> Path:
    return _loader_dir() / "loader.conf"


def _entry_path(slot: str) -> Path:
    return _entries_dir() / f"regicide-{slot}.conf"


def _entry_backup_path(slot: str) -> Path:
    return Path(str(_entry_path(slot)) + _BACKUP_SUFFIX)


def _loader_backup_path() -> Path:
    return Path(str(_loader_conf()) + _BACKUP_SUFFIX)


def _entry_title(slot: str) -> str:
    active = " (active)" if root_ab.read_active_slot() == slot else ""
    return f"RegicideOS {slot.upper()}{active}"


def _find_best(path: str, patterns: tuple[str, ...]) -> str | None:
    """Return the basename of the best kernel/initramfs file in the slot.

    Preference order: exact stable name (vmlinuz/initramfs.img) first, then
    sorted versioned names (shortest, then alphabetical). This keeps selection
    deterministic even if multiple files are present.
    """
    candidates: list[str] = []
    for pattern in patterns:
        candidates.extend(
            os.path.basename(p) for p in glob.glob(os.path.join(path, pattern))
        )
    if not candidates:
        return None
    stable_names = {"vmlinuz", "initramfs.img", "initrd.img"}
    for name in sorted(stable_names):
        if name in candidates:
            return name
    return sorted(candidates, key=lambda n: (len(n), n))[0]


def ensure_dirs() -> None:
    """Create the systemd-boot loader directories if they do not exist."""
    _entries_dir().mkdir(parents=True, exist_ok=True)


def _backup(path: Path) -> Path:
    """Create a backup of an existing file next to the original."""
    backup = Path(str(path) + _BACKUP_SUFFIX)
    if path.is_file():
        shutil.copy2(path, backup)
    return backup


def _restore_or_delete_backup(path: Path, backup: Path) -> None:
    """Restore original file from backup if it existed; otherwise remove backup."""
    if backup.is_file():
        if path.is_file():
            path.unlink()
        shutil.move(backup, path)
    elif backup.exists():
        backup.unlink()


def _atomic_rename(src: Path, dst: Path) -> None:
    """Rename src to dst using an atomic os.replace where available."""
    os.replace(src, dst)


def write_entry(slot: str, kernel: str, initrd: str) -> None:
    """Write a systemd-boot entry file for the named root slot."""
    ensure_dirs()
    entry = _entry_path(slot)
    backup = _backup(entry)
    tmp = Path(tempfile.mktemp(dir=_entries_dir(), prefix=f"regicide-{slot}-"))
    try:
        title = _entry_title(slot)
        rootflags = _ROOT_SLOT_OPTION.format(slot=slot)
        contents = (
            f"title {title}\n"
            f"linux {kernel}\n"
            f"initrd {initrd}\n"
            f"options root=LABEL=ROOTS ro {rootflags}\n"
        )
        tmp.write_text(contents)
        _atomic_rename(tmp, entry)
        rc.info(f"Wrote boot entry: {entry}")
    except Exception:
        _restore_or_delete_backup(entry, backup)
        raise
    finally:
        if backup.is_file():
            backup.unlink()
        if tmp.is_file():
            tmp.unlink()


def set_default(slot: str) -> None:
    """Set the default bootloader entry to the named root slot."""
    if slot not in (root_ab.SLOT_A, root_ab.SLOT_B):
        rc.die(f"Invalid slot for boot default: {slot}")
    ensure_dirs()
    if not _entry_path(slot).is_file():
        rc.die(f"Boot entry for slot {slot} does not exist")
    loader_conf = _loader_conf()
    backup = _backup(loader_conf)
    tmp = Path(tempfile.mktemp(dir=_loader_dir(), prefix="loader.conf-"))
    try:
        tmp.write_text(f"default regicide-{slot}\ntimeout 5\n")
        _atomic_rename(tmp, loader_conf)
        rc.info(f"Set default boot entry to regicide-{slot}")
    except Exception:
        _restore_or_delete_backup(loader_conf, backup)
        raise
    finally:
        if backup.is_file():
            backup.unlink()
        if tmp.is_file():
            tmp.unlink()


def discover_kernel_initrd(slot: str | None = None) -> tuple[str, str]:
    """Discover the kernel and initramfs basenames in the active root slot.

    Paths are returned relative to /boot (the ESP). systemd-boot resolves them
    against the ESP root.
    """
    target_slot = slot or root_ab.read_active_slot()
    boot_dir = os.path.join(rc.ROOTS_DIR, root_ab.ROOT_SLOT_SUBVOL.format(slot=target_slot), "boot")
    if not os.path.isdir(boot_dir):
        rc.die(f"Boot directory not found for slot {target_slot}: {boot_dir}")
    kernel = _find_best(boot_dir, KERNEL_PATTERNS)
    initrd = _find_best(boot_dir, INITRD_PATTERNS)
    if not kernel:
        rc.die(f"No kernel found in {boot_dir}")
    if not initrd:
        rc.die(f"No initramfs found in {boot_dir}")
    return f"/{kernel}", f"/{initrd}"


def sync_entries() -> None:
    """Ensure entries exist for both slots and default to the active slot.

    Updates are performed atomically for each entry file and for loader.conf.
    If any part fails, previously written entries are left intact; individual
    entry writes restore their own old files on failure.
    """
    for slot in (root_ab.SLOT_A, root_ab.SLOT_B):
        kernel, initrd = discover_kernel_initrd(slot)
        write_entry(slot, kernel, initrd)
    set_default(root_ab.read_active_slot())


def install_and_sync(image: Path) -> str:
    """Install a new root image, verify it, activate it, and update boot entries."""
    slot = root_ab.install_and_activate(image)
    sync_entries()
    return slot


def rollback_and_sync() -> str:
    """Rollback to the previous root slot and update boot entries."""
    slot = root_ab.rollback()
    sync_entries()
    return slot
