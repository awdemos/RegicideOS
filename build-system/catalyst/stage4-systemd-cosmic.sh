#!/bin/bash
# RegicideOS COSMIC Desktop post-build script
# Based on Xenia Linux stage4-systemd.sh, adapted for RegicideOS

# Ensure dracut can unlock LUKS-encrypted root when the image is built with
# the --encrypt option. The crypt module and /etc/crypttab are included so
# systemd in the initramfs can prompt for the passphrase at boot.
mkdir -p /etc/dracut.conf.d
cat > /etc/dracut.conf.d/99-regicide-luks.conf << 'EOF'
add_dracutmodules+=" crypt "
install_items+=" /etc/crypttab "
EOF

dracut --force --no-hostonly --kver $(ls /lib/modules/)

# Dracut's switch-root executes /sbin/init.  In a merged-/usr layout
# (/sbin -> usr/bin and /usr/sbin -> bin -> usr/bin) the real binary must
# live in /usr/bin.  Create the canonical systemd symlink if it is missing.
if [[ -x /usr/lib/systemd/systemd && ! -e /usr/bin/init ]]; then
    ln -sf /usr/lib/systemd/systemd /usr/bin/init
fi

# Set default root password (user should change this)
echo "root:regicide" | chpasswd
chown root:root /etc/sudoers

rm -rf /opt
mkdir /usr/opt
ln -sf /usr/opt /

mkdir -p /.recovery/etc /.recovery/home/recovery

cp /etc/passwd /.recovery/etc/passwd
cp /etc/shadow /.recovery/etc/shadow

echo "recovery:x:1000:1000::/home/recovery:/bin/bash" >> /.recovery/etc/passwd
echo "recovery:$6$ovJXS/P4rKaURNaD$IUmaP2JW5uiJgrFVr31bEMb6kEF.ARL.x23m.qvyJ3.oRRbJ1qQ/pU5R2VocEzunYqSGF/YvLFGqF5gn0BQY90:19574::::::" >> /.recovery/etc/shadow

sed s/wheel:x:10:root/wheel:x:10:root,recovery/ /etc/group > /.recovery/etc/group
echo "recovery:x:1000:" >> /.recovery/etc/group

chown 1000:1000 -R /.recovery/home/recovery

chown portage:portage /var/cache/distfiles

cp /usr/share/i18n/SUPPORTED /etc/locale.gen
locale-gen

systemctl enable bluetooth
systemctl enable NetworkManager
systemctl enable cups
systemctl enable systemd-timesyncd
systemctl enable cosmic-greeter
systemctl enable lvm2-monitor
systemctl enable qemu-guest-agent
systemctl enable spice-vdagentd
systemctl enable zfs.target
systemctl enable zfs-import-cache
systemctl enable zfs-mount
systemctl enable zfs-import.target

systemctl --global enable pipewire.socket pipewire-pulse.socket wireplumber.service

rm /boot/*.old

cp /boot/vmlinuz* /boot/vmlinuz
cp /boot/initramfs* /boot/initramfs.img
cp /boot/System* /boot/System.map
cp /boot/config* /boot/config

flatpak remote-add flathub https://flathub.org/repo/flathub.flatpakrepo

eselect repository add regicide-rust git https://github.com/awdemos/RegicideOS.git
emaint sync --repo regicide-rust

chown --from=1001:1001 root:root /etc -R
chown --from=1001:1001 root:root /
chown --from=1001:1001 root:root /boot -R
chown --from=1001:1001 root:root /overlay -R
chown --from=1001:1001 root:root /roots -R
chown --from=1001:1001 root:root /usr -R
chown --from=1001:1001 root:root /var -R

chown --from=1000:1000 root:root /etc -R
chown --from=1000:1000 root:root /
chown --from=1000:1000 root:root /boot -R
chown --from=1000:1000 root:root /overlay -R
chown --from=1000:1000 root:root /roots -R
chown --from=1000:1000 root:root /usr -R
chown --from=1000:1000 root:root /var -R
