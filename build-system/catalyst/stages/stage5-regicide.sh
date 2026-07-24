#!/bin/bash
# Stage 5: install RegicideOS tools.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage5-regicide"

log_status "start" "installing RegicideOS tools"
REGICIDE_PACKAGES=(
    regicide-tools/regicide-installer
    sys-fs/btrfs-assistant
)

for pkg in "${REGICIDE_PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "RegicideOS tools installed"
echo "Stage 5 complete."
