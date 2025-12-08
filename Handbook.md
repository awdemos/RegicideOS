# RegicideOS Handbook

<div align="center">

![RegicideOS Logo](regicideos_poster.png)

**The Complete Installation and Administration Guide**

*Version 1.0 - Draft*

</div>

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

RegicideOS is a specialized fork of Xenia Linux focused on:

- **Rust-First Architecture**: System components migrated to Rust for memory safety and performance
- **Immutable System Architecture**: Read-only root filesystem for enhanced security and reliability

### 1.2 Key Differentiators from Xenia Linux

| Feature | Xenia Linux | RegicideOS |
|---------|-------------|------------|
| **Desktop Environment** | Multiple choices | Cosmic Desktop only |
| **Language Focus** | Mixed ecosystem | Rust-first approach |
| **System Updates** | Traditional | Immutable/atomic |
| **Package Management** | Standard repositories | Overlay-based |

---

## 2. System Requirements

### 2.1 Hardware Requirements

#### Minimum Specifications
- **Processor**: 64-bit x86 CPU (Intel/AMD)
- **Memory**: 4GB RAM
- **Storage**: 12GB available disk space
- **Firmware**: UEFI or Legacy BIOS

#### Recommended Specifications
- **Processor**: Multi-core x86-64
- **Memory**: 8GB+ RAM
- **Storage**: 20GB+ SSD storage
- **Firmware**: UEFI with Secure Boot support

### 2.2 Supported Architectures

Currently supported:
- `x86_64` (AMD64)

---

## 3. Installation

### 3.1 Pre-Installation

#### 3.1.1 Live Environment Setup

RegicideOS requires installation from a Linux live environment. We recommend Fedora Live:

1. **Download Fedora Live Workstation**: https://getfedora.org/workstation/download/
2. **Create bootable USB**: Use tools like `dd`, Rufus, or balenaEtcher
3. **Boot target machine**: From the live USB environment

#### 3.1.2 Prepare Live Environment

Once booted into the live environment:

```bash
# Install dependencies (including gdisk for EFI support)
sudo dnf install -y git curl gcc sgdisk rust cargo

# Clone RegicideOS repository
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS/installer

# Prevent system suspend during installation (critical for LUKS setups)
sudo systemctl mask sleep.target suspend.target hibernate.target hybrid-sleep.target
sudo loginctl disable-lid-switch

# Build RegicideOS repository
cargo build --release
sudo ./target/release/installer
```

> **⚠️ IMPORTANT**: Prevent system suspend during installation to avoid state corruption, especially when using LUKS encryption. The installer now handles this automatically, but manual prevention is recommended for reliability.

### 3.2 Installation Methods

#### 3.2.1 Using Pre-built Installer (Recommended)

The easiest installation method is to use the pre-built installer binary:

```bash
# Download and run the pre-built installer
sudo ./binaries/regicide-installer

# Or run with configuration file
sudo ./binaries/regicide-installer -c regicide-config.toml
```

The installer will guide you through:
1. **Drive Selection**: Choose target installation drive
2. **Filesystem Layout**: BTRFS with LUKS encryption (recommended)
3. **User Setup**: Create administrative user account
4. **Application Sets**: Choose minimal or recommended packages

#### 3.2.2 Building from Source

If you prefer to build the installer from source or need to customize it:

```bash
# Build the installer from source (now without warnings)
cargo build --release

# Verify build completed successfully
./target/release/installer --version

# Run interactive installation
sudo ./target/release/installer

# Or run with configuration file
sudo ./target/release/installer -c regicide-config.toml

# For development/testing with dry run
cargo run --bin installer -- --dry-run
```

#### 3.2.3 Automated Installation

For scripted deployments, create a configuration file:

```bash
# Create configuration
cat > regicide-config.toml << EOF
drive = "/dev/sda"
repository = "https://repo.xenialinux.com/releases/"
flavour = "cosmic-fedora"
release_branch = "main"
filesystem = "btrfs_encryption_dev"
username = "admin"
applications = "recommended"
EOF

# Run with pre-built installer (recommended)
sudo ./binaries/regicide-installer -c regicide-config.toml

# Or run with source-built installer
sudo ./target/release/installer -c regicide-config.toml
```

