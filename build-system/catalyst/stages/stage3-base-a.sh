#!/bin/bash
# Stage 3a: install base CLI, toolchain, and foundation packages.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage3-base-a"

log_status "start" "installing base foundation packages"
PACKAGES=(
    app-admin/sudo
    app-editors/nano
    app-eselect/eselect-repository
    app-shells/bash-completion
    dev-lang/rust-bin
    dev-util/glib-utils
    dev-util/wayland-scanner
    dev-vcs/git
    media-fonts/fonts-meta
    sys-apps/iproute2
    sys-apps/lsb-release
    sys-apps/mlocate
    sys-auth/rtkit
    sys-block/io-scheduler-udev-rules
    sys-power/power-profiles-daemon
)

for pkg in "${PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "base foundation packages installed"
echo "Stage 3a complete."
