#!/usr/bin/env python3
"""GRUB bootloader entry management for A/B root slots.

RegicideOS images ship GRUB, not systemd-boot. This module maintains a small
GRUB dispatch config (/boot/grub/regicide.cfg) that selects the active root
slot via a `regicide_slot` environment variable stored in grubenv. The main
grub.cfg only carries a one-line `source` stub so that grub-mkconfig output
and the A/B dispatch never stomp each other. Updates atomically rewrite
grubenv (with a backup/restore), so the next boot boots the correct slot.
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

# The A/B menu lives in its own file; grub.cfg only sources it. The marker
# line identifies a dispatch file as managed so hand edits are preserved.
_DISPATCH_MARKER = "# RegicideOS A/B dispatch (managed)"
_STUB_MARKER = "# RegicideOS A/B managed stub"
_SOURCE_LINE = "source $prefix/regicide.cfg"

_KERNEL_PATTERNS = ("vmlinuz*", "vmlinuz-linux*", "Image*", "bzImage*", "kernel*")
_INITRD_PATTERNS = ("initramfs*", "initrd*")


def _grubenv_path() -> Path:
    return ESP_BOOT_DIR / "grub" / "grubenv"


def _grubenv_backup_path() -> Path:
    return ESP_BOOT_DIR / "grub" / "grubenv.regicide-backup"


def _grub_cfg() -> Path:
    return ESP_BOOT_DIR / "grub" / "grub.cfg"


def _grub_dispatch() -> Path:
    return ESP_BOOT_DIR / "grub" / "regicide.cfg"


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
    # Pick the highest (newest) version via numeric version segments, so
    # vmlinuz-6.9.10 sorts above vmlinuz-6.9.9 (plain string sort reverses).
    return sorted(candidates, key=root_ab.version_segments)[-1]


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


def read_slot_from_grubenv() -> str | None:
    """Return the slot GRUB will boot, or None if grubenv holds no preference.

    Aborts if grubenv exists but cannot be read or names an invalid slot:
    the bootloader's choice is the source of truth for what actually boots,
    so an unreadable grubenv must stop any destructive update work.
    """
    env_path = _grubenv_path()
    if not env_path.exists():
        return None
    try:
        text = env_path.read_text(errors="replace")
    except OSError as exc:
        rc.die(f"Cannot read grubenv {env_path}: {exc}")
    for line in text.splitlines():
        key, _, value = line.partition("=")
        if key.strip() == "regicide_slot":
            slot = value.strip().lower()
            if slot in (root_ab.SLOT_A, root_ab.SLOT_B):
                return slot
            rc.die(
                f"grubenv {env_path} has invalid regicide_slot: {value.strip()!r}"
            )
    return None


def _dispatch_config_text() -> str:
    """Build the A/B dispatch menu with the real per-slot kernel/initrd names.

    Slots without a bootable kernel/initramfs are omitted (with a warning)
    rather than generating menuentries that cannot boot.
    """
    entries: list[tuple[str, str, str]] = []
    for slot in (root_ab.SLOT_A, root_ab.SLOT_B):
        try:
            kernel, initrd = discover_kernel_initrd(slot)
        except SystemExit:
            rc.warn(
                f"Slot {slot} has no bootable kernel/initramfs; "
                f"omitting it from the GRUB menu"
            )
            continue
        entries.append((slot, kernel, initrd))

    lines = [
        _DISPATCH_MARKER,
        "# Generated by regicide-update. Do not edit by hand.",
        'set default="regicide-${regicide_slot}"',
        "set timeout=5",
        "",
        "insmod btrfs",
        "search --label ROOTS --set=root",
        "",
        "# A/B slot selection: regicide_slot is set by regicide-update in grubenv.",
        'if [ -z "$regicide_slot" ]; then',
        "    set regicide_slot=a",
        "fi",
        "",
    ]
    for slot, kernel, initrd in entries:
        # Paths are relative to the ROOTS filesystem root: the subvolume is
        # roots_<slot>, so the fs-relative path is /roots_<slot>/boot/....
        subvol = root_ab.ROOT_SLOT_SUBVOL.format(slot=slot)
        lines += [
            f'menuentry "RegicideOS ({slot})" --id=regicide-{slot} {{',
            f"    linux /{subvol}/boot/{kernel} root=LABEL=ROOTS ro rootflags=subvol={subvol}",
            f"    initrd /{subvol}/boot/{initrd}",
            "}",
            "",
        ]
    return "\n".join(lines)


def _ensure_source_stub(cfg_path: Path) -> None:
    """Make the main grub.cfg source the A/B dispatch, preserving other content.

    grub-mkconfig rewrites grub.cfg from scratch (see cli_update's
    maybe_refresh_bootloader); if the source line is missing it is appended
    again so the A/B menu keeps loading alongside the generated entries.
    """
    text = ""
    if cfg_path.is_file():
        try:
            text = cfg_path.read_text()
        except OSError as exc:
            rc.die(f"Cannot read GRUB config {cfg_path}: {exc}")
        if _SOURCE_LINE in text:
            return
    addition = ""
    if text:
        if not text.endswith("\n"):
            addition += "\n"
        addition += "\n"
    addition += f"{_STUB_MARKER}\n{_SOURCE_LINE}\n"
    with open(cfg_path, "a") as f:
        f.write(addition)
    rc.info(f"Ensured A/B dispatch stub in {cfg_path}")


def ensure_source_stub() -> None:
    """Restore the A/B dispatch source line in the main grub.cfg if missing.

    Shared by ensure_grub_cfg and cli_update.maybe_refresh_bootloader so both
    paths keep grub.cfg sourcing regicide.cfg.
    """
    cfg_path = _grub_cfg()
    cfg_path.parent.mkdir(parents=True, exist_ok=True)
    _ensure_source_stub(cfg_path)


def ensure_grub_cfg() -> None:
    """Ensure GRUB can boot the slot stored in grubenv.

    The A/B dispatch menu lives in its own file (regicide.cfg) so a
    grub-mkconfig run cannot clobber it; the main grub.cfg only needs the
    `source` stub. Idempotent: a dispatch file already carrying the managed
    marker is left alone.
    """
    cfg_path = _grub_cfg()
    cfg_path.parent.mkdir(parents=True, exist_ok=True)

    dispatch_path = _grub_dispatch()
    managed = (
        dispatch_path.is_file()
        and _DISPATCH_MARKER in dispatch_path.read_text()
    )
    if not managed:
        tmp = Path(tempfile.mktemp(dir=cfg_path.parent, prefix="regicide.cfg-"))
        try:
            tmp.write_text(_dispatch_config_text())
            _atomic_rename(tmp, dispatch_path)
            rc.info(f"Wrote GRUB A/B dispatch config: {dispatch_path}")
        except Exception:
            if tmp.is_file():
                tmp.unlink()
            raise

    _ensure_source_stub(cfg_path)


def sync_entries() -> None:
    """Ensure GRUB is configured and grubenv points to the active root slot.

    A missing kernel/initramfs on the ACTIVE slot is fatal: the system could
    not boot. An unbootable STANDBY slot is only a warning: it cannot affect
    the next boot, and rollback() refuses to switch to it anyway.
    """
    ensure_grub_cfg()
    active = root_ab.read_active_slot()
    try:
        discover_kernel_initrd(active)
    except SystemExit:
        rc.die(f"Active root slot {active} has no bootable kernel/initramfs")
    for slot in (root_ab.SLOT_A, root_ab.SLOT_B):
        if slot == active:
            continue
        try:
            discover_kernel_initrd(slot)
        except SystemExit:
            rc.warn(
                f"Standby slot {slot} is not bootable; a rollback to it "
                f"will be refused"
            )
    write_slot_to_grubenv(active)


def install_and_sync(image: Path) -> str:
    """Install a new root image, verify it, and update GRUB.

    Write order matters: grubenv is updated BEFORE the CURRENT_FILE marker.
    A crash in between then fails safe toward the previously verified slot:
    GRUB keeps booting the old slot per grubenv, and the next update
    reconciles the marker from grubenv before choosing what to wipe.
    """
    slot = root_ab.install_image(image)
    write_slot_to_grubenv(slot)
    root_ab.activate_slot(slot)
    ensure_grub_cfg()
    return slot


def rollback_and_sync() -> str:
    """Rollback to the previous root slot and update GRUB."""
    slot = root_ab.rollback()
    write_slot_to_grubenv(slot)
    ensure_grub_cfg()
    return slot
