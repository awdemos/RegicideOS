#!/bin/bash
# Stage 4b: install COSMIC applications and the greeter.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage4-cosmic-b"

log_status "start" "installing COSMIC applications"

COSMIC_PACKAGES=(
    cosmic-base/cosmic-applets
    cosmic-base/cosmic-edit
    cosmic-base/cosmic-files
    cosmic-base/cosmic-greeter
    cosmic-base/cosmic-icons
    cosmic-base/cosmic-settings
    cosmic-base/cosmic-store
    cosmic-base/cosmic-term
    cosmic-base/cosmic-wallpapers
    cosmic-base/cosmic-screenshot
)

for pkg in "${COSMIC_PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg"
done

echo "Verifying COSMIC greeter binary..."
if [[ ! -x "${ROOTFS}/usr/bin/cosmic-greeter" ]]; then
    log_status "error" "cosmic-greeter binary missing"
    echo "ERROR: cosmic-greeter was not installed. Check the Portage log above."
    exit 1
fi

clean_rootfs_transient
log_status "complete" "COSMIC applications installed"
echo "Stage 4b complete."
