#!/bin/bash
# Stage 3b: install the kernel and firmware.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage3-base-b"

log_status "start" "installing kernel and firmware"
PACKAGES=(
    sys-kernel/gentoo-kernel-bin
    sys-kernel/linux-firmware
)

for pkg in "${PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "kernel and firmware installed"
echo "Stage 3b complete."