### 3.3 Manual Installation (Legacy)

> **⚠️ DEPRECATED**: The 4-partition overlayfs layout is deprecated. Use BTRFS-native architecture instead.

For reference, the legacy system used:
- EFI System Partition (512MB, FAT32)
- ROOTS partition (holds system images)
- OVERLAY partition (writable layers) 
- HOME partition (user data)

This has been replaced by the BTRFS-native approach described in Section 4.1.

### 3.5 Post-Installation

After successful installation:

1. **Remove installation media** and reboot
2. **Complete initial setup** through Cosmic Desktop
3. **Verify system integrity**:
   ```bash
   # Check all mounts are working
   mount | grep -E "(overlay|btrfs)"
   
   # Verify AI services status
   systemctl status portcl btrmind
   
   # Check for installation errors
   sudo journalctl -u installer.service --since "5 minutes ago"
   ```

### 3.3 Installation Process

The RegicideOS installer performs these steps:

1. **System Preparation**
   - Validate system dependencies (gdisk, cryptsetup, etc.)
   - Check network connectivity to Xenia repositories

2. **Drive Partitioning**
   - EFI System Partition (512MB, FAT32) with boot flag
   - Root Partition with LUKS-encrypted BTRFS (remaining space)

3. **Filesystem Setup**
   - Create BTRFS subvolumes for overlay system
   - Configure read-only root filesystem via SquashFS
   - Set up writable overlays for `/etc`, `/var`, `/usr`

4. **System Image Download**
   - Download compressed system image from Xenia repositories
   - Uses `cosmic-fedora` flavor

5. **Bootloader Installation**
   - Install GRUB for EFI or Legacy BIOS
   - Configure boot parameters for immutable system

6. **Post-Installation Cleanup**
   - Verify all mounts and services
   - Generate installation report

### 3.4 Post-Installation

After successful installation:

1. **Remove installation media** and reboot
2. **Complete initial setup** through Cosmic Desktop
3. **Verify system integrity**:
   ```bash
   # Check all mounts are working
   mount | grep -E "(overlay|btrfs)"
   
   # Check for installation errors
   sudo journalctl -u installer.service --since "5 minutes ago"
   ```

---

## 4. System Architecture

### 4.1 BTRFS-Native Architecture

RegicideOS uses BTRFS-native snapshots and sub-volumes to provide an immutable base system with writable overlays.

#### 4.1.1 Disk Layout

```
/dev/sda1   512 MB  FAT32   label "EFI"
/dev/sda2   rest    BTRFS   label "ROOTS"
```

BTRFS sub-volumes:
- `@etc` - writable layer for /etc
- `@var` - writable layer for /var  
- `@usr` - writable layer for /usr
- `@home` - /home tree

#### 4.1.2 Boot Sequence

1. **UEFI → GRUB → kernel + initrd** (loaded from SquashFS)
2. **BTRFS sub-volumes bind-mounted** on top of SquashFS directories
3. **Switch-root** → systemd starts

#### 4.1.3 Key Benefits

- **Single partition** - no separate OVERLAY partition needed
- **Instant snapshots** - `btrfs subvolume snapshot -r @etc @etc-backup`
- **Easy rollback** - restore snapshots without rebooting
- **Storage efficiency** - immediate space reclamation

### 4.2 BTRFS Commands

#### Subvolume Management
```bash
# List subvolumes
btrfs subvolume list /

# Create snapshot
btrfs subvolume snapshot -r /@etc /@etc-backup

# Delete subvolume
btrfs subvolume delete /@old-subvol
```

#### System Maintenance
```bash
# Check filesystem status
btrfs filesystem df /

# Balance filesystem
btrfs balance start /

# Scrub for errors
btrfs scrub start /
```

### 4.3 Security Model

- **Read-Only Root**: Base system cannot be modified during runtime
- **Verified Boot**: System image integrity verified at boot
- **Namespace Isolation**: Containers provide application isolation

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
- **Distrobox**: Application containerization
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

### 6.1 Overlay System

RegicideOS uses a hybrid package management approach:

