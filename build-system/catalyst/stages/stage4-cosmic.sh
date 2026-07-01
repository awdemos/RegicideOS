#!/bin/bash
# Stage 4: install the COSMIC desktop packages.
set -euo pipefail

source "$(dirname "$0")/common.sh"

COSMIC_OVERLAY_DIR="${REGICIDE_COSMIC_OVERLAY_DIR:-${ROOTFS}/var/db/repos/cosmic-overlay}"
if [[ ! -d "${COSMIC_OVERLAY_DIR}/.git" ]]; then
    echo "Cloning cosmic-overlay..."
    git clone --depth 1 https://github.com/fsvm88/cosmic-overlay.git "${COSMIC_OVERLAY_DIR}" 2>/dev/null || \
        echo "WARNING: Could not clone cosmic-overlay. COSMIC packages will not be available."
fi
if [[ "${COSMIC_OVERLAY_DIR}" != "${ROOTFS}/var/db/repos/cosmic-overlay" ]] && [[ -d "${COSMIC_OVERLAY_DIR}/.git" ]]; then
    echo "Linking cached cosmic-overlay into rootfs..."
    mkdir -p "${ROOTFS}/var/db/repos"
    ln -sf "${COSMIC_OVERLAY_DIR}" "${ROOTFS}/var/db/repos/cosmic-overlay"
fi

REGICIDE_OVERLAY="${CATALYST_DIR}/../../overlays/regicide-rust"
if [[ -d "${REGICIDE_OVERLAY}" ]]; then
    echo "Copying regicide-rust overlay..."
    cp -r "${REGICIDE_OVERLAY}" "${ROOTFS}/var/db/repos/regicide-rust" 2>/dev/null || true
else
    echo "WARNING: regicide-rust overlay not found at ${REGICIDE_OVERLAY}"
fi

if [[ -d "${CATALYST_DIR}/overlay" ]]; then
    cp -r "${CATALYST_DIR}/overlay"/* "${ROOTFS}" 2>/dev/null || true
fi
if [[ -d "${CATALYST_DIR}/cosmic-overlay" ]]; then
    cp -r "${CATALYST_DIR}/cosmic-overlay"/* "${ROOTFS}" 2>/dev/null || true
fi

COSMIC_PACKAGES=(
    cosmic-base/cosmic-meta
    cosmic-base/cosmic-applets
    cosmic-base/cosmic-bg
    cosmic-base/cosmic-comp
    cosmic-base/cosmic-edit
    cosmic-base/cosmic-files
    cosmic-base/cosmic-greeter
    cosmic-base/cosmic-icons
    cosmic-base/cosmic-launcher
    cosmic-base/cosmic-notifications
    cosmic-base/cosmic-osd
    cosmic-base/cosmic-panel
    cosmic-base/cosmic-randr
    cosmic-base/cosmic-screenshot
    cosmic-base/cosmic-session
    cosmic-base/cosmic-settings
    cosmic-base/cosmic-settings-daemon
    cosmic-base/cosmic-store
    cosmic-base/cosmic-term
    cosmic-base/cosmic-wallpapers
    cosmic-base/cosmic-workspaces-epoch
    cosmic-base/pop-launcher
    cosmic-base/xdg-desktop-portal-cosmic
)

for pkg in "${COSMIC_PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

echo "Stage 4 complete."
