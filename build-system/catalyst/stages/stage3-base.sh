#!/bin/bash
# Stage 3: install the base system packages.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage3-base"

log_status "start" "installing base system packages"
PACKAGES=(
    app-admin/sudo
    app-containers/crun
    dev-lang/rust-bin
    app-containers/distrobox
    app-containers/podman
    app-editors/nano
    app-emulation/qemu-guest-agent
    app-emulation/spice-vdagent
    app-eselect/eselect-repository
    app-shells/bash-completion
    dev-util/glib-utils
    dev-util/wayland-scanner
    dev-vcs/git
    media-fonts/fonts-meta
    media-libs/gstreamer
    media-libs/mesa
    media-video/wireplumber
    net-fs/samba
    net-print/cups
    sys-apps/flatpak
    sys-apps/iproute2
    sys-apps/lsb-release
    sys-apps/mlocate
    sys-apps/xdg-desktop-portal
    sys-apps/xdg-desktop-portal-gtk
    sys-auth/fprintd
    sys-auth/rtkit
    sys-block/gparted
    sys-block/io-scheduler-udev-rules
    sys-boot/grub
    sys-boot/plymouth
    sys-fs/bcache-tools
    sys-fs/btrfs-progs
    sys-fs/cryptsetup
    sys-fs/dmraid
    sys-fs/dosfstools
    sys-fs/e2fsprogs
    sys-fs/exfatprogs
    sys-fs/f2fs-tools
    sys-fs/fuse-exfat
    sys-fs/fuse-overlayfs
    sys-fs/fuseiso
    sys-fs/go-mtpfs
    sys-fs/jfsutils
    sys-fs/lsscsi
    sys-fs/lvm2
    sys-fs/mac-fdisk
    sys-fs/mdadm
    sys-fs/multipath-tools
    sys-fs/ntfs3g
    sys-fs/squashfs-tools
    sys-fs/xfsprogs
    sys-fs/zfs
    sys-fs/zfs-kmod
    sys-kernel/gentoo-kernel-bin
    sys-kernel/linux-firmware
    sys-power/power-profiles-daemon
)

for pkg in "${PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "base system packages installed"
echo "Stage 3 complete."