1. **Base System**: Immutable system image from Xenia repositories
2. **GURU Overlay**: Community-maintained packages
3. **Regicide Overlay**: Custom modifications

### 6.2 Foxmerge Package Management

Foxmerge is Xenia Linux's primary package management tool:

#### Basic Operations
```bash
# Update all overlay packages
sudo foxmerge update

# Install a package
sudo foxmerge install package-name

# Remove a package
sudo foxmerge remove package-name

# Search for packages
sudo foxmerge search search-term
```

#### Overlay Management
```bash
# List available overlays
sudo foxmerge overlay list

# Enable an overlay
sudo foxmerge overlay enable overlay-name

# Sync overlay repositories
sudo foxmerge overlay sync
```

### 6.3 User Configuration and Dotfiles

RegicideOS provides official dotfiles:

```bash
# Install dotfiles package
sudo emerge -av app-misc/regicide-dotfiles

# Install dotfiles for your user
install-regicide-dotfiles
```

**Features included:**
- Modern Rust CLI tools (eza, bat, fd, ripgrep)
- Enhanced bash with intelligent aliases
- RegicideOS-themed tmux configuration
- Starship prompt with castle theming

### 6.4 Rust Development Environment

```bash
# Install Rust toolchain
sudo dnf install -y rust cargo rustfmt clippy

# Install additional targets
rustc target add thumbv6m-none-eabi  # ARM Cortex-M
rustc target add wasm32-unknown-unknown  # WebAssembly
```

### 6.5 Container-Based Applications

#### Distrobox Integration
```bash
# Create development environment
distrobox create --name dev --image fedora:39
distrobox enter dev

# Install applications in container
sudo dnf install -y code brave-browser
```

#### Flatpak Integration
```bash
# Install applications
flatpak install flathub com.brave.Browser
flatpak install flathub com.visualstudio.code
```

---

## 7. Package Management

### 7.1 Overlay System

RegicideOS uses a hybrid package management approach combining:

1. **Base System**: Immutable system image from Xenia repositories
2. **GURU Overlay**: Community-maintained packages
3. **Regicide Overlay**: Custom modifications and AI-enhanced packages

### 7.2 Foxmerge Package Management

Foxmerge is Xenia Linux's primary package management tool, inherited by RegicideOS for managing overlay packages and system updates. It provides a simplified interface for Gentoo's Portage system while maintaining compatibility with the underlying Gentoo ecosystem.

#### 7.2.1 Basic Foxmerge Operations

**System Updates:**
```bash
# Update all overlay packages
sudo foxmerge update

# Update specific package
sudo foxmerge update package-name

# Check for available updates
sudo foxmerge check-updates

# Clean up old packages
sudo foxmerge clean
```

**Package Installation:**
```bash
# Install a package
sudo foxmerge install package-name

# Install multiple packages
sudo foxmerge install package1 package2 package3

# Install with specific USE flags
sudo foxmerge install package-name --use "flag1 flag2"

# Install from specific overlay
sudo foxmerge install package-name --overlay overlay-name
```

**Package Management:**
```bash
# Remove a package
sudo foxmerge remove package-name

# Search for packages
sudo foxmerge search search-term

# Get package information
sudo foxmerge info package-name

# List installed packages
sudo foxmerge list

# List available packages
sudo foxmerge list --available
```

#### 7.2.2 Advanced Foxmerge Usage

**Overlay Management:**
```bash
# List available overlays
sudo foxmerge overlay list

# Enable an overlay
sudo foxmerge overlay enable overlay-name

# Disable an overlay
sudo foxmerge overlay disable overlay-name

# Sync overlay repositories
sudo foxmerge overlay sync

# Add custom overlay
sudo foxmerge overlay add https://github.com/user/overlay.git
```

**Configuration Management:**
```bash
# View current configuration
sudo foxmerge config show

# Edit configuration
sudo foxmerge config edit

# Set configuration option
sudo foxmerge config set option value

# Reset configuration to defaults
sudo foxmerge config reset
```

