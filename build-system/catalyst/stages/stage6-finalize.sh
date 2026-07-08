#!/bin/bash
# Stage 6: post-build configuration, cleanup, and stage4 tarball creation.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage6-finalize"

log_status "start" "post-build configuration and tarball creation"
echo "Applying post-build configuration..."
run_in_chroot bash <<'STAGE6EOF'
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

    # Portage configuration must be readable by regular users so commands like
    # `emerge --info` and `emerge -pv <pkg>` work for the default desktop user.
    # Catalyst creates /etc/portage as 0700, which blocks traversal for non-root.
    chmod 0755 /etc/portage
    chmod -R go+rX /etc/portage

    # Make key /etc files writable by the default desktop user so the immutable
    # overlay behaves more like a regular system for day-to-day administration.
    # This matches the Xenia Linux installer convention of giving the created
    # user ownership of hosts/fstab and Portage configuration.
    chown regicide:regicide /etc/hosts /etc/fstab
    chmod 0664 /etc/hosts /etc/fstab
    chown regicide:regicide /etc/portage/make.conf
    chmod 0664 /etc/portage/make.conf

    # The COSMIC overlay ships a README at the root of the stage4 rootfs. Rename
    # it so it is clear it belongs to the overlay, and keep it world-readable.
    if [[ -f /README.md ]]; then
        mv /README.md /README_COSMIC_OVERLAY.md
    fi
    chmod 0644 /README_COSMIC_OVERLAY.md 2>/dev/null || true

    # Disable sshd reverse-DNS lookups so SSH logins do not hang when DNS is
    # temporarily unavailable inside the VM.
    if ! grep -q '^UseDNS no' /etc/ssh/sshd_config 2>/dev/null; then
        echo 'UseDNS no' >> /etc/ssh/sshd_config
    fi

    # Make glibc NSS fall back to files/dns when systemd-resolved is not active.
    # The default Gentoo line uses `[!UNAVAIL=return]` which breaks name
    # resolution whenever resolved is disabled.
    if [[ -f /etc/nsswitch.conf ]]; then
        sed -i 's/resolve \[!UNAVAIL=return\] files myhostname dns/resolve files myhostname dns/' /etc/nsswitch.conf
    fi

    # Do not bake host-specific SSH keys into the image.  sshd.socket uses
    # Accept=yes and spawns sshd -i per connection, which does not create missing
    # host keys.  ssh-keygen -A also fails on overlay /etc with EXDEV.  Use a
    # dedicated one-shot service that generates keys directly into /etc/ssh
    # before sshd.socket starts.
    rm -f /etc/ssh/ssh_host_*_key /etc/ssh/ssh_host_*_key.pub

    mkdir -p /usr/lib/regicide
    cat > /usr/lib/regicide/regicide-ssh-keygen <<'KEYGENEOF'
#!/bin/bash
# RegicideOS first-boot SSH host key generator.
# Runs before sshd.socket and creates host keys if they do not exist.
set -euo pipefail
mkdir -p /etc/ssh
# /etc is an overlayfs lowerdir in the runtime image; chmod on an existing
# lowerdir directory can fail with EXDEV (Invalid cross-device link). The
# directory is already 0755 in the stage4 rootfs, so tolerate the failure.
chmod 0755 /etc/ssh 2>/dev/null || true
# ssh-keygen creates a temporary file in the target directory and renames it
# into place. On overlayfs this rename can fail with EXDEV, so generate keys
# in a fixed tmpfs-backed temporary path and copy the result to /etc/ssh.
# Use a literal path (no variables) in the trap so the heredoc is safe even
# if the enclosing delimiter quotes are stripped during container transport.
KEY_TMP="/tmp/regicide-ssh-tmp"
rm -rf "${KEY_TMP}"
mkdir -p "${KEY_TMP}"
trap 'rm -rf /tmp/regicide-ssh-tmp' EXIT
if [[ ! -f /etc/ssh/ssh_host_ed25519_key ]]; then
    ssh-keygen -t ed25519 -f "${KEY_TMP}/ssh_host_ed25519_key" -N "" -C "regicideos-$(date -u +%Y%m%d%H%M%S)" >/dev/null 2>&1
    # Use shell redirection instead of cp/mv; overlayfs can reject renames
    # and hard links across the lower/upper boundary with EXDEV.
    cat "${KEY_TMP}/ssh_host_ed25519_key" > /etc/ssh/ssh_host_ed25519_key
    cat "${KEY_TMP}/ssh_host_ed25519_key.pub" > /etc/ssh/ssh_host_ed25519_key.pub
fi
if [[ ! -f /etc/ssh/ssh_host_rsa_key ]]; then
    ssh-keygen -t rsa -b 4096 -f "${KEY_TMP}/ssh_host_rsa_key" -N "" -C "regicideos-$(date -u +%Y%m%d%H%M%S)" >/dev/null 2>&1
    cat "${KEY_TMP}/ssh_host_rsa_key" > /etc/ssh/ssh_host_rsa_key
    cat "${KEY_TMP}/ssh_host_rsa_key.pub" > /etc/ssh/ssh_host_rsa_key.pub
