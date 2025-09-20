# COSMIC Desktop Setup for RegicideOS

This guide explains how to build and deploy RegicideOS with COSMIC Desktop on a pure Gentoo base using the Xenia build system.

## Overview

RegicideOS uses the same architecture as Xenia Linux - a pure Gentoo base with COSMIC Desktop packaged into a read-only SquashFS image that boots with BTRFS overlays for writable layers.

## Prerequisites

### 1. Build Environment

Any Gentoo system or chroot with the necessary tools:

```bash
emerge -av dev-util/catalyst app-arch/pixz sys-fs/squashfs-tools
```

### 2. Xenia Catalyst Configuration

Clone and set up the Xenia catalyst repository:

```bash
git clone https://gitlab.com/xenia-group/catalyst ~/xenia-catalyst
ln -s ~/xenia-catalyst/config /var/tmp/catalyst/config
```

## COSMIC Package Setup

### 1. Enable GURU Repository

```bash
eselect repository enable guru
emerge --sync
```

### 2. Create Local Overlay

```bash
mkdir -p /var/db/repos/local-{metadata,profiles}
cat >/var/db/repos/local/metadata/layout.conf <<'EOF'
masters = gentoo
thin-manifests = true
EOF
```

### 3. Add COSMIC Ebuilds

The COSMIC packages are available in GURU. You can either:
- Use them directly from GURU (recommended)
- Copy them to your local overlay for customization

To use directly from GURU, ensure the packages are unmasked in your catalyst configuration.

## Catalyst Spec File

Create a new spec file based on the existing GNOME configuration:

```bash
cd ~/xenia-catalyst
cp stage4-systemd-gnome.spec stage4-systemd-cosmic.spec
```

Edit the new file with these key changes:

### stage4-systemd-cosmic.spec

```spec
# Change target name
target: stage4-amd64-systemd-cosmic

# Add COSMIC packages to the package list
stage4/packages:
  gui-libs/cosmic-panel
  gui-libs/cosmic-workspaces
  gui-apps/cosmic-launcher
  gui-libs/cosmic-session
  gui-apps/cosmic-terminal
  gui-apps/cosmic-files
  gui-apps/cosmic-text-editor
  gui-libs/cosmic-comp
  gui-libs/cosmic-settings-daemon
  gui-libs/cosmic-applets
  gui-libs/cosmic-bg
  gui-libs/cosmic-greeter
  gui-libs/cosmic-icons
  gui-libs/cosmic-theme
  gui-libs/cosmic-wallpapers

# Add cosmic-session to default runlevel
stage4/rc:
  cosmic-session

# Optional: Remove GNOME components if desired
# Remove or comment out gnome-base/gnome-shell
```

## Build Process

### 1. Run Catalyst

```bash
catalyst -a -f stage4-systemd-cosmic.spec
```

This will take a while (go grab coffee). The output will be:
```
/var/tmp/catalyst/builds/default/stage4-amd64-systemd-cosmic.tar.xz
```

### 2. Create SquashFS Image

```bash
mkdir /tmp/cosmic
tar -C /tmp/cosmic -xpJf /var/tmp/catalyst/builds/default/stage4-amd64-systemd-cosmic.tar.xz
mksquashfs /tmp/cosmic root-cosmic.img -comp zstd -Xcompression-level 19
```

## Deployment

### 1. On Existing RegicideOS Installation

```bash
mount /dev/disk/by-label/ROOTS /mnt
cp root-cosmic.img /mnt/
sync
umount /mnt
```

### 2. During Installation

Follow the standard RegicideOS installation process, but use your custom `root-cosmic.img` instead of the default image.

### 3. GRUB Configuration

GRUB will automatically detect the new image by modification time. The boot entry will look like:

```grub
set root=(hd0,gpt2)
linux /boot/vmlinuz-<version> root=LABEL=ROOTS overlay=LABEL=OVERLAY home=LABEL=HOME quiet
initrd /boot/initramfs-<version>.img
```

