#!/usr/bin/env python3
"""GRUB bootloader entry management for A/B root slots.

RegicideOS images ship GRUB, not systemd-boot. This module maintains a small
GRUB configuration that selects the active root slot via a `regicide_slot`
environment variable stored in grubenv. Updates atomically rewrite grubenv
(with a backup/restore), so the next boot boots the correct slot without
needing to edit the main grub.cfg.
"""

import glob
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from regicide_update import common as rc
from regicide_update import root_ab


ESP_BOOT_DIR = Path("/boot")
_GRUBENV = Path("/boot/grub/grubenv")
_GRUBENV_BACKUP = Path("/boot/grub/grubenv.regicide-backup")
_GRUB_CFG = Path("/boot/grub/grub.cfg")
_BACKUP_SUFFIX = ".regicide-backup"

_KERNEL_PATTERNS = ("vmlinuz*", "vmlinuz-linux*", "Image*", "bzImage*", "kernel*")
_INITRD_PATTERNS = ("initramfs*", "initrd*")


def _grubenv_path() -> Path:
    return ESP_BOOT_DIR / "grub" / "grubenv"


def _grubenv_backup_path() -> Path:
    return ESP_BOOT_DIR / "grub" / "grubenv.regicide-backup"


def _grub_cfg() -> Path:
    return ESP_BOOT_DIR / "grub" / "grub.cfg"


def _find_best(path: str, patterns: tuple[str, ...]) -> str | None:
    """Return the basename of the best kernel/initramfs file in the slot."""
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
    # Pick the highest (newest) version string via version-sort semantics.
    return sorted(candidates, key=lambda n: (len(n), n))[-1]


def discover_kernel_initrd(slot: str | None = None) -> tuple[str, str]:
    """Discover the kernel and initramfs basenames inside the named slot.

    Paths are returned relative to the slot's /boot directory.
    """
    target_slot = slot or root_ab.read_active_slot()
    boot_dir = os.path.join(
        rc.ROOTS_DIR, root_ab.ROOT_SLOT_SUBVOL.format(slot=target_slot), "boot"
    )
    if not os.path.isdir(boot_dir):
        rc.die(f"Boot directory not found for slot {target_slot}: {boot_dir}")
    kernel = _find_best(boot_dir, _KERNEL_PATTERNS)
    initrd = _find_best(boot_dir, _INITRD_PATTERNS)
    if not kernel:
        rc.die(f"No kernel found in {boot_dir}")
    if not initrd:
        rc.die(f"No initramfs found in {boot_dir}")
    return kernel, initrd


def _backup(path: Path) -> Path:
    """Create a backup of an existing file next to the original."""
    backup = Path(str(path) + _BACKUP_SUFFIX)
    if path.is_file():
        shutil.copy2(path, backup)
    return backup


def _restore_or_delete_backup(path: Path, backup: Path) -> None:
    if backup.is_file():
        if path.is_file():
            path.unlink()
        shutil.move(backup, path)
    elif backup.exists():
        backup.unlink()


def _atomic_rename(src: Path, dst: Path) -> None:
    os.replace(src, dst)


def _grub_editenv(args: list[str]) -> None:
    """Run grub-editenv with the given arguments.

    If the host does not have grub-editenv, fall back to a chroot inside /boot
    (some images ship GRUB tools only in the installed root).
    """
    env_path = _grubenv_path()
    env_path.parent.mkdir(parents=True, exist_ok=True)
    for cmd in ("grub-editenv", "grub2-editenv"):
        if shutil.which(cmd):
            subprocess.run([cmd, str(env_path), *args], check=True)
            return
    # Last resort: chroot into the active root if GRUB tools live there.
    active = root_ab.read_active_slot()
    active_root = os.path.join(rc.ROOTS_DIR, root_ab.ROOT_SLOT_SUBVOL.format(slot=active))
    for cmd in ("grub-editenv", "grub2-editenv"):
        chroot_cmd = shutil.which(cmd) or f"/usr/bin/{cmd}"
        if os.path.exists(os.path.join(active_root, chroot_cmd.lstrip("/"))):
            subprocess.run(
                ["chroot", active_root, chroot_cmd, str(env_path), *args],
                check=True,
            )
            return
    rc.die("grub-editenv not found; cannot update GRUB slot selection")


