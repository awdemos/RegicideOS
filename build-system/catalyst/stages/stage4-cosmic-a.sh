#!/bin/bash
# Stage 4a: install core COSMIC desktop session packages.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage4-cosmic-a"

log_status "start" "installing COSMIC core packages"
COSMIC_OVERLAY_DIR="${REGICIDE_COSMIC_OVERLAY_DIR:-${ROOTFS}/var/db/repos/cosmic-overlay}"
mkdir -p "${ROOTFS}/var/db/repos"

if [[ ! -d "${COSMIC_OVERLAY_DIR}/.git" && ! -f "${COSMIC_OVERLAY_DIR}/profiles/repo_name" ]]; then
    echo "Cloning cosmic-overlay..."
    git clone --depth 1 https://github.com/fsvm88/cosmic-overlay.git "${COSMIC_OVERLAY_DIR}" 2>/dev/null || \
        echo "WARNING: Could not clone cosmic-overlay. COSMIC packages will not be available."
fi

# Skip the copy when the overlay was cloned directly into the rootfs (no
# external REGICIDE_COSMIC_OVERLAY_DIR) — otherwise rm would delete the clone.
if [[ -d "${COSMIC_OVERLAY_DIR}" && "${COSMIC_OVERLAY_DIR}" != "${ROOTFS}/var/db/repos/cosmic-overlay" && ! -L "${ROOTFS}/var/db/repos/cosmic-overlay" ]]; then
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
if [[ -d "${CATALYST_DIR}/cosmic-overlay/cosmic-utils/minimon" ]]; then
    # RegicideOS's vendored minimon ebuild (with the renamed binary) overrides
    # the freshly cloned upstream one. Copy ONLY minimon: the vendored tree is
    # a stale snapshot whose old ebuilds/eclasses break current COSMIC builds.
    mkdir -p "${ROOTFS}/var/db/repos/cosmic-overlay/cosmic-utils"
    cp -a "${CATALYST_DIR}/cosmic-overlay/cosmic-utils/minimon" \
        "${ROOTFS}/var/db/repos/cosmic-overlay/cosmic-utils/"
fi

mkdir -p "${ROOTFS}/etc/portage/repos.conf"

# Mask live 9999 ebuilds from the COSMIC overlay so release versions stay
# selected. The overlay keywords its 9999 ebuilds ~amd64, so they always
# outrank releases, but release ebuilds pin each other (e.g.
# cosmic-launcher-1.3.0 requires ~pop-launcher-1.3.0), which breaks
# `emerge -uDU @world` (regicide-update upgrade) with slot conflicts.
# Exception: any 9999 package that a RELEASE ebuild depends on must stay
# unmasked (e.g. cosmic-meta-1.3.0 requires ~pop-theme-meta-9999,
# cosmic-initial-setup-1.3.0 requires ~pop-appstream-data-9999). Compute
# that set dynamically so upstream dependency changes don't break the build.
mkdir -p "${ROOTFS}/etc/portage/package.mask"
(
    cd "${ROOTFS}/var/db/repos/cosmic-overlay"
    find . -name '*-9999.ebuild' | sed -E 's|^\./([^/]+)/[^/]+/([^/]+)\.ebuild$|=\1/\2|' | sort -u > /tmp/mask-all
    find . -name '*.ebuild' ! -name '*-9999.ebuild' -print0 \
        | xargs -0 grep -hoE '[a-z0-9._-]+/[a-z0-9._-]+-9999' \
        | sed -E 's|^([a-z0-9._-]+/[a-z0-9._-]+-9999).*$|=\1|' | sort -u > /tmp/mask-keep
    # minimon is RegicideOS's own applet and only exists as a 9999 ebuild.
    echo '=cosmic-utils/minimon-9999' >> /tmp/mask-keep
    grep -Fvx -f /tmp/mask-keep /tmp/mask-all
) > "${ROOTFS}/etc/portage/package.mask/cosmic-overlay-live"
echo "Masked $(wc -l < "${ROOTFS}/etc/portage/package.mask/cosmic-overlay-live") live 9999 ebuilds from cosmic-overlay"

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
    cosmic-base/cosmic-comp
    cosmic-base/cosmic-session
    cosmic-base/cosmic-panel
    cosmic-base/cosmic-launcher
    cosmic-base/cosmic-notifications
    cosmic-base/cosmic-osd
    cosmic-base/cosmic-bg
    cosmic-base/cosmic-settings-daemon
    cosmic-base/cosmic-randr
    cosmic-base/cosmic-workspaces-epoch
    cosmic-base/pop-launcher
    cosmic-base/xdg-desktop-portal-cosmic
)

for pkg in "${COSMIC_PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg"
done

clean_rootfs_transient
log_status "complete" "COSMIC core packages installed"
echo "Stage 4a complete."
