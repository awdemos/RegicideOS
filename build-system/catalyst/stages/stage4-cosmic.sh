#!/bin/bash
# Stage 4: install the COSMIC desktop packages.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage4-cosmic"

log_status "start" "installing COSMIC desktop packages"
COSMIC_OVERLAY_DIR="${REGICIDE_COSMIC_OVERLAY_DIR:-${ROOTFS}/var/db/repos/cosmic-overlay}"
mkdir -p "${ROOTFS}/var/db/repos"

if [[ ! -d "${COSMIC_OVERLAY_DIR}/.git" && ! -f "${COSMIC_OVERLAY_DIR}/profiles/repo_name" ]]; then
    echo "Cloning cosmic-overlay..."
    git clone --depth 1 https://github.com/fsvm88/cosmic-overlay.git "${COSMIC_OVERLAY_DIR}" 2>/dev/null || \
        echo "WARNING: Could not clone cosmic-overlay. COSMIC packages will not be available."
fi

if [[ -d "${COSMIC_OVERLAY_DIR}" && ! -L "${ROOTFS}/var/db/repos/cosmic-overlay" ]]; then
    echo "Installing cosmic-overlay into rootfs..."
    rm -rf "${ROOTFS}/var/db/repos/cosmic-overlay"
    cp -a "${COSMIC_OVERLAY_DIR}" "${ROOTFS}/var/db/repos/cosmic-overlay"
fi

REGICIDE_OVERLAY="${CATALYST_DIR}/../../overlays/regicide-rust"
if [[ -d "${REGICIDE_OVERLAY}" ]]; then
    echo "Installing regicide-rust overlay into rootfs..."
    rm -rf "${ROOTFS}/var/db/repos/regicide-rust"
    cp -a "${REGICIDE_OVERLAY}" "${ROOTFS}/var/db/repos/regicide-rust"
else
    echo "WARNING: regicide-rust overlay not found at ${REGICIDE_OVERLAY}"
fi

if [[ -d "${CATALYST_DIR}/overlay" ]]; then
    cp -a "${CATALYST_DIR}/overlay"/* "${ROOTFS}" 2>/dev/null || true
fi
if [[ -d "${CATALYST_DIR}/cosmic-overlay" ]]; then
    cp -a "${CATALYST_DIR}/cosmic-overlay"/* "${ROOTFS}" 2>/dev/null || true
fi

mkdir -p "${ROOTFS}/etc/portage/repos.conf"
cat > "${ROOTFS}/etc/portage/repos.conf/regicide.conf" << 'EOF'
[cosmic-overlay]
location = /var/db/repos/cosmic-overlay
sync-type = git
sync-uri = https://github.com/fsvm88/cosmic-overlay.git
auto-sync = no

[regicide-rust]
location = /var/db/repos/regicide-rust
sync-type = git
sync-uri = https://github.com/awdemos/RegicideOS.git
auto-sync = no
EOF

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
log_status "complete" "COSMIC desktop packages installed"
echo "Stage 4 complete."
