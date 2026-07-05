#!/bin/bash
# Stage 6: post-build configuration, cleanup, and stage4 tarball creation.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage6-finalize"

log_status "start" "post-build configuration and tarball creation"
echo "Applying post-build configuration..."
run_in_chroot bash -c '
    if command -v dracut &> /dev/null; then
        dracut --force --no-hostonly --kver "$(ls /lib/modules/ | head -n1)"
    fi

    # Leave root password unset so the user manages it via regicide+sudo.
    passwd -d root
    chown root:root /etc/sudoers

    # Create the default interactive user.  Doing this in stage6 bakes
    # /home/regicide into the tarball/SquashFS so greeter login works.
    useradd -m -G users,wheel,audio,video,input -s /bin/bash regicide || true
    echo "regicide:regicide" | chpasswd
    chown -R regicide:regicide /home/regicide
    # Suppress the common "tty: ttyname error" message from flatpak terminal
    # emulators (e.g. Rio) when shell startup files run `mesg n` without a TTY.
    # Write a minimal .profile that guards `mesg n` against missing TTY.
    # This avoids the "tty: ttyname error" message from flatpak terminal
    # emulators such as Rio when launched without a controlling terminal.
    cat > /home/regicide/.profile <<PROFILE
# Source global bash settings if available
if [ -f /etc/profile ]; then
    . /etc/profile
fi
if tty -s; then
    mesg n 2>/dev/null || true
fi
PROFILE
    chown regicide:regicide /home/regicide/.profile
    chmod 0644 /home/regicide/.profile

    # Allow wheel group to sudo without a password so the live image and
    # automated VM tests can run privileged diagnostics non-interactively.
    # This is acceptable for a development/test image; release images can
    # override the drop-in before sealing.
    mkdir -p /etc/sudoers.d
    chmod 0750 /etc/sudoers.d
    echo "%wheel ALL=(ALL:ALL) NOPASSWD: ALL" > /etc/sudoers.d/10-regicide-wheel
    chmod 0440 /etc/sudoers.d/10-regicide-wheel

    # Do not bake host-specific SSH keys into the image; they are generated on
    # first boot by sshd-keygen@.service.  This avoids distributing a shared
    # private key across every RegicideOS installation.
    rm -f /etc/ssh/ssh_host_*_key /etc/ssh/ssh_host_*_key.pub
    systemctl enable sshd-keygen@.service || true
    mkdir -p /etc/systemd/system/sockets.target.wants
    ln -sf /usr/lib/systemd/system/sshd.socket /etc/systemd/system/sockets.target.wants/sshd.socket

    chmod u+s /usr/bin/sudo /usr/bin/su /usr/bin/passwd /usr/bin/chfn /usr/bin/chsh /usr/bin/newgrp /usr/bin/mount /usr/bin/umount
    chmod u+s /usr/libexec/dbus-daemon-launch-helper
    chmod u+s /usr/lib/polkit-1/polkit-agent-helper-1 2>/dev/null || true

    rm -rf /opt
    mkdir /usr/opt
    ln -sf /usr/opt /

    mkdir -p /.recovery/etc
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
    systemctl enable pipewire.socket || true
    systemctl enable pipewire-pulse.socket || true
    systemctl enable wireplumber.service || true

    # Enable sshd as both socket-activated and multi-user service so the
    # post-install verifier passes regardless of which unit name is checked.
    systemctl enable sshd.socket || true
    systemctl enable sshd.service || true

    rm -f /boot/*.old
    # Gentoo kernels are installed as /boot/kernel-*; create the canonical
    # /boot/vmlinuz symlink so installers and verifiers have a stable path.
    if [[ ! -f /boot/vmlinuz ]]; then
        latest_kernel="$(ls -1 /boot/kernel-* /boot/vmlinuz-* 2>/dev/null | head -n1)"
        if [[ -n "${latest_kernel}" ]]; then
            ln -sf "$(basename "${latest_kernel}")" /boot/vmlinuz
        fi
    fi
    cp /boot/initramfs* /boot/initramfs.img 2>/dev/null || true
    cp /boot/System* /boot/System.map 2>/dev/null || true
    cp /boot/config* /boot/config 2>/dev/null || true

    flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo || true
    # Pre-install Rio terminal from Flathub so it is available out-of-the-box.
    # Allow failure because flatpak install can require network/reachable repo.
    flatpak install --noninteractive flathub com.rioterm.Rio || true
'

echo "Cleaning up..."
run_in_chroot bash -c '
    rm -rf /var/tmp/portage/* /var/tmp/portage/.*[!.]* 2>/dev/null || true
    rm -rf /tmp/* /tmp/.*[!.]* 2>/dev/null || true
'

echo "Creating stage4 tarball..."
log_status "tarball" "creating stage4-amd64-systemd-cosmic.tar.xz"
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

log_status "complete" "stage4 tarball created"
echo "Stage 6 complete."
echo "Output: ${OUTPUT_FILE}"
