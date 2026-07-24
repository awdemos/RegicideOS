#!/bin/bash
# Stage 2: sync Portage and update @world.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage2-sync"

log_status "start" "syncing portage and updating world"
mkdir -p "${ROOTFS}/etc/portage"

# Consume the local binary package cache (var/cache/binpkgs) so rebuilds
# reuse previously built binpkgs instead of recompiling. FEATURES=buildpkg
# keeps producing binpkgs for anything that still needs a source build.
# Set REGICIDE_USE_BINPKGS=0 to force source builds.
EMERGE_OPTS="--jobs --load-average"
if [[ "${REGICIDE_USE_BINPKGS:-1}" != "0" ]]; then
    EMERGE_OPTS="${EMERGE_OPTS} --usepkg --binpkg-respect-use=y"
    log_status "binpkgs" "local binpkg reuse enabled (--usepkg)"
else
    log_status "binpkgs" "local binpkg reuse disabled (REGICIDE_USE_BINPKGS=0)"
fi

cat > "${ROOTFS}/etc/portage/make.conf" << EOF
COMMON_FLAGS="-O2 -pipe"
CFLAGS="\${COMMON_FLAGS}"
CXXFLAGS="\${COMMON_FLAGS}"
FCFLAGS="\${COMMON_FLAGS}"
FFLAGS="\${COMMON_FLAGS}"
MAKEOPTS="-j8"
NINJAFLAGS="-j8"
SAMUFLAGS="-j8"
CARGO_BUILD_JOBS="8"
USE="wayland dist-kernel fuse flatpak gstreamer lvm networkmanager nls pipewire pipewire-alsa policykit udev usb screencast ${GENTOO_VIDEO_CARDS} vaapi vpx xkb"
FEATURES="parallel-fetch buildpkg -ipc-sandbox -network-sandbox -pid-sandbox -userfetch -usersandbox -userpriv"
ACCEPT_LICENSE="*"
ACCEPT_KEYWORDS="${GENTOO_KEYWORDS}"
EMERGE_DEFAULT_OPTS="${EMERGE_OPTS}"
GENTOO_MIRRORS="${GENTOO_MIRRORS:-https://distfiles.gentoo.org}"
EOF

cat > "${ROOTFS}/etc/portage/package.use/tiff-libwebp" << 'EOF'
media-libs/tiff -webp
media-libs/libwebp -tiff
EOF

cat > "${ROOTFS}/etc/portage/package.use/pillow-cycle" << 'EOF'
dev-python/pillow -truetype
EOF

cat > "${ROOTFS}/etc/portage/package.use/regicide-deps" << 'EOF'
>=net-firewall/iptables-1.8.13 nftables
>=net-libs/ngtcp2-1.22.0 gnutls
>=sys-kernel/installkernel-68 dracut
# zfs requires an older Python target than the system default (3.14);
# its REQUIRED_USE only accepts python3_12/3_13.
sys-fs/zfs PYTHON_TARGETS: python3_13
# podman >= 6 requires container-libs[extra].
app-containers/container-libs extra
# netavark (podman networking) requires nftables[json].
net-firewall/nftables json
EOF

# qemu-guest-agent's meson build opens /var/lib/containers/storage/db.sql for
# writing (podman is installed in an earlier stage), which trips the portage
# sandbox with ACCESS DENIED. Predict the path for that package only.
mkdir -p "${ROOTFS}/etc/portage/env"
cat > "${ROOTFS}/etc/portage/env/allow-container-storage.conf" << 'EOF'
SANDBOX_PREDICT="/var/lib/containers"
EOF
cat > "${ROOTFS}/etc/portage/package.env" << 'EOF'
app-emulation/qemu-guest-agent allow-container-storage.conf
EOF

echo "Syncing Portage tree..."
log_status "sync" "emerge-webrsync"
run_in_chroot emerge-webrsync

echo "Updating @world..."
log_status "world-update" "emerge -uDNq @world"
run_in_chroot emerge -uDNq @world

clean_rootfs_transient
log_status "complete" "portage sync and world update done"
echo "Stage 2 complete."
