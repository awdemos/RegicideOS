#!/bin/bash
# Stage 3d: install container and virtualization tooling.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage3-base-d"

log_status "start" "installing container packages"
PACKAGES=(
    app-containers/crun
    app-containers/distrobox
    app-containers/podman
    sys-apps/flatpak
)

for pkg in "${PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "container packages installed"
echo "Stage 3d complete."