**Dependency Resolution:**
```bash
# Resolve dependencies for a package
sudo foxmerge deps package-name

# Show reverse dependencies
sudo foxmerge deps --reverse package-name

# Check for conflicts
sudo foxmerge check-conflicts package-name

# Fix broken dependencies
sudo foxmerge fix-deps
```

#### 7.2.3 System Maintenance with Foxmerge

**Regular Maintenance:**
```bash
# Full system update and cleanup
sudo foxmerge update && sudo foxmerge clean

# Update overlays and sync repositories
sudo foxmerge overlay sync && sudo foxmerge update

# Check system health
sudo foxmerge health-check

# Optimize package database
sudo foxmerge optimize
```

**Troubleshooting:**
```bash
# Check for broken packages
sudo foxmerge check-broken

# Repair broken packages
sudo foxmerge repair

# Clear package cache
sudo foxmerge cache-clear

# Rebuild package database
sudo foxmerge rebuild-db
```

#### 7.2.4 Foxmerge Configuration

**Main Configuration File (`/etc/foxmerge.conf`):**
```bash
# Overlay repositories
overlays = ["guru", "regicide", "science"]

# Update settings
auto_sync = true
auto_clean = true
parallel_jobs = 4

# Build options
makeopts = "-j4"
use_flags = ["X", "alsa", "pulseaudio"]

# Logging
log_level = "info"
log_file = "/var/log/foxmerge.log"
```

**User Configuration (`~/.config/foxmerge/config`):**
```bash
# User-specific settings
[settings]
  default_overlay = "regicide"
  color_output = true
  verbose_output = false

[aliases]
  up = "update"
  in = "install"
  rm = "remove"
  se = "search"
```

#### 7.2.5 Integration with System Updates

Foxmerge works seamlessly with Xenia Linux's update system:

```bash
# Automated system update process
sudo xenia-update  # Updates base system image
sudo foxmerge update  # Updates overlay packages

# Combined update script
sudo system-update-full
```

This two-tier approach ensures:
- **Base System**: Updated atomically via SquashFS images
- **Overlay Packages**: Updated incrementally via foxmerge
- **AI Optimization**: PortCL optimizes package selection and updates

### 7.3 User Configuration and Dotfiles

RegicideOS provides official dotfiles for a consistent, modern development experience:

#### 7.3.1 Installing RegicideOS Dotfiles

The official dotfiles package provides a Rust-focused shell configuration with RegicideOS theming:

```bash
# Add the dotfiles overlay
sudo eselect repository enable regicide-dotfiles
sudo emaint sync -r regicide-dotfiles

# Install the dotfiles package
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
- Starship prompt with castle theming
- OpenRC/systemd service management
- BTRFS optimization tools
- Portage helper functions

#### 7.3.2 Dotfiles Customization

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

**Backup and Restore:**
```bash
# Uninstall dotfiles (creates backup)
uninstall-regicide-dotfiles

# Restore from backup
uninstall-regicide-dotfiles --restore-backup
```

#### 7.3.3 System-wide Configuration

For system administrators deploying dotfiles to multiple users:

```bash
# Install to skeleton directory for new users
sudo install-regicide-dotfiles --system-only

# New users will automatically get dotfiles
sudo useradd -m newuser
```

**Portage Configuration Template:**
```bash
# Copy system optimization template
sudo cp /usr/share/regicide-dotfiles/contrib/make.conf.template /etc/portage/make.conf

# Edit for your hardware
sudo nano /etc/portage/make.conf
```

### 7.4 Rust Development Environment

#### 7.4.1 Rust Toolchain Management

RegicideOS provides comprehensive Rust development support:

```bash
# Install Rust toolchain (if not present)
sudo dnf install -y rust cargo rustfmt clippy

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

### 7.5 Container-Based Applications

#### 7.5.1 Distrobox Integration

Most user applications run in containers for isolation:

```bash
# Create development environment
distrobox create --name dev --image fedora:39
distrobox enter dev

# Install applications in container
sudo dnf install -y code brave-browser thunderbird

# Applications appear in desktop menu automatically
```

#### 7.5.2 Flatpak Integration

System-wide applications use Flatpak:

```bash
# Install applications
flatpak install flathub com.brave.Browser
flatpak install flathub com.visualstudio.code

# Manage applications
flatpak list
flatpak update
```

