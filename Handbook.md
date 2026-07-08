# RegicideOS Handbook

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [System Requirements](#2-system-requirements)
3. [Installation](#3-installation)
4. [System Architecture](#4-system-architecture)
5. [Core Components](#5-core-components)
6. [Package Management](#6-package-management)
7. [Development Environment](#7-development-environment)
8. [System Administration](#8-system-administration)
9. [Troubleshooting](#9-troubleshooting)

---

## 1. Introduction

### 1.1 What is RegicideOS?

RegicideOS is a specialized Gentoo-based Linux distribution focused on:

- **Rust-First Architecture**: System components migrated to Rust for memory safety and performance
- **Immutable System Architecture**: Read-only Btrfs root with atomic updates and instant rollback
- **AI-Integrated**: AI capabilities at the system level for predictive maintenance and context-aware assistance
- **Reproducible Source Builds**: Built locally from a Gentoo stage4 with the Catalyst/Dagger pipeline
- **Privacy by Default**: LUKS encryption recommended, local-first control, minimal telemetry

### 1.2 Key Differentiators

| Feature | Traditional Distros | RegicideOS |
|---------|---------------------|------------|
| **Desktop Environment** | Multiple choices | Cosmic Desktop only |
| **Language Focus** | Mixed ecosystem | Rust-first approach |
| **System Updates** | Traditional | Immutable/atomic |
| **Package Management** | Standard repositories | Direct download + overlay system |

---

## 2. System Requirements

### 2.1 Hardware Requirements

#### Minimum Specifications
- **Processor**: 64-bit x86 CPU (Intel/AMD)
- **Memory**: 4GB RAM
- **Storage**: 20GB available disk space (20GB ROOTS + 12GB HOME recommended for LUKS encrypted systems)
- **Firmware**: UEFI only (BIOS not supported)

#### Recommended Specifications
- **Processor**: Multi-core x86-64
- **Memory**: 8GB+ RAM
- **Storage**: 30GB+ SSD storage
- **Firmware**: UEFI (Secure Boot is not yet supported)

### 2.2 Supported Architectures

Currently supported:
- `x86_64` (AMD64)

---

## 3. Installation

### 3.1 Pre-Installation

RegicideOS is developed and tested primarily inside virtual machines. You do not need a Linux live environment unless you are installing to bare metal.

For developer builds:
- A Linux host with Docker/Podman and the Dagger CLI
- `/dev/kvm` access for VM image creation
- At least 100 GB free disk space

For bare-metal installs:
- Boot any Linux live environment (e.g., Fedora Live)
- Clone the repo, build the Rust installer, and point it at a local SquashFS image

> **⚠️ IMPORTANT**: Prevent system suspend during installation to avoid state corruption, especially when using LUKS encryption.

### 3.2 Installation Methods

#### 3.2.1 Build from Source with Dagger

The intended developer build for RegicideOS uses the **Dagger pipeline**. It runs the Gentoo stage4 build inside a container, so you do not need a Gentoo host or Catalyst installed locally.

**Prerequisites:**
- A Linux host with Docker or Podman
- The Dagger CLI (`dagger`)
- `git`
- At least 100 GB free disk space (COSMIC compiles many Rust packages from source)
- `/dev/kvm` access if you plan to build a QCOW2 VM image

**Clone the repository:**

```bash
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS
```

**Build the stage4 + live SquashFS:**

```bash
DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain
```

This runs six cacheable stages in `build-system/catalyst/stages/`. Use `--plain` (or set `DAGGER_PROGRESS=plain`) to stream plain text logs instead of the interactive TUI, which is easier to read in agent/CI environments:

1. `stage1-setup.sh` — download and extract the Gentoo stage3 seed
2. `stage2-sync.sh` — sync Portage and update `@world`
3. `stage3-base.sh` — install the base system packages
4. `stage4-cosmic.sh` — install COSMIC desktop packages
5. `stage5-regicide.sh` — install RegicideOS tools
6. `stage6-finalize.sh` — enable services, run dracut, create the stage4 tarball

The first run can take several hours because the COSMIC stage compiles Rust packages from source. Subsequent runs reuse the `distfiles` and `binpkgs` Dagger cache volumes and are much faster.

Outputs:
- `build-system/catalyst/output/stage4-amd64-systemd-cosmic.tar.xz`
- `build-system/catalyst/output/regicide-cosmic.img` (live SquashFS)

**Build a bootable QCOW2 VM image:**

`build-vm-image.sh` takes the stage4 tarball and SquashFS and creates a fully bootable QCOW2 disk image by running the installer inside a KVM VM. No host root access or loop devices are required.

```bash
cd build-system/catalyst

# Basic unencrypted image
./build-vm-image.sh \
    --squashfs output/regicide-cosmic.img \
    output/stage4-amd64-systemd-cosmic.tar.xz \
    output/regicide-cosmic.qcow2 20G

# Encrypted image with LUKS2
printf 'regicide-secure-test' > /tmp/regicide-passphrase.txt
./build-vm-image.sh \
    --encrypt --passphrase-file /tmp/regicide-passphrase.txt \
    --squashfs output/regicide-cosmic.img \
    output/stage4-amd64-systemd-cosmic.tar.xz \
    output/regicide-cosmic-enc.qcow2 20G
```

The VM builder exposes a SPICE display on port `5920` so you can watch the build:

```bash
remote-viewer spice://localhost:5920
```

**Boot the QCOW2 image in QEMU**

Serial console:

```bash
pkill -9 -f 'qemu-system-x86_64' 2>/dev/null || true
cp /usr/share/OVMF/OVMF_VARS.fd /tmp/ovmf-vars.fd
qemu-system-x86_64 \
    -enable-kvm -m 8G -smp 4 -cpu host -machine type=q35,accel=kvm \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
    -drive if=pflash,format=raw,file=/tmp/ovmf-vars.fd \
    -nographic \
    -hda output/regicide-cosmic.qcow2
```

For encrypted images, enter the LUKS passphrase `regicide-secure-test` when prompted.

Graphical SPICE window:

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

Then attach a SPICE viewer:

```bash
remote-viewer spice://localhost:5920
```

**Install to bare metal from the local SquashFS:**

Boot the target machine from any Linux live environment, clone the repo, build the installer, and run it against the SquashFS:

```bash
cd installer
cargo build --release

sudo ./target/release/installer \
    --image ../build-system/catalyst/output/regicide-cosmic.img \
    /dev/sdX
```

Or use a configuration file:

```toml
# regicide-local.toml
drive = "/dev/sda"
image_path = "build-system/catalyst/output/regicide-cosmic.img"
filesystem = "btrfs"
username = "regicide"
applications = ""
```

```bash
sudo ./target/release/installer -c regicide-local.toml
```

**Other deployment formats:**

- The live SquashFS (`regicide-cosmic.img`) can be written directly to a USB drive or deployed to a ROOTS partition.
- Bootable ISO / hybrid image generation is not yet automated; track progress in `STATUS.md`.

#### 3.2.3 Bare-Metal Automated Installation

For scripted deployments, create a configuration file and run the source-built installer:

```bash
# Create configuration
cat > regicide-config.toml << EOF
drive = "/dev/sda"
image_path = "/path/to/regicide-cosmic.img"
filesystem = "btrfs_encryption_dev"
username = "admin"
applications = "recommended"
EOF

# Run the source-built installer
sudo ./target/release/installer -c regicide-config.toml
```

### 3.3 Installation Process

The RegicideOS installer performs these steps:

1. **System Preparation**
   - Validate system dependencies (gdisk, cryptsetup, etc.)
   - Check network connectivity to RegicideOS repositories

2. **Drive Partitioning** (4-Partition Overlayfs Layout)
   - EFI System Partition (512MB, FAT32) with boot flag
   - ROOTS Partition (BTRFS, read-only base system template)
   - OVERLAY Partition (BTRFS, writable layers)
   - HOME Partition (LUKS-encrypted BTRFS, user data)

3. **Filesystem Setup**
   - Create BTRFS filesystem on ROOTS, OVERLAY, and HOME partitions
   - Setup LUKS encryption on HOME partition with cryptsetup
   - Set up overlay directory structure on OVERLAY partition

4. **System Image Deployment**
   - Extract SquashFS `root.img` to ROOTS partition
   - Uses locally-built or downloaded COSMIC desktop image
   - Verify image integrity with checksums

5. **Bootloader Installation**
   - Install GRUB for UEFI
   - Configure boot parameters for immutable system
   - **Configure LUKS initramfs support BEFORE GRUB installation**
   - Install GRUB with crypto modules for encrypted boot

6. **Post-Installation Configuration**
   - Set up overlay filesystem mounts (/etc, /var, /usr)
   - Configure LUKS initramfs scripts and crypttab
   - Generate GRUB configuration with dynamic UUID detection

7. **User Account Creation**
   - Default user `regicide` is baked into the stage4 image with password `regicide`
   - User is in `wheel` and can `sudo`
   - Root password is intentionally unset; set it after first login with `sudo passwd root`

8. **Application Installation**
   - Install Flatpak applications from Flathub
   - Setup application containers (Distrobox) for isolated workspaces

9. **System Configuration**
   - Install official dotfiles (regicide-dotfiles)
   - Configure system services
   - Setup networking

10. **Cleanup and Verification**
   - Verify all mounts and services
   - Generate installation report

> **See [INSTALLATION_ARCHITECTURE.md](INSTALLATION_ARCHITECTURE.md) for complete technical details on the 4-Partition Overlayfs architecture and LUKS boot implementation.

---

## 4. System Architecture

### 4.1 Current Implementation: 4-Partition Overlayfs Layout

RegicideOS uses a **4-Partition Overlayfs architecture** inherited from its upstream project. The base system is a Gentoo stage4 with COSMIC as the default desktop, built locally through the Catalyst/Dagger pipeline and deployed as a read-only SquashFS image.

```
/dev/sda1   512 MB  FAT32   label "EFI"
/dev/sda2   ~12-20GB  BTRFS    label "ROOTS"  (read-only base system image)
/dev/sda3   ~4-8GB   BTRFS    label "OVERLAY"  (writable overlay layers)
/dev/sda4   Remaining  LUKS-encrypted BTRFS label "HOME"  (user data)
```

**Overlay Structure:**
```
/                       # Merged view: read-only ROOTS lowerdir + writable OVERLAY upperdir
├── boot/efi            # EFI System Partition
├── root/               # Read-only lowerdir from ROOTS partition
└── overlay/            # Writable upperdir for /etc, /var, /usr
```

**Boot Process:**
1. **UEFI → GRUB → kernel**
2. **initrd** loads and mounts:
   - ROOTS partition containing the validated SquashFS `root.img`
   - Overlayfs layers for `/etc`, `/var`, `/usr` backed by the OVERLAY partition
   - `/home` partition (writable, LUKS-encrypted BTRFS)
3. **systemd** starts with the immutable root and writable overlays in place

Because the live image is a SquashFS produced directly by the Catalyst pipeline, the bootable artifact is byte-for-byte what was validated during the build.

### 4.2 Benefits of Current Architecture

- **Simplicity**: Proven overlayfs approach
- **Reliability**: Read-only base cannot be corrupted during normal operation
- **Instant Rollback**: Boot the previous validated `root.img` instead of debugging partial mutations
- **Atomic Updates**: Updates are prepared offline, verified, and swapped by replacing the entire `root.img`
- **LUKS Encryption**: Full LUKS encryption support with dynamic partition detection
- **Auditability**: The base OS is compiled from source on your own hardware with declarative inputs

### 4.3 Known Limitations

- **No Subvolume Management**: Overlays are flat directories, not BTRFS sub-volumes
- **Limited Rollback**: Only to previous system image, not granular
- **No Snapshots**: Cannot snapshot individual system states

### 4.4 Future Roadmap: BTRFS-Native Architecture (Planned for 2026-2027)

**Note**: BTRFS-Native architecture is planned for a future major version and will provide:

- Subvolume-based system layout (@etc, @var, @usr, @home)
- Instant snapshots via `btrfs subvolume snapshot`
- Granular rollback to specific system states
- Better storage efficiency with copy-on-write

> **See [INSTALLATION_ARCHITECTURE.md](INSTALLATION_ARCHITECTURE.md) for complete details on current architecture, LUKS boot implementation, and future roadmap.

---

## 5. Core Components

### 5.1 Cosmic Desktop Environment

RegicideOS exclusively ships with Cosmic Desktop:

#### Features:
- **Rust Implementation**: Built with Iced framework
- **Wayland Native**: Modern display protocol support
- **Tiling Window Manager**: Efficient workspace organization

#### Configuration:
```bash
# Cosmic settings are stored in
~/.config/cosmic/

# Example: Enable tiling by default
cosmic-settings set tiling.default true
```

### 5.2 System Components

#### 5.2.1 Init System
- **systemd**: Service and process management

#### 5.2.2 Container Runtime
- **Distrobox**: Application containerization for isolated workspaces
- **Podman Backend**: Secure, rootless containers

### 5.3 Network Management

RegicideOS uses NetworkManager:

```bash
# View network status
nmcli general status

# Connect to WiFi
nmcli dev wifi connect "SSID" password "password"
```

---

## 6. Package Management

### 6.1 Current Implementation: Portage-Based Immutable Image

The installer **does not use Foxmerge** for package management. Instead:

**Base System:**
- Built locally from a Gentoo stage4 with COSMIC desktop packages
- Deployed as a compressed `root.img` SquashFS on the ROOTS partition
- No package installation happens during bare-metal install
- System updates via atomic image replacement

**Overlay Packages:**
- Installed into overlay directories (`/etc`, `/var`, `/usr`)
- Managed with Portage (`emerge`) on the running system if needed
- GUI applications should prefer Flatpak for isolation from the base image

**Architecture Decision:**
- Immutable image model chosen for **simplicity, reliability, and auditability**
- Package management happens **post-installation** via Portage on the overlay or inside Distrobox containers
- Foxmerge was described in early planning but was not implemented

### 6.2 Package Installation Workflow

**During Installation:**
```bash
# No package installation - uses pre-built system image
```

**Post-Installation (user-initiated):**
```bash
# GUI applications (recommended)
flatpak install <app>

# System-level packages on the overlay (advanced)
sudo emerge <package>

# Project-specific toolchains inside containers
distrobox create --name dev --image fedora:44
distrobox enter dev
```

**System Updates:**
- Atomic: Build or download a new `regicide-cosmic.img`, replace `root.img` on ROOTS, reboot
- Incremental: Overlay packages updated via `emerge` on the running system

---

## 7. Post-Installation Updates

> **Current status**: RegicideOS does **not** have an automated update manager. There is no `foxmerge`, `foxbox`, or equivalent CLI yet. Post-install updates are manual.

### 7.1 Update Model

RegicideOS is an **image-based, immutable-root** distribution with a writable overlay layer:

- **ROOTS** partition: read-only base system image (`root.img` SquashFS)
- **OVERLAY** partition: writable `/etc`, `/var`, `/usr` overlays
- **HOME** partition: user data

This means there are two ways to update the system:

1. **Replace the base image** (atomic, clean, recommended)
2. **Use Portage on the overlay** (incremental, can drift from the base image)

### 7.2 Replacing the Base Image (Recommended)

When a new `regicide-cosmic.img` is available, replace the ROOTS image and reboot:

```bash
# Boot from a live environment or another root
sudo mkdir -p /mnt/roots
sudo mount /dev/disk/by-label/ROOTS /mnt/roots

# Back up the current image
sudo mv /mnt/roots/root.img /mnt/roots/root.img.previous

# Copy the new image
sudo cp /path/to/new/regicide-cosmic.img /mnt/roots/root.img
sync
sudo umount /mnt/roots
```

Then reboot. The overlay layer persists, so your `/etc`, `/var`, and `/usr` customizations remain — but compatibility with the new base image is your responsibility.

### 7.3 Using Portage on the Overlay (Advanced)

Because `/usr` is an overlay, you can run `emerge` directly on a running system, but this is **not the intended workflow** and can leave the system in an inconsistent state:

```bash
# Sync Portage (if network is configured)
sudo emerge --sync

# Update a single package
sudo emerge -av1 <package>
```

If you go this route, treat the system as a normal Gentoo install and accept that atomic rollback via image replacement may no longer apply cleanly.

### 7.4 User Toolchains

User-facing development tools are **not** installed in the base image by default. Install them per-user inside containers:

```bash
# Distrobox is the recommended container tool
distrobox create --name dev --image fedora:44
distrobox enter dev

# Inside the container (Fedora example)
sudo dnf install zed rustup nodejs
```

This keeps the base image small and lets each user pick their own toolchain.

### 7.5 Future: RegicideOS Update CLI

A first-party update tool is the next priority after the MVP build boots. The planned `regicide-update` CLI will:

- Accept a local SquashFS image produced by the Dagger pipeline
- Back up the current ROOTS image before replacing it
- Install the new image atomically
- Offer rollback to the previous image on failure

See `.omo/plans/root-update-system.md` for the full plan. For now, updates are manual using the steps in Section 7.2.

### 7.6 Customizing Your Base Image

Power users are expected to modify the Dagger pipeline and stage scripts to build their own base image:

```bash
# Edit stage scripts, e.g. add packages to stage4-cosmic.sh
$EDITOR build-system/catalyst/stages/stage4-cosmic.sh

# Rebuild with Dagger
DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain
```

The output `build-system/catalyst/output/regicide-cosmic.img` is the image you install or apply with the planned `regicide-update apply` command.

---

## 8. Development Environment

### 8.1 Development Environment Setup

User-facing development tools are installed inside **Distrobox** containers, not in the immutable base image. This keeps the base system minimal and lets each user choose their own toolchain.

#### 8.1.1 Container-First Toolchain

```bash
# Create a personal development container
distrobox create --name dev --image fedora:44
distrobox enter dev

# Inside the container, install whatever you need (Fedora example)
sudo dnf install zed rustup nodejs python3-pip
```

Applications launched from inside Distrobox integrate with the host desktop (icons, notifications, GPU access), so this feels native.

#### 8.1.2 Why Distrobox and not Toolbox?

- **Distrobox** works on Podman or Docker, supports more distributions, and has smoother GUI/Steam/GPU integration.
- **Toolbox** is simpler but more Fedora-centric.

RegicideOS uses **Distrobox** as the default. Toolbox may be supported later if users ask for it.

#### 8.1.3 IDE Setup (in a container)

**Zed Editor:**
```bash
# Install Zed inside your Distrobox container
flatpak install flathub dev.zed.Zed

# Or via your distribution's package manager
cargo install zed
```

#### 8.1.4 Development Tools

```bash
# Essential Rust tools
cargo install cargo-watch cargo-edit cargo-audit cargo-tarpaulin

# Code formatting and linting
rustup component add rustfmt clippy

# Cross-compilation support
rustup target add x86_64-unknown-linux-musl
```

### 8.2 AI/ML Development

#### 8.2.1 Machine Learning Frameworks

Some recommended Rust-native ML frameworks:

```bash
# PyTorch bindings for Rust
cargo add tch

# Candle - Rust-native ML framework
cargo add candle-core candle-nn

# Tokenizers and NLP
cargo add tokenizers hf-hub
```

#### 8.2.2 Development Containers

AI development environments in containers:

```bash
# Create ML development environment
distrobox create --name ml-dev --image fedora:44
distrobox enter ml-dev

# Install additional tools
pip install jupyter transformers datasets
```

### 8.3 User Configuration and Dotfiles

RegicideOS provides official dotfiles for a consistent, modern development experience:

#### 8.3.1 Installing RegicideOS Dotfiles

The official dotfiles package provides a Rust-focused shell configuration with RegicideOS theming:

```bash
# Add RegicideOS dotfiles overlay
sudo eselect repository enable regicide-dotfiles
sudo emaint sync -r regicide-dotfiles

# Install dotfiles package
sudo emerge -av app-misc/regicide-dotfiles

# Install dotfiles for your user
install-regicide-dotfiles

# For specific users (system administrators)
sudo install-regicide-dotfiles --user username
```

**Features included:**
- Modern Rust CLI tools (eza, bat, fd, ripgrep, zoxide)
- Enhanced bash with intelligent aliases and functions
- RegicideOS-themed tmux configuration
- Starship prompt with castling theming
- OpenRC/systemd service management
- BTRFS optimization tools
- Portage helper functions

#### 8.3.2 Dotfiles Customization

After installation, you can customize your environment:

```bash
# Edit shell configuration
nano ~/.bashrc

# Customize tmux
nano ~/.tmux.conf

# Configure prompt
nano ~/.config/starship.toml

# Set up Git configuration
nano ~/.gitconfig
```

#### 8.3.3 Backup and Restore

```bash
# Uninstall dotfiles (creates backup)
uninstall-regicide-dotfiles

# Restore from backup
uninstall-regicide-dotfiles --restore-backup
```

### 8.4 System Development

#### 8.4.1 Rust Toolchain Management

RegicideOS provides comprehensive Rust development support:

```bash
# Install Rust toolchain (if not present; use Portage on RegicideOS or the container's package manager)
sudo emerge -av rust cargo rustfmt clippy

# Install additional targets
rustc target add thumbv6m-none-eabi  # ARM Cortex-M
rustc target add riscv32imc-unknown-none-elf  # RISC-V
rustc target add wasm32-unknown-unknown  # WebAssembly
```

RegicideOS includes special support for embedded development:

```bash
# Install embedded tools
cargo install cargo-embed cargo-flash probe-run

# Create embedded project
cargo generate --git https://github.com/rust-embedded/cortex-m-quickstart

# Flash to hardware
cargo embed --target thumbv6m-none-eabi
```

### 8.5 Contributing to RegicideOS

Development environment for RegicideOS itself:

```bash
# Clone development repositories
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS

# Set up development environment
cargo build
cargo test

# Run installer in development mode
cargo run --bin installer -- --dry-run
```

---

## 9. System Administration

### 9.1 System Updates

RegicideOS uses an atomic update system via `root.img` tarball images:

#### 9.1.1 Base System Updates

> **Note**: RegicideOS does not currently host a remote update repository. Updates are performed by rebuilding the OS image locally with Catalyst and reinstalling, or by downloading updated release images from GitHub.
>
> ```bash
> # Manual update process (using locally-built image)
> sudo mount -L ROOTS /mnt/roots
> sudo tar xf /path/to/new-regicide-image.tar -C /mnt/roots
> sudo umount /mnt/roots
> sudo reboot
> ```

#### 9.1.2 Overlay Updates

Package overlays are updated using Portage on the writable overlay layer:

```bash
# Sync Portage and update world on the overlay
sudo emerge --sync
sudo emerge -avuDN @world

# Update a specific package
sudo emerge -av1 <package-name>

# Check for available updates
sudo emerge -avuDN @world --pretend

# Clean up old packages
sudo emerge --depclean
```

#### 9.1.3 Update Workflow

The installer automates base system updates:
1. Download newest `root.img` from RegicideOS repositories
2. Copy to ROOTS partition
3. Touch to ensure newest timestamp
4. Sync filesystem
5. Reboot

GRUB's `10_linux` helper automatically picks the newest `root-*.img` by modification time.

### 9.2 System Configuration

System configuration persists in overlay filesystems:

```bash
# Edit configuration (automatically goes to overlay)
sudo nano /etc/systemd/system.conf

# View overlay changes
sudo ls -la /overlay/etc

# View package status
sudo qlist -I  # Portage installed packages
```

### 9.3 System Snapshots

The current 4-Partition architecture does not support granular snapshots. For rollback:

```bash
# Boot from live environment
sudo mount -L ROOTS /mnt/regicide
sudo mount -L EFI /mnt/regicide/boot/efi

# Reinstall bootloader
sudo chroot /mnt/regicide
grub-install --target=x86_64-efi --efi-directory=/boot/efi
grub-mkconfig -o /boot/grub/grub.cfg

# Reboot
```

> **Note**: BTRFS-native architecture with subvolume snapshot support is planned for future versions (see [INSTALLATION_ARCHITECTURE.md](INSTALLATION_ARCHITECTURE.md)).

### 9.4 OS Personality Swapping

The phrase "OS personality" refers to ability to replace the read-only base system image (`root.img`) while keeping your local changes and home directory intact. This is one of the most powerful features of RegicideOS's architecture.

#### 9.4.1 Understanding OS Personalities

An "OS personality" is essentially the complete system environment contained in a compressed tarball (`root.img`). This includes:

- **Base system**: `/usr`, `/bin`, `/lib`, `/sbin`, and other core directories
- **Desktop environment**: Cosmic Desktop, GNOME, or other environments
- **System configuration**: Default settings and skeletons
- **Package set**: Pre-installed applications and libraries

Your personal data remains in separate locations:
- **Overlay**: Local modifications to `/etc`, `/var`, and `/usr`
- **Home directory**: User files in `/home`
- **Persistent configuration**: Custom settings and installed packages

#### 9.4.2 The Personality Swap Mechanism

Because RegicideOS's boot loader (GRUB) always picks the **newest** `root-*.img` by modification time, swapping personalities is as simple as copying a new image file to ROOTS partition.

#### 9.4.3 Step-by-Step Personality Swap

**While running RegicideOS or from a live USB:**

1. **Mount ROOTS partition**
   ```bash
   sudo mount /dev/disk/by-label/ROOTS /mnt
   ```

2. **Drop new image in place**
   ```bash
   sudo cp root-cosmic.img /mnt/
   sudo touch /mnt/root-cosmic.img   # ensure it has newest timestamp
   sync
   ```

3. **Reboot**
   ```bash
   sudo reboot
   ```

On the next boot:
- The initrd mounts `root-cosmic.img` as the read-only lowerdir
- The system overlays your writable BTRFS layers
- You are instantly running the new personality

#### 9.4.4 Rolling Back Personalities

Keep old file for easy rollback:

```bash
# Backup current personality
sudo mv /mnt/root-gnome.img /mnt/root-gnome.img.bak
```

If you want to switch back to GNOME:

```bash
sudo touch /mnt/root-gnome.img.bak   # make it newest again
reboot
```

#### 9.4.5 Managing Multiple Personalities

You can keep multiple personalities available:

```bash
# List available personalities
ls -la /mnt/root-*.img

# Switch to a specific personality
sudo touch /mnt/root-desired-personality.img
reboot
```

#### 9.4.6 Deleting Obsolete Images

Clean up old personalities to save space:

```bash
# Remove old personality
sudo rm /mnt/root-old.img

# Free space immediately (BTRFS-specific)
sudo btrfs filesystem sync /mnt
```

### 9.5 Managing Multiple Personalities

You can create your own personalities:

```bash
# Create a custom personality from current system
# Create a custom overlay template from current system
sudo tar czf /mnt/root-custom.img -C /tmp/rootfs .

# Ensure it's newest for next boot
sudo touch /mnt/root-custom.img
```

---

## 10. Troubleshooting

### 10.1 Installation Issues

#### 10.1.1 Common Installation Problems

**LUKS Device Not Found:**
```bash
# Check if LUKS partition is detected
lsblk -f NAME,FSTYPE,UUID | grep crypto

# Verify mapper device
sudo ls -la /dev/mapper/

# Manual LUKS setup workaround
sudo cryptsetup luksFormat /dev/sdX4  # Replace with your partition
sudo cryptsetup open /dev/sdX4 ROOTS_HOME
sudo mkfs.btrfs -L HOME /dev/mapper/ROOTS_HOME
sudo cryptsetup close ROOTS_HOME
```

**Boot Issues After Installation:**
```bash
# Check boot partition mount
mount | grep efi

# Verify GRUB files exist
ls -la /boot/efi/EFI/

# Check GRUB configuration
cat /boot/efi/grub/grub.cfg
```

#### 10.1.2 LUKS-Specific Issues

> **Note**: As of v2.0 (January 2026), the installer includes comprehensive LUKS boot improvements including dynamic partition detection and proper UUID handling. The issues below describe legacy problems that should no longer occur.

**No Password Prompt at Boot (Legacy - Should Not Occur in v2.0+):**

**Legacy Causes (Fixed in v2.0):**
1. ~~GRUB installed before initramfs configured~~ → Now configures initramfs BEFORE GRUB installation
2. ~~Hardcoded `/dev/sda3` partition reference~~ → Now uses dynamic `find_luks_partition()` detection
3. Missing crypttab entry
4. Initramfs lacks encrypt hooks

**Current Troubleshooting (if issues persist):**
```bash
# Check LUKS partition is detected (v2.0+ uses dynamic detection)
sudo blkid -o device -t TYPE=crypto_LUKS

# Verify initramfs has LUKS support
lsinitramfs /boot/initrd.img-* | grep cryptsetup

# Check crypttab
cat /etc/crypttab

# Reinstall GRUB with proper modules (v2.0+ does this automatically)
sudo grub-install --modules="cryptodisk luks gcry_rijndael gcry_sha256 gcry_sha1 aesni part_gpt lvm" --target=x86_64-efi --efi-directory=/boot/efi

# Regenerate initramfs
sudo update-initramfs -u -k all
```

**LUKS Device Not Found (v2.0+ Enhancement):**

The installer v2.0 now uses `find_luks_partition()` to dynamically detect LUKS partitions across multiple schemes:
- `/dev/sda3`, `/dev/sdb3` (standard SATA/SCSI)
- `/dev/nvme0n1p3`, `/dev/nvme1n1p3` (NVMe drives)
- `/dev/mmcblk0p3` (eMMC storage)

If detection fails:
```bash
# Manually verify LUKS partition
sudo blkid -o device -t TYPE=crypto_LUKS

# Check all block devices
sudo lsblk -f | grep crypto
```

### 10.2 Boot Issues

#### 10.2.1 GRUB Recovery

If system fails to boot:

1. **Boot from live environment**
   ```bash
   sudo mount -L ROOTS /mnt/regicide
   sudo mount -L EFI /mnt/regicide/boot/efi
   ```

2. **Reinstall bootloader:**
   ```bash
   sudo chroot /mnt/regicide
   grub-install --target=x86_64-efi --efi-directory=/boot/efi
   grub-mkconfig -o /boot/grub/grub.cfg
   ```

3. **Try different desktop environments:**
   - Cosmic Desktop (default)
   - GNOME (via different image)
   - Wayland-only mode

#### 10.2.2 Snapshot Recovery

Since current architecture doesn't support granular snapshots, rollback requires personality swapping (see Section 9.4).

```bash
# List available personalities (system images)
ls -la /mnt/root-*.img

# Boot from live USB and mount
sudo mount -L ROOTS /mnt/regicide

# Copy previous working image
sudo cp /mnt/root-working.img /mnt/

# Reboot to use previous personality
sudo reboot
```

### 10.3 Service Management

#### 10.3.1 System Services

```bash
# Manage core services
sudo systemctl status networking
sudo systemctl restart bluetooth

# View service logs
sudo journalctl -u systemd-networkd
```

#### 10.3.2 AI Agent Management

```bash
# Control BtrMind storage agent
sudo systemctl enable btrmind
sudo systemctl start btrmind

# Monitor agent performance
sudo journalctl -u btrmind -f
btrmind stats
```

### 10.4 System Health

```bash
# Check filesystem status
sudo btrfs filesystem df /

# Balance filesystem
sudo btrfs balance start /

# Scrub for errors
sudo btrfs scrub start /
```

### 10.5 Debug Information

**Enable Verbose Boot:**
```bash
# Add kernel parameter for detailed boot output
# In GRUB: append "verbose" to kernel params
# Or edit /etc/default/grub: GRUB_CMDLINE_LINUX="verbose"
```

**Enable Installer Debugging:**
```bash
# Run installer with debug output
RUST_LOG=debug RUST_BACKTRACE=1 sudo ./installer

# Enable dry-run mode
./installer --dry-run
```

---

## 11. FAQ

### Q1: Does RegicideOS use BTRFS subvolumes?

**A:** No, the current implementation uses flat overlay directories (`/etc`, `/var`, `/usr`) instead of BTRFS sub-volumes. BTRFS-native architecture with subvolume support is planned for a future major version (2026-2027). See [INSTALLATION_ARCHITECTURE.md](INSTALLATION_ARCHITECTURE.md) for details.

### Q2: Does RegicideOS support multiple desktop environments?

**A:** Currently only Cosmic Desktop is supported. GNOME and other Wayland compositors are planned for future releases.

### Q3: How do I enable LUKS encryption?

**A:** Use the `btrfs_encryption_dev` filesystem layout during installation:
```bash
sudo ./installer
# Select: btrfs_encryption_dev
# Enter LUKS password when prompted
```

### Q4: How do I rollback to a previous system version?

**A:** The current 4-Partition architecture supports personality swapping:
1. Download previous system image from RegicideOS repositories
2. Copy to ROOTS partition: `sudo cp root.img /mnt/ROOTS/`
3. Reboot and select older image in GRUB

> **Note**: BTRFS-native architecture with subvolume snapshots will provide granular rollback when implemented.

### Q5: Where are user settings stored?

**A:** User settings are stored in overlay filesystem:
- `/etc/config/regicide/` - System-wide settings
- `/home/$USER/.config/regicide/` - User-specific settings
- `/etc/hosts`, `/etc/resolv.conf` - Network configuration

### Q6: What happened to Foxmerge?

**A:** Foxmerge was described in early planning but was not implemented. RegicideOS does not currently have a first-party update CLI. Post-installation base-system updates are done by replacing the `root.img` on the ROOTS partition, and per-user packages are installed in Distrobox containers. Standard Portage commands can be used on the overlay layer, but that is not the recommended workflow.

### Q7: How do post-install updates work?

**A:** There is no automated update manager yet. The recommended manual workflow is:

1. Build or download a new `regicide-cosmic.img`.
2. Mount the ROOTS partition from a live environment.
3. Replace `/mnt/roots/root.img` with the new image.
4. Reboot.

See Section 7 for the full update procedure.

### Q8: How do I verify LUKS is working correctly?

**A:**
```bash
# Check partition type
sudo blkid /dev/sda3
# Should show: TYPE="crypto_LUKS"

# Check mapper device
sudo ls -la /dev/mapper/
# Should show: regicideos -> ../sda3

# Check crypttab
cat /etc/crypttab
# Should show: regicideos UUID=<uuid> none luks

# Test initramfs
sudo update-initramfs -u -k all
lsinitramfs /boot/initrd.img-* | grep cryptsetup
```

---

## 12. References

### Related Documentation
- [INSTALLATION_ARCHITECTURE.md](INSTALLATION_ARCHITECTURE.md) - Complete technical architecture details, LUKS boot implementation
- [HANDBOOK_ISSUES.md](HANDBOOK_ISSUES.md) - Discrepancies analysis between documentation and implementation
- [README.md](README.md) - Project overview and quick start
- [DEVELOPMENT_ROADMAP.md](DEVELOPMENT_ROADMAP.md) - Long-term technical roadmap
- [AGENTS.md](AGENTS.md) - AI agent development guidelines
- [iso-config.toml](iso-config.toml) - ISO build configuration

### External Documentation
- [Gentoo Linux Handbook](https://wiki.gentoo.org/wiki/Handbook:AMD64)
- [GRUB Documentation](https://www.gnu.org/software/grub/manual/)
- [BTRFS Documentation](https://btrfs.wiki.kernel.org/)
- [LUKS/cryptsetup Documentation](https://gitlab.com/cryptsetup/cryptsetup/-/wikis/home)
- [Cosmic Desktop Documentation](https://github.com/pop-os/cosmic-desktop)

### Code Repository
- [Main Repository](https://github.com/awdemos/RegicideOS)
- [Installer](/installer/)
- [AI Agents](/ai-agents/)

---

## Appendix: Technical Details

### A.1 LUKS Boot Configuration

**GRUB Boot Entry (Encrypted):**
```
menuentry "RegicideOS (Encrypted)" {
    linux /boot/vmlinuz-*
    initrd /boot/initrd.img-*
    options "cryptdevice=UUID=<detected-uuid>:regicideos root=/dev/mapper/regicideos quiet splash rw"
}
```

**Kernel Parameters:**
- `cryptdevice=UUID=<uuid>:regicideos` - Tell GRUB which device to open
- `root=/dev/mapper/regicideos` - Root filesystem after LUKS decryption
- `quiet splash rw` - Boot options

**Initramfs Components:**
- `cryptsetup` - LUKS management utility
- `encrypt` hook - Handles LUKS decryption during boot
- `crypttab` - Persistent LUKS device mapping

### A.2 Partition Detection Logic

**Detection Algorithm:**
1. Try `blkid -o device -t TYPE=crypto_LUKS` (most reliable)
2. Fall back to device enumeration (sda3, sdb3, nvme0n1p3, nvme1n1p3)
3. Extract UUID from detected partition via `blkid -s UUID -o value`
4. Use device name as ultimate fallback if all methods fail

**Supported Partition Schemes:**
- `/dev/sda3`, `/dev/sdb3` (standard SATA/SCSI)
- `/dev/nvme0n1p3`, `/dev/nvme1n1p3` (NVMe drives)
- `/dev/mmcblk0p3` (eMMC storage)

---

## Changelog

### Version 2.0 (January 2026)

**Added:**
- Dynamic LUKS partition detection via `find_luks_partition()` helper function
- LUKS UUID extraction for boot configuration
- Comprehensive LUKS initramfs configuration
- GRUB cryptodisk module installation for encrypted boot support
- Improved error handling throughout installer

**Changed:**
- Updated 3 hardcoded `/dev/sda3` references to use dynamic partition detection
- LUKS initramfs now configured BEFORE GRUB installation
- Complete rewrite of architecture documentation to accurately reflect 4-Partition Overlayfs implementation
- Removed Foxmerge references (not implemented)

**Removed:**
- `verify_grub_environment()` function (~200 lines of redundant debugging)
- `create_grub_configuration()` function (~182 lines of duplicate logic)

**Fixed:**
- LUKS boot failures due to hardcoded partition references
- Missing password prompt at boot time
- Incorrect UUID usage in GRUB boot parameters
- Documentation inaccuracies about architecture and package management

### Version 1.0 (November 2024)

**Initial release**
- 4-Partition overlayfs architecture
- Cosmic Desktop integration
- Gentoo stage3 base system with COSMIC desktop
- Basic LUKS encryption support
- Basic installer functionality

---

*Document Version: 2.1*
*Last Updated: April 2026*
