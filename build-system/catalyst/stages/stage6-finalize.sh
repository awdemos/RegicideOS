#!/bin/bash
# Stage 6: post-build configuration, cleanup, and stage4 tarball creation.
set -euo pipefail

source "$(dirname "$0")/common.sh"

echo "Applying post-build configuration..."
run_in_chroot bash -c '
    if command -v dracut &> /dev/null; then
        dracut --force --no-hostonly --kver "$(ls /lib/modules/ | head -n1)"
    fi

    echo "root:regicide" | chpasswd
    chown root:root /etc/sudoers

    rm -rf /opt
    mkdir /usr/opt
    ln -sf /usr/opt /

    cp /etc/passwd /.recovery/etc/passwd
    cp /etc/shadow /.recovery/etc/shadow
    echo "recovery:x:1000:1000::/home/recovery:/bin/bash" >> /.recovery/etc/passwd
    echo "recovery:x:1000:" >> /.recovery/etc/group
    chown 1000:1000 -R /.recovery/home/recovery

    chown portage:portage /var/cache/distfiles

    cp /usr/share/i18n/SUPPORTED /etc/locale.gen
    locale-gen

    systemctl enable bluetooth || true
    systemctl enable NetworkManager || true
    systemctl enable cups || true
    systemctl enable systemd-timesyncd || true
    systemctl enable cosmic-greeter || true
    systemctl enable lvm2-monitor || true
    systemctl enable qemu-guest-agent || true
    systemctl enable spice-vdagentd || true
    systemctl enable zfs.target || true
    systemctl enable zfs-import-cache || true
    systemctl enable zfs-mount || true
    systemctl enable zfs-import.target || true

    systemctl --global enable pipewire.socket pipewire-pulse.socket wireplumber.service || true

    rm -f /boot/*.old
    cp /boot/vmlinuz* /boot/vmlinuz 2>/dev/null || true
    cp /boot/initramfs* /boot/initramfs.img 2>/dev/null || true
    cp /boot/System* /boot/System.map 2>/dev/null || true
    cp /boot/config* /boot/config 2>/dev/null || true

    flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo || true
'

echo "Cleaning up..."
run_in_chroot bash -c '
    rm -rf /var/tmp/portage/* /var/tmp/portage/.*[!.]* 2>/dev/null || true
    rm -rf /tmp/* /tmp/.*[!.]* 2>/dev/null || true
'

echo "Creating stage4 tarball..."
mkdir -p "${OUTPUT_DIR}"
OUTPUT_FILE="${OUTPUT_DIR}/stage4-amd64-systemd-cosmic.tar.xz"

tar -C "${ROOTFS}" -cpJf "${OUTPUT_FILE}" \
    --xattrs-include='*.*' \
    --numeric-owner \
    --exclude='./var/tmp/*' \
    --exclude='./tmp/*' \
    --exclude='./var/cache/distfiles/*' \
    --exclude='./var/cache/binpkgs/*' \
    .

echo "Stage 6 complete."
echo "Output: ${OUTPUT_FILE}"