---

## 8. Development Environment

### 8.1 Development Environment Setup

RegicideOS is optimized for modern development with comprehensive tooling. After installing the official dotfiles (see Section 7.2), you have a complete development environment:

#### 8.1.1 IDE Setup

**Zed Editor:**
```bash
# Install Zed via Flatpak
flatpak install flathub dev.zed.Zed

# Recommended extensions
zed --install-extension rust-analyzer
zed --install-extension lldb
zed --install-extension crates
```

#### 8.1.2 Development Tools

```bash
# Essential Rust tools
cargo install cargo-watch cargo-edit cargo-audit cargo-tarpaulin

# Code formatting and linting
cargo install rustfmt clippy

# Documentation generation
cargo install cargo-doc

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
distrobox create --name ml-dev --image pytorch/pytorch:latest
distrobox enter ml-dev

# Install additional tools
pip install jupyter transformers datasets
```

### 8.3 System Development

#### 8.3.1 Contributing to RegicideOS

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

#### 8.3.2 AI Agent Development

Creating custom AI agents:

```bash
# Agent template
cargo new --bin my-agent
cd my-agent

# Add AI dependencies
cargo add tch serde tokio anyhow

# Implement agent interface
# See examples in /usr/share/regicide/examples/
```

---

## 9. System Administration

### 9.1 System Updates

#### 9.1.1 Base System Updates

RegicideOS inherits Xenia Linux's atomic update system using SquashFS images:

```bash
# First-time setup (one-time)
sudo emerge --sync xenia

# Check for available updates
curl -s https://repo.xenialinux.com/releases/Manifest.toml

# Automated update process
sudo xenia-update

# Manual update process
sudo mount -L ROOTS /mnt/roots
cd /mnt/roots
sudo wget https://repo.xenialinux.com/releases/amd64/main/root.img
sudo umount /mnt/roots
sudo reboot
```

The system automatically boots from the newest root image. For rollback, you can select previous images in GRUB.

#### 9.1.2 Overlay Updates

Package overlays are updated separately using Xenia Linux's package management:

```bash
# Add Xenia overlay (first-time setup)
sudo emerge --sync xenia

# Update overlay packages using foxmerge
# See Section 7.2 for comprehensive foxmerge usage
sudo foxmerge update

# Traditional Portage updates
sudo emerge --sync
sudo emerge --update --deep @world
```

### 9.2 System Configuration

#### 9.2.1 Persistent Configuration

System configuration persists in overlays:

```bash
# Edit configuration (automatically goes to overlay)
sudo nano /etc/systemd/system.conf

# View overlay changes
sudo btrfs subvolume show /overlay/etc
```

#### 9.2.2 System Snapshots

BTRFS snapshots for system rollback:

```bash
# Create manual snapshot
sudo btrfs subvolume snapshot / /snapshots/manual-$(date +%Y%m%d)

# List snapshots
sudo btrfs subvolume list /

# Rollback to snapshot
sudo regicide-rollback --snapshot /snapshots/manual-20240101
```

### 9.3 OS Personality Swapping

The phrase "swap OS personality" refers to the ability to replace the read-only SquashFS system image while keeping your local changes and home directory intact. This is one of the most powerful features of RegicideOS's architecture.

#### 9.3.1 Understanding OS Personalities

An "OS personality" is essentially the complete system environment contained in a SquashFS image file. This includes:
- **Base system**: `/usr`, `/bin`, `/lib`, `/sbin`, and other core directories
- **Desktop environment**: COSMIC Desktop, GNOME, or other environments
- **System configuration**: Default settings and skel files
- **Package set**: Pre-installed applications and libraries

Your personal data remains in separate locations:
- **Overlay**: Local modifications to `/etc`, `/var`, and `/usr`
- **Home directory**: User files in `/home`
- **Persistent configuration**: Custom settings and installed packages

#### 9.3.2 The Personality Swap Mechanism

Because RegicideOS's boot loader (GRUB) always picks the **newest** `root-*.img` by modification time, swapping personalities is as simple as copying a new image file to the ROOTS partition.

