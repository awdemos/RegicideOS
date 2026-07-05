#!/bin/bash
# Stage 3e: install desktop, media, and boot support packages.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage3-base-e"

log_status "start" "installing desktop and media packages"
PACKAGES=(
    media-libs/gstreamer
    media-libs/mesa
    media-video/wireplumber
    net-print/cups
    sys-apps/xdg-desktop-portal
    sys-apps/xdg-desktop-portal-gtk
    sys-auth/fprintd
    sys-block/gparted
    sys-boot/grub
    sys-boot/plymouth
)

for pkg in "${PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "desktop and media packages installed"
echo "Stage 3e complete."
