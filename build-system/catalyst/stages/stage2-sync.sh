#!/bin/bash
# Stage 2: sync Portage and update @world.
set -euo pipefail

source "$(dirname "$0")/common.sh"

mkdir -p "${ROOTFS}/etc/portage"

cat > "${ROOTFS}/etc/portage/make.conf" << EOF
COMMON_FLAGS="-O2 -pipe"
CFLAGS="\${COMMON_FLAGS}"
CXXFLAGS="\${COMMON_FLAGS}"
FCFLAGS="\${COMMON_FLAGS}"
FFLAGS="\${COMMON_FLAGS}"
MAKEOPTS="-j12"
NINJAFLAGS="-j12"
SAMUFLAGS="-j12"
USE="wayland dist-kernel fuse flatpak gstreamer lvm networkmanager nls pipewire pipewire-alsa policykit udev usb screencast video_cards_amdgpu video_cards_intel video_cards_nouveau video_cards_radeon video_cards_radeonsi video_cards_virgl video_cards_vmware vaapi vpx xkb"
FEATURES="parallel-fetch buildpkg -ipc-sandbox -network-sandbox -pid-sandbox -userfetch -usersandbox -userpriv"
ACCEPT_LICENSE="*"
EMERGE_DEFAULT_OPTS="--jobs --load-average"
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
EOF

echo "Syncing Portage tree..."
run_in_chroot emerge-webrsync

echo "Updating @world..."
run_in_chroot emerge -uDNq @world

echo "Stage 2 complete."