#### 9.3.3 Step-by-Step Personality Swap

**While running RegicideOS or from a live USB:**

1. **Mount the ROOTS partition**
   ```bash
   sudo mount /dev/disk/by-label/ROOTS /mnt
   ```

2. **Drop the new image in place**
   ```bash
   sudo cp root-cosmic.img /mnt/
   sudo touch /mnt/root-cosmic.img   # ensure it has newest timestamp
   sync
   ```

3. **Reboot**

   GRUB's `10_linux` helper automatically does:
   ```bash
   newest=$(ls -t /mnt/root-*.img | head -n1)
   ```
   and builds the menu entry pointing at that file. On the next boot:
   - The initrd loopsquash-mounts `root-cosmic.img`
   - The system overlays your writable BTRFS subvolumes
   - You are instantly running the new personality

#### 9.3.4 Rolling Back Personalities

Keep the old file for easy rollback:

```bash
# Backup current personality
sudo mv /mnt/root-gnome.img /mnt/root-gnome.img.bak
```

If you want to switch back to GNOME:
```bash
sudo touch /mnt/root-gnome.img.bak   # make it newest again
reboot
```

#### 9.3.5 Managing Multiple Personalities

You can keep multiple personalities available:

```bash
# List available personalities
ls -la /mnt/root-*.img

# Switch to a specific personality
sudo touch /mnt/root-desired-personality.img
reboot
```

#### 9.3.6 Deleting Obsolete Images

Clean up old personalities to save space:

```bash
# Remove old personality
sudo rm /mnt/root-old.img

# Free space immediately (BTRFS-specific)
sudo btrfs filesystem sync /mnt
```

#### 9.3.7 Creating Custom Personalities

You can create your own personalities:

```bash
# Create a custom personality from current system
sudo mksquashfs /tmp/rootfs /mnt/root-custom.img -comp zstd -Xcompression-level 19

# Ensure it's the newest for next boot
sudo touch /mnt/root-custom.img
```

#### 9.3.8 Best Practices

1. **Always backup before swapping**: Keep your current personality until you're sure the new one works
2. **Test in a VM first**: Try new personalities in a virtual machine before deploying on hardware
3. **Keep known-good personalities**: Maintain at least one stable personality for quick recovery
4. **Monitor disk space**: Remove unused personalities to free up ROOTS partition space
5. **Verify image integrity**: Always check that downloaded images are complete and uncorrupted

#### 9.3.9 Troubleshooting

**If boot fails after personality swap:**
1. Reboot and select an older personality from the GRUB menu
2. Use `overlay=disabled` kernel parameter to boot clean
3. Verify the new image file isn't corrupted
4. Check available disk space in ROOTS partition

**If GRUB doesn't detect new personality:**
1. Ensure the image file has the correct extension (`root-*.img`)
2. Verify the file is executable and readable
3. Manually run `sudo update-grub` from a live environment

This system makes it trivial to:
- Try different desktop environments
- Test major system updates safely
- Switch between development and production environments
- Maintain multiple configurations for different use cases
- Roll back broken updates instantly

### 9.4 Service Management

#### 9.4.1 System Services

```bash
# Manage core services
sudo systemctl status networking
sudo systemctl restart bluetooth

# View service logs
sudo journalctl -u systemd-networkd
```

#### 9.4.2 AI Agent Management

```bash
# Control AI agents
sudo systemctl enable portcl btrmind
sudo systemctl start portcl btrmind

# Monitor agent performance
portcl dashboard
btrmind monitor
```

---

## 10. Troubleshooting

### 10.1 Installation Issues

#### 10.1.1 Common Installation Problems

**Flavor Not Available Error:**
```
[ERROR] The cosmic-desktop flavour is not available in the repository
```
**Solution**: This is fixed in newer installer versions. The correct flavor is `cosmic-fedora`. If using older installer:
```bash
# Manual fix in config file
sed -i 's/cosmic-desktop/cosmic-fedora/g' regicide-config.toml
```

