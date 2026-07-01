# RegicideOS Build System

## Overview

This directory contains the build infrastructure for RegicideOS. The build uses **Catalyst** (Gentoo's official stage builder) to create stage4 tarballs, which are then converted to SquashFS images for live ISO deployment or directly deployed to ROOTS partitions.

## Architecture

```
build-system/
├── catalyst/           # Catalyst specs, scripts, and overlays
│   ├── stage4-systemd-cosmic.spec   # COSMIC desktop stage4 spec
│   ├── stage4-systemd-cosmic.sh     # Post-build configuration
│   ├── build.sh                     # Catalyst build wrapper script
│   ├── build-vm-image.sh            # Bootable QCOW2 image builder
│   ├── overlay/                     # Base overlay (repos.conf)
│   └── cosmic-overlay/              # COSMIC-specific portage config
├── dagger_pipeline.py  # Dagger CI/CD orchestration
├── dagger.py           # Legacy aspirational Dagger config (deprecated)
└── regicide_image_builder.py  # Legacy image builder (deprecated)
```

## Quick Start

### Prerequisites

You need a Gentoo system (or chroot) with Catalyst and the VM builder tools installed:

```bash
emerge -av dev-util/catalyst app-arch/pixz sys-fs/squashfs-tools
emerge -av app-emulation/qemu sys-fs/cryptsetup
```

### Build COSMIC Desktop

```bash
cd build-system/catalyst
sudo ./build.sh
```

This produces:
- a stage4 tarball at `/var/tmp/catalyst/builds/default/stage4-amd64-systemd-cosmic.tar.xz`
- a live SquashFS image at `build-system/catalyst/output/regicide-cosmic.img`

### Build a Bootable QCOW2 VM Image

`build-vm-image.sh` creates a fully bootable QCOW2 disk image from the Catalyst stage4 tarball. It boots the stage4 rootfs inside a KVM VM and runs `build-qemu-image.sh` against a virtio block device, so it works in environments without loop device support.

```bash
cd build-system/catalyst

# Basic unencrypted image
sudo ./build-vm-image.sh \
    --squashfs output/regicide-cosmic.img \
    output/stage4-amd64-systemd-cosmic.tar.xz \
    output/regicide-cosmic.qcow2 20G

# Encrypted image with LUKS2
printf 'regicide-secure-test' > /tmp/regicide-passphrase.txt
sudo ./build-vm-image.sh \
    --encrypt --passphrase-file /tmp/regicide-passphrase.txt \
    --squashfs output/regicide-cosmic.img \
    output/stage4-amd64-systemd-cosmic.tar.xz \
    output/regicide-cosmic-enc.qcow2 20G
```

> **Current status**: both `output/regicide-cosmic.qcow2` (unencrypted) and `output/regicide-cosmic-enc.qcow2` (encrypted) build and boot to a serial-console `regicideos login:` prompt, and `systemd-logind` starts successfully (verified on both). The default root password is `regicide`; the example LUKS passphrase is `regicide-secure-test`. The COSMIC Desktop packages are listed in the stage4 spec but are not installed because the local `cosmic-overlay/` contains only Portage configuration and no ebuilds; `cosmic-greeter` is enabled but cannot start until the ebuilds are present.

Arguments:
- `stage4-tarball` (required): path to the stage4 `.tar.xz` tarball
- `output-qcow2` (optional): path for the output `.qcow2` file. Default is `output/regicide-qemu.qcow2`. Example verified outputs are `output/regicide-cosmic.qcow2` (unencrypted) and `output/regicide-cosmic-enc.qcow2` (encrypted).
- `disk-size` (optional): disk size for the image, e.g. `20G` (default: `20G`)
- `--encrypt`: encrypt the ROOTS partition with LUKS2
- `--passphrase-file`: path to a file containing the LUKS passphrase (required with `--encrypt`; use `-` for stdin)
- `--squashfs`: path to the live SquashFS image used to extract the kernel and initramfs (optional; the script discovers the SquashFS next to the stage4 tarball or in common output directories)

A SPICE display is exposed on port `5920` during the VM build. Watch the build with:

```bash
remote-viewer spice://localhost:5920
```

### Boot the QCOW2 Image

#### Serial console

Boot the generated image with UEFI and a serial console. The unencrypted image is `output/regicide-cosmic.qcow2`; the encrypted image is `output/regicide-cosmic-enc.qcow2`.

```bash
pkill -9 -f 'qemu-system-x86_64' 2>/dev/null || true
cp /usr/share/OVMF/OVMF_VARS.fd /tmp/ovmf-vars.fd
qemu-system-x86_64 \
    -enable-kvm \
    -m 8G \
    -smp 4 \
    -cpu host \
    -machine type=q35,accel=kvm \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
    -drive if=pflash,format=raw,file=/tmp/ovmf-vars.fd \
    -nographic \
    -hda output/regicide-cosmic.qcow2
```

For encrypted images, enter the LUKS passphrase `regicide-secure-test` when prompted. Log in as `root` / `regicide` and run `systemctl status systemd-logind` to verify it is `active (running)`.

#### GUI window (SPICE)

To observe the VM in a graphical window, launch it with a SPICE display instead of `-nographic`:

```bash
cp /usr/share/OVMF/OVMF_VARS.fd /tmp/ovmf-gui.fd
qemu-system-x86_64 \
    -enable-kvm -m 8G -smp 4 -cpu host -machine type=q35,accel=kvm \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
    -drive if=pflash,format=raw,file=/tmp/ovmf-gui.fd \
    -vga qxl -spice port=5920,disable-ticketing=on \
    -serial file:/tmp/regicide-gui-serial.log \
    -hda output/regicide-cosmic.qcow2
```

Then attach `remote-viewer`:

```bash
remote-viewer spice://localhost:5920
```

For encrypted images, enter the LUKS passphrase `regicide-secure-test` in the VM window. If `remote-viewer`/GTK fails (common when `/tmp` is full), use the serial-console command above.

### Create Live Image Manually

If you only need the SquashFS and have already built the stage4 tarball:

```bash
mkdir /tmp/cosmic-root
tar -C /tmp/cosmic-root -xpJf /var/tmp/catalyst/builds/default/stage4-amd64-systemd-cosmic-*.tar.xz
mksquashfs /tmp/cosmic-root regicide-cosmic.img -comp zstd -Xcompression-level 19
```

### Deploy to ROOTS

```bash
mount /dev/disk/by-label/ROOTS /mnt
cp regicide-cosmic.img /mnt/roots/
sync
umount /mnt
```

## Dagger CI/CD

Dagger is used as an **orchestration layer**, not a replacement for Catalyst. The actual OS build is done by `build-system/catalyst/stages/*.sh` scripts inside a Gentoo stage3 container. Dagger provides:

- Reproducible CI/CD builds
- Per-stage caching via separate `withExec` calls
- Multi-arch orchestration
- Clean environment isolation

### Usage

```bash
dagger run python build-system/dagger_pipeline.py
```

The pipeline runs six cacheable stages:

1. `stages/stage1-setup.sh` — stage3 seed and Portage snapshot
2. `stages/stage2-sync.sh` — Portage sync and `@world` update
3. `stages/stage3-base.sh` — base system packages
4. `stages/stage4-cosmic.sh` — COSMIC desktop packages
5. `stages/stage5-regicide.sh` — RegicideOS tools
6. `stages/stage6-finalize.sh` — post-build config and stage4 tarball

> **Note**: the COSMIC stage compiles many Rust packages from source. The first build can take several hours. Subsequent runs reuse the `distfiles` and `binpkgs` Dagger cache volumes, so they are much faster.

To reuse an existing stage4 tarball or SquashFS, pass:

```bash
dagger run python build-system/dagger_pipeline.py --from-tarball ./output/stage4-amd64-systemd-cosmic.tar.xz --from-squashfs ./output/regicide-cosmic.img
```

## COSMIC Stage4 Spec

The `stage4-systemd-cosmic.spec` defines:

- **Profile**: `default/linux/amd64/23.0/desktop/systemd`
- **Overlays**: fsvm88/cosmic-overlay (for COSMIC packages), regicide-rust (for RegicideOS tools)
- **Desktop**: COSMIC Desktop from cosmic-overlay
- **RegicideOS tools**: btrmind, regicide-installer
- **Post-build**: Enables cosmic-greeter, NetworkManager, PipeWire, Flatpak

## Why Catalyst?

Catalyst is Gentoo's official stage builder. It:
- Handles Portage profiles, USE flags, and package sets correctly
- Creates clean, reproducible stage tarballs
- Supports overlays and custom configuration
- Is the same tool used by Gentoo Release Engineering

The previous aspirational build system (`dagger.py`, `regicide_image_builder.py`) has been deprecated in favor of this proven approach.