fi
chmod 0600 /etc/ssh/ssh_host_*_key
chmod 0644 /etc/ssh/ssh_host_*_key.pub
KEYGENEOF
    chmod 0755 /usr/lib/regicide/regicide-ssh-keygen

    mkdir -p /usr/lib/systemd/system
    cat > /usr/lib/systemd/system/regicide-ssh-keygen.service <<'SVCEOF'
[Unit]
Description=RegicideOS first-boot SSH host key generator
DefaultDependencies=no
Before=sshd.socket
Before=ssh.service
After=systemd-tmpfiles-setup.service

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/usr/lib/regicide/regicide-ssh-keygen
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=sockets.target
SVCEOF

    mkdir -p /etc/systemd/system/sockets.target.wants
    ln -sf /usr/lib/systemd/system/sshd.socket /etc/systemd/system/sockets.target.wants/sshd.socket
    systemctl enable regicide-ssh-keygen.service || true
    # Ensure the standalone sshd.service does not start and bind port 22 in
    # parallel with socket activation.
    systemctl disable sshd.service 2>/dev/null || true

    chmod u+s /usr/bin/sudo /usr/bin/su /usr/bin/passwd /usr/bin/chfn /usr/bin/chsh /usr/bin/newgrp /usr/bin/mount /usr/bin/umount
    chmod u+s /usr/bin/newuidmap /usr/bin/newgidmap
    chmod u+s /usr/libexec/dbus-daemon-launch-helper
    chmod u+s /usr/lib/polkit-1/polkit-agent-helper-1 2>/dev/null || true
    # pam_unix uses unix_chkpwd to verify passwords when the calling process
    # is not root. cosmic-greeter runs as the logged-in user, so this helper
    # must be setuid or graphical login fails while SSH still works.
    chmod u+s /sbin/unix_chkpwd /usr/sbin/unix_chkpwd 2>/dev/null || true

    rm -rf /opt
    mkdir /usr/opt
    ln -sf /usr/opt /

    mkdir -p /.recovery/etc
    cp /etc/passwd /.recovery/etc/passwd
    cp /etc/shadow /.recovery/etc/shadow
    echo "recovery:x:1001:1001::/home/recovery:/bin/bash" >> /.recovery/etc/passwd
    echo "recovery:x:1001:" >> /.recovery/etc/group
    chown 1001:1001 -R /.recovery/home/recovery

    chown portage:portage /var/cache/distfiles

    # Generate only the locale we need to keep the image smaller.
    cat > /etc/locale.gen << EOF
en_US.UTF-8 UTF-8
EOF
    locale-gen

    # Ensure home directory ownership is correct even if extraction/creation
    # left it as root or another uid.
    chown 1000:1000 -R /home/regicide || true

    # COSMIC defaults: active window hint off, screen reader disabled,
    # and UI event sounds off for a quieter first-boot experience.
    mkdir -p /home/regicide/.config/cosmic/com.system76.CosmicComp/v1
    printf false > /home/regicide/.config/cosmic/com.system76.CosmicComp/v1/active_hint

    # Pin Rio terminal to the dock alongside the default COSMIC apps.
    mkdir -p /home/regicide/.config/cosmic/com.system76.CosmicAppList/v1
    cat > /home/regicide/.config/cosmic/com.system76.CosmicAppList/v1/favorites <<'FAVEOF'
[
  "com.system76.CosmicAppList",
  "com.system76.CosmicFiles",
  "com.system76.CosmicEdit",
  "com.system76.CosmicTerminal",
  "com.rioterm.Rio",
  "com.system76.CosmicSettings"
]
FAVEOF

    chown -R regicide:regicide /home/regicide/.config

    # Disable the Orca screen reader and GNOME/COSMIC event sounds by default.
    mkdir -p /etc/dconf/db/local.d
    cat > /etc/dconf/db/local.d/00-regicide-a11y << EOF
[org/gnome/desktop/a11y/applications]
screen-reader-enabled=false

[org/gnome/desktop/sound]
event-sounds=false
EOF
    dconf update 2>/dev/null || true

    systemctl enable bluetooth || true
    systemctl enable NetworkManager || true
    # Work around overlayfs whiteouts that can hide the NetworkManager unit in
    # /etc/systemd/system: ensure the wants symlink points directly at the unit
    # file in /usr/lib/systemd/system rather than a relative path through /etc.
    ln -sf /usr/lib/systemd/system/NetworkManager.service /etc/systemd/system/multi-user.target.wants/NetworkManager.service
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

    # Use socket activation for sshd so host keys are generated by
    # sshd-keygen@.service before the first connection is accepted.
    systemctl enable sshd.socket || true

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
    # Pre-install Rio terminal and OpenCode from Flathub so they are available
    # out-of-the-box. Allow failure because flatpak install can require a
    # network/reachable repo.
    flatpak install --noninteractive flathub com.rioterm.Rio ai.opencode.opencode || true
STAGE6EOF

echo "Cleaning up..."
run_in_chroot bash <<'STAGE6CLEANEOF'
    rm -rf /var/tmp/portage/* /var/tmp/portage/.*[!.]* 2>/dev/null || true
    rm -rf /tmp/* /tmp/.*[!.]* 2>/dev/null || true
STAGE6CLEANEOF

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