**LUKS Device Not Found Error:**
```
[ERROR] Device /dev/disk/by-label/XENIA does not exist
```
**Solution**: Fixed in newer installer versions. LUKS containers don't have labels until opened. If using older installer:
```bash
# Manual LUKS setup workaround
sudo cryptsetup luksFormat /dev/sdX3  # Replace with your partition
sudo cryptsetup open /dev/sdX3 XENIA
sudo mkfs.btrfs -L XENIA /dev/mapper/XENIA
sudo cryptsetup close XENIA
```

**System Suspend During Partitioning:**
```
Installer crashes with Option::unwrap() panic at main.rs:1206
```
**Solution**: System went into suspend during partitioning. Fixed in newer installer, but manual prevention:
```bash
# Prevent suspend before installation
sudo systemctl mask sleep.target suspend.target hibernate.target hybrid-sleep.target
sudo loginctl disable-lid-switch
```

**Missing sgdisk Warning:**
```
[WARN] sgdisk not available, skipping EFI boot flag
```
**Solution**: Fixed in newer installer with automatic gdisk installation. Manual fix:
```bash
# Install gdisk package
sudo dnf install -y gdisk  # Fedora
# or
sudo apt install gdisk  # Ubuntu
```

**BTRFS Labeling Failure:**
```
mkfs.btrfs -L ROOTS /dev/sdX2 failed
```
**Solution**: Enhanced error handling in newer installer provides better diagnostics. Manual checks:
```bash
# Check if partition exists
lsblk /dev/sdX2
# Try manual formatting with verbose output
sudo mkfs.btrfs -L ROOTS -f /dev/sdX2
```

#### 10.1.2 EFI Partition Problems

If EFI partition creation fails:

```bash
# Manual partition creation
sudo gdisk /dev/sda
# Create GPT table
# Add 512MB EFI System Partition (type EF00)
# Add remaining space as Linux filesystem (type 8300)

# Format EFI partition
sudo mkfs.vfat -F 32 -n EFI /dev/sda1

# Format root partition
sudo mkfs.btrfs -L ROOTS /dev/sda2
```

#### 10.1.3 Network Connectivity

If installer cannot download system image:

```bash
# Test connectivity
ping -c 3 repo.xenialinux.com

# Check DNS resolution
nslookup repo.xenialinux.com

# Manual network configuration
sudo ip addr add 192.168.1.100/24 dev eth0
sudo ip route add default via 192.168.1.1
```

### 10.2 Boot Issues

#### 10.2.1 GRUB Recovery

If system fails to boot:

1. Boot from live environment
2. Mount RegicideOS filesystem:
   ```bash
   sudo mkdir /mnt/regicide
   sudo mount -L ROOTS /mnt/regicide
   sudo mount -L EFI /mnt/regicide/boot/efi
   ```
3. Reinstall bootloader:
   ```bash
   sudo chroot /mnt/regicide
   grub-install --target=x86_64-efi --efi-directory=/boot/efi
   grub-mkconfig -o /boot/grub/grub.cfg
   ```

#### 10.2.2 Snapshot Recovery

Rollback to previous working state:

```bash
# Boot from live environment
sudo mkdir /mnt/regicide
sudo mount -L ROOTS /mnt/regicide

# List available snapshots
sudo btrfs subvolume list /mnt/regicide

# Set default subvolume to snapshot
sudo btrfs subvolume set-default <snapshot-id> /mnt/regicide
```

### 10.3 AI Agent Issues

#### 10.3.1 PortCL Problems

If PortCL is not optimizing correctly:

```bash
# Reset learning model
sudo systemctl stop portcl
sudo rm /var/lib/portcl/model.pt
sudo systemctl start portcl

# Check configuration
portcl config validate

# View detailed logs
sudo journalctl -u portcl -n 100
```

#### 10.3.2 BtrMind Storage Issues

If BtrMind reports storage problems:

```bash
# Manual storage analysis
sudo btrfs filesystem usage /
sudo btrfs filesystem df /

# Force cleanup
sudo btrmind cleanup --force

# Disable AI if needed
sudo systemctl disable btrmind
```

---

**Document Version**: 1.0  
**Last Updated**: 2024  
**License**: GNU General Public License v3.0

---

<div align="center">

**RegicideOS Handbook**

*System Administration and Reference Guide*

</div>

