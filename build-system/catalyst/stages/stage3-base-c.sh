#!/bin/bash
# Stage 3c: install filesystem, storage, and RAID tools.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage3-base-c"

log_status "start" "installing filesystem and storage packages"
PACKAGES=(
    sys-fs/bcache-tools
    sys-fs/btrfs-progs
    sys-fs/cryptsetup
    sys-fs/dmraid
    sys-fs/dosfstools
    sys-fs/e2fsprogs
    sys-fs/exfatprogs
    sys-fs/f2fs-tools
    sys-fs/fuse-exfat
    sys-fs/fuse-overlayfs
    sys-fs/fuseiso
    sys-fs/go-mtpfs
    sys-fs/jfsutils
    sys-fs/lsscsi
    sys-fs/lvm2
    sys-fs/mac-fdisk
    sys-fs/mdadm
    sys-fs/multipath-tools
    sys-fs/ntfs3g
    sys-fs/squashfs-tools
    sys-fs/xfsprogs
    # NOTE: ZFS is intentionally omitted for now: zfs-2.3.8 supports at most
    # kernel 6.18 while the image ships 7.1.x, and zfs-kmod-2.4.0 is RC-only
    # (missing keywords) while zfs userspace is at 2.4.3 — no matching pair.
    # Re-add once zfs-kmod 2.4.x goes stable.
)

for pkg in "${PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "filesystem and storage packages installed"
echo "Stage 3c complete."