def _init_grubenv() -> None:
    """Ensure grubenv exists and is writable by grub-editenv."""
    env_path = _grubenv_path()
    if not env_path.exists():
        env_path.parent.mkdir(parents=True, exist_ok=True)
        for cmd in ("grub-editenv", "grub2-editenv"):
            if shutil.which(cmd):
                subprocess.run([cmd, str(env_path), "create"], check=False)
                return
        # Fallback: write a minimal grubenv header.
        env_path.write_bytes(b"# GRUB Environment Block\n" + b"#" * (1024 - 25) + b"\n")


def write_slot_to_grubenv(slot: str) -> None:
    """Atomically update the GRUB environment to select the named root slot."""
    if slot not in (root_ab.SLOT_A, root_ab.SLOT_B):
        rc.die(f"Invalid slot for boot default: {slot}")

    env_path = _grubenv_path()
    _init_grubenv()

    backup = _backup(env_path)
    tmp = Path(tempfile.mktemp(dir=env_path.parent, prefix="grubenv-"))
    try:
        if env_path.is_file():
            shutil.copy2(env_path, tmp)
        # Use grub-editenv to set the slot variable. This preserves the existing
        # environment block format and avoids corrupting the 1024-byte block.
        _grub_editenv(["set", f"regicide_slot={slot}"])
        rc.info(f"Set GRUB default slot to {slot}")
    except Exception:
        _restore_or_delete_backup(env_path, backup)
        raise
    finally:
        if backup.is_file():
            backup.unlink()
        if tmp.is_file():
            tmp.unlink()


def ensure_grub_cfg() -> None:
    """Ensure a GRUB config that boots the slot stored in grubenv.

    The generated config is minimal and GRUB-specific. It is idempotent: if the
    file already contains the regicide_slot dispatch, it is left alone.
    """
    cfg_path = _grub_cfg()
    cfg_path.parent.mkdir(parents=True, exist_ok=True)

    if cfg_path.is_file() and "regicide_slot" in cfg_path.read_text():
        return

    cfg = """set default="RegicideOS"
set timeout=5

# A/B slot selection: regicide_slot is set by regicide-update in grubenv.
if [ -z "$regicide_slot" ]; then
    set regicide_slot=a
fi

set rootflags="subvol=roots_${regicide_slot}"

menuentry "RegicideOS ($regicide_slot)" {
    linux /roots/roots_${regicide_slot}/boot/vmlinuz root=LABEL=ROOTS ro $rootflags
    initrd /roots/roots_${regicide_slot}/boot/initramfs.img
}
"""
    tmp = Path(tempfile.mktemp(dir=cfg_path.parent, prefix="grub.cfg-"))
    try:
        tmp.write_text(cfg)
        _atomic_rename(tmp, cfg_path)
        rc.info(f"Wrote GRUB A/B config: {cfg_path}")
    except Exception:
        if tmp.is_file():
            tmp.unlink()
        raise


def sync_entries() -> None:
    """Ensure GRUB is configured and grubenv points to the active root slot."""
    ensure_grub_cfg()
    for slot in (root_ab.SLOT_A, root_ab.SLOT_B):
        discover_kernel_initrd(slot)
    write_slot_to_grubenv(root_ab.read_active_slot())


def install_and_sync(image: Path) -> str:
    """Install a new root image, verify it, activate it, and update GRUB."""
    slot = root_ab.install_and_activate(image)
    sync_entries()
    return slot


def rollback_and_sync() -> str:
    """Rollback to the previous root slot and update GRUB."""
    slot = root_ab.rollback()
    sync_entries()
    return slot
