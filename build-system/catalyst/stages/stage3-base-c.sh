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
    sys-fs/zfs
    sys-fs/zfs-kmod
)

for pkg in "${PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "filesystem and storage packages installed"
echo "Stage 3c complete."