## Post-Installation Setup

### 1. First Boot

- Select the COSMIC image in GRUB (newest by mtime)
- Complete the COSMIC initial setup wizard
- Configure your user account and preferences

### 2. Verify COSMIC Components

```bash
# Check if cosmic-session is running
systemctl --user status cosmic-session

# Verify cosmic components
ps aux | grep cosmic
```

## Maintenance and Updates

### 1. Updating the Build Environment

```bash
# Sync Portage tree
emerge --sync

# Update catalyst configuration if needed
cd ~/xenia-catalyst
git pull
```

### 2. Rebuilding the Image

```bash
# Update packages in the chroot
# Test changes if needed

# Rebuild with catalyst
catalyst -a -f stage4-systemd-cosmic.spec

# Create new SquashFS
mksquashfs /tmp/cosmic root-cosmic-$(date +%Y%m%d).img -comp zstd -Xcompression-level 19
```

### 3. Live Testing

Because RegicideOS uses BTRFS overlays, you can test package changes live:

```bash
# Install test packages in the running system
emerge -av test-package

# If successful, add to catalyst spec for next build
```

## Troubleshooting

### 1. Catalyst Build Failures

```bash
# Check catalyst logs
less /var/tmp/catalyst/logs/stage4-systemd-cosmic.log

# Common issues:
# - Missing dependencies in spec file
# - Network connectivity problems
# - Disk space issues
```

### 2. COSMIC Session Issues

```bash
# Check session logs
journalctl --user -u cosmic-session

# Verify display manager
systemctl status display-manager

# Check X11/Wayland status
loginctl show-session $XDG_SESSION_ID
```

### 3. Boot Problems

```bash
# Use recovery mode if COSMIC fails to start
# Boot with overlay=disabled to get clean system

# Check boot logs
journalctl -b -1
```

## Advanced Configuration

### 1. Custom COSMIC Settings

Create `/etc/cosmic/config.d/custom.conf`:

```toml
[workspace]
auto-arrange = true

[panel]
position = "top"
auto-hide = false

[wallpaper]
path = "/usr/share/wallpapers/cosmic/default.jpg"
```

### 2. Performance Tuning

```bash
# Enable BTRFS compression for COSMIC files
btrfs property set /mnt/roots/@usr compression zstd

# Optimize BTRFS for COSMIC usage
btrfs balance start -dusage=20 /mnt/roots
```

### 3. Multiple COSMIC Versions

Keep multiple COSMIC images for easy rollback:

```bash
ls -la /mnt/root-cosmic-*.img
# Boot with the version you want in GRUB
```

## Integration with RegicideOS Features

### 1. AI System Management

The PortCL AI agent works seamlessly with COSMIC:

```bash
# Enable AI optimization for COSMIC
systemctl enable --now portcl-agent

# Monitor COSMIC performance
portcl-monitor --dashboard
```

### 2. Snapshot Management

Use BTRFS snapshots with COSMIC:

```bash
# Snapshot before COSMIC updates
btrfs subvolume snapshot -r /mnt/roots/@etc /mnt/roots/@etc-cosmic-pre-update

# Roll back if issues occur
btrfs subvolume delete /mnt/roots/@etc
btrfs subvolume snapshot /mnt/roots/@etc-cosmic-pre-update /mnt/roots/@etc
```

## Conclusion

RegicideOS with COSMIC Desktop provides:
- **Pure Gentoo base** with full Portage compatibility
- **Immutable system** with BTRFS overlays for safety
- **Modern Rust-based desktop** with Wayland support
- **AI-powered optimization** through PortCL
- **Easy updates and rollbacks** via SquashFS images
- **High performance** with Zstd compression and BTRFS optimization

This approach gives you the best of both worlds: Gentoo's flexibility and control, with COSMIC's modern desktop experience and RegicideOS's innovative architecture.