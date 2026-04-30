# RegicideOS Build System

## Overview

This directory contains the build infrastructure for RegicideOS. The build uses **Catalyst** (Gentoo's official stage builder) to create stage4 tarballs, which are then converted to SquashFS images for live ISO deployment or directly deployed to ROOTS partitions.

## Architecture

```
build-system/
├── catalyst/           # Catalyst specs, scripts, and overlays
│   ├── stage4-systemd-cosmic.spec   # COSMIC desktop stage4 spec
│   ├── stage4-systemd-cosmic.sh     # Post-build configuration
│   ├── build.sh                     # Build wrapper script
│   ├── overlay/                     # Base overlay (repos.conf)
│   └── cosmic-overlay/              # COSMIC-specific portage config
├── dagger_pipeline.py  # Dagger CI/CD orchestration
├── dagger.py           # Legacy aspirational Dagger config (deprecated)
└── regicide_image_builder.py  # Legacy image builder (deprecated)
```

## Quick Start

### Prerequisites

You need a Gentoo system (or chroot) with Catalyst installed:

```bash
emerge -av dev-util/catalyst app-arch/pixz sys-fs/squashfs-tools
```

### Build COSMIC Desktop

```bash
cd build-system/catalyst
sudo ./build.sh
```

This produces a stage4 tarball at `/var/tmp/catalyst/builds/default/`.

### Create Live Image

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

Dagger is used as an **orchestration layer**, not a replacement for Catalyst. The actual OS build is done by Catalyst (proven Gentoo-native tooling). Dagger provides:

- Reproducible CI/CD builds
- Multi-arch orchestration
- Clean environment isolation

### Usage

```bash
dagger run python build-system/dagger_pipeline.py
```

## COSMIC Stage4 Spec

The `stage4-systemd-cosmic.spec` defines:

- **Profile**: `default/linux/amd64/23.0/desktop/systemd`
- **Overlays**: fsvm88/cosmic-overlay (for COSMIC packages), regicide-rust (for RegicideOS tools)
- **Desktop**: COSMIC Desktop from cosmic-overlay
- **RegicideOS tools**: btrmind, portcl, regicide-installer, regicide-ai-tools
- **Post-build**: Enables cosmic-greeter, NetworkManager, PipeWire, Flatpak

## Why Catalyst?

Catalyst is Gentoo's official stage builder. It:
- Handles Portage profiles, USE flags, and package sets correctly
- Creates clean, reproducible stage tarballs
- Supports overlays and custom configuration
- Is the same tool used by Gentoo Release Engineering

The previous aspirational build system (`dagger.py`, `regicide_image_builder.py`) has been deprecated in favor of this proven approach.
