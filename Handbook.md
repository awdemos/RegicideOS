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
6. [AI-Powered System Management](#6-ai-powered-system-management)
7. [Package Management](#7-package-management)
8. [Development Environment](#8-development-environment)
9. [System Administration](#9-system-administration)
10. [Troubleshooting](#10-troubleshooting)
11. [Advanced Topics](#11-advanced-topics)

---

## 1. Introduction

### 1.1 What is RegicideOS?

RegicideOS is a revolutionary Linux distribution that represents the future of operating systems. Built as a specialized fork of Xenia Linux, RegicideOS embodies two core principles:

- **Rust-First Architecture**: Every system component that can be implemented in Rust is being migrated to Rust for maximum memory safety and performance
- **AI-Powered Operations**: Intelligent system management using continual reinforcement learning for autonomous optimization

### 1.2 Philosophy and Design Goals

RegicideOS challenges the status quo of traditional operating systems by:

- **Eliminating Memory Vulnerabilities**: Through Rust's ownership model and zero-cost abstractions
- **Autonomous System Management**: AI agents that learn and adapt to optimize system performance
- **Immutable System Architecture**: Read-only root filesystem for enhanced security and reliability
- **Future-Proof Design**: Architecture ready for next-generation technologies like the Asterinas kernel

### 1.3 Key Differentiators from Xenia Linux

| Feature | Xenia Linux | RegicideOS |
|---------|-------------|------------|
| **Desktop Environment** | Multiple choices | Cosmic Desktop only |
| **AI Integration** | Limited | Core system feature |
| **Language Focus** | Mixed ecosystem | Rust-first approach |
| **System Updates** | Traditional | Immutable/atomic |
| **Package Management** | Standard repositories | Overlay-based with AI optimization |

---

## 2. System Requirements

### 2.1 Hardware Requirements

#### Minimum Specifications
- **Processor**: 64-bit x86 CPU (Intel/AMD)
- **Memory**: 4GB RAM
- **Storage**: 12GB available disk space
- **Firmware**: UEFI or Legacy BIOS
- **Graphics**: Any GPU with basic framebuffer support

#### Recommended Specifications
- **Processor**: Multi-core x86-64 with AVX2 support
- **Memory**: 8GB+ RAM (for AI features)
- **Storage**: 20GB+ SSD storage
- **Firmware**: UEFI with Secure Boot support
- **Graphics**: GPU with Vulkan/OpenGL 4.0+ support
- **Network**: Ethernet or Wi-Fi for package updates

### 2.2 Supported Architectures

Currently supported:
- `x86_64` (AMD64)

Future planned support:
- `aarch64` (ARM64)
- `riscv64` (RISC-V)

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
# Install dependencies
sudo dnf install git curl gcc

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone RegicideOS repository
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS/installer
```

### 3.2 Installation Methods

#### 3.2.1 Interactive Installation

The standard installation method with guided prompts:

```bash
# Build the installer
cargo build --release

# Run interactive installation
sudo ./target/release/installer
```

The installer will guide you through:
1. **Drive Selection**: Choose target installation drive
2. **Filesystem Layout**: BTRFS with encryption options
3. **User Setup**: Create administrative user account
4. **Application Sets**: Choose minimal or recommended packages

#### 3.2.2 Automated Installation

For scripted deployments, create a configuration file:

```bash
# Create configuration
cat > regicide-config.toml << EOF
drive = "/dev/sda"
repository = "https://repo.xenialinux.com/releases/"
flavour = "cosmic-fedora"
release_branch = "main"
filesystem = "btrfs"
username = "admin"
applications = "recommended"
EOF

# Run automated installation
sudo ./target/release/installer -c regicide-config.toml
```

### 3.3 Installation Process

The RegicideOS installer performs these steps:

1. **Drive Partitioning**
   - EFI System Partition (512MB, FAT32)
   - Root Partition (remaining space, BTRFS)

2. **Filesystem Setup**
   - Create BTRFS subvolumes for overlay system
   - Configure read-only root filesystem
   - Set up writable overlays for `/etc`, `/var`, `/usr`

3. **System Image Download**
   - Download compressed system image from Xenia repositories
   - Verify integrity and decompress to target

4. **Bootloader Installation**
   - Install GRUB for EFI or Legacy BIOS
   - Configure boot parameters for immutable system

5. **AI Component Setup**
   - Initialize AI system monitoring agents
   - Configure continual learning frameworks
   - Set up default optimization policies

### 3.4 Manual Installation with OverlayFS ⚠️ **DEPRECATED**

> **⚠️ DEPRECATION NOTICE**: This 4-partition overlayfs layout is deprecated. RegicideOS now recommends the BTRFS-native architecture described in Section 4.1 for better performance, snapshots, and simpler management. This section is provided for reference only and should not be used for new installations.

Below is the legacy walk-through showing how the old 4-partition overlayfs system works. This should only be used for understanding existing systems or migration purposes.

Run the commands on a live ISO (or read them only for the concept—either way you will see the whole picture).

------------------------------------------------
**0. Start with empty disk (assume `/dev/sda`)**
------------------------------------------------

```
gdisk /dev/sda
```

Create four partitions and give them the **GPT partition-labels** that the Xenia initrd looks for:

| number | size       | label   | code | purpose |
|--------|------------|---------|------|---------|
| 1      | 512 MiB    | EFI     | EF00 | ESP (FAT32) |
| 2      | 5 GiB      | ROOTS   | 8300 | ext4, will hold SquashFS |
| 3      | 2 GiB      | OVERLAY | 8300 | ext4, writable layer |
| 4      | rest       | HOME    | 8300 | ext4, /home |

(You can use `fdisk -t gpt` or `parted`—only the **label string** matters.)

------------------------------------------------
**1. Make the filesystems**
------------------------------------------------

```
mkfs.vfat -n EFI     /dev/sda1
mkfs.ext4 -L ROOTS   /dev/sda2
mkfs.ext4 -L OVERLAY /dev/sda3
mkfs.ext4 -L HOME    /dev/sda4
```

------------------------------------------------
**2. Build (or download) the read-only master image**
------------------------------------------------

You need *one* directory tree that contains a complete Gentoo/rootfs.
You can:

- `emerge --root=/tmp/rootfs -N @world` on another Gentoo box, or
- extract the official Xenia root tarball into `/tmp/rootfs`.

------------------------------------------------
**3. Turn that tree into a SquashFS file**
------------------------------------------------

```
mksquashfs /tmp/rootfs  /tmp/root.img  -comp zstd -Xcompression-level 19
```

`root.img` is now the immutable "golden master".

------------------------------------------------
**4. Populate the partitions**
------------------------------------------------

Mount them anywhere convenient:

```
mkdir -p /mnt/{efi,roots,overlay,home}
mount /dev/sda1 /mnt/efi
mount /dev/sda2 /mnt/roots
mount /dev/sda3 /mnt/overlay
mount /dev/sda4 /mnt/home
```

Copy the image:

```
cp /tmp/root.img /mnt/roots/
```

Create empty overlay directories (the initrd will use them):

```
mkdir -p /mnt/overlay/upper /mnt/overlay/work
```

(You can leave `/mnt/home` empty; the first boot will create lost+found and any dot-files.)

------------------------------------------------
**5. Install GRUB so UEFI can start it**
------------------------------------------------

We still need a kernel + initrd **inside** the SquashFS so GRUB can load them.
The easiest way is to bind-mount the SquashFS once, chroot into it, build a kernel/initrd, then copy those two files back to the ESP.

```
mkdir /tmp/sq
mount -t squashfs /mnt/roots/root.img /tmp/sq
cp -a /tmp/sq/boot/{vmlinuz-*,initramfs-*} /mnt/roots/
umount /tmp/sq
```

Now install GRUB (still from the live environment):

```
grub-install --target=x86_64-efi --efi-directory=/mnt/efi --boot-directory=/mnt/roots/boot --removable
```

`--removable` makes the firmware find `\EFI\BOOT\BOOTX64.EFI` (no NVRAM entry needed).

Create a minimal `grub.cfg` (on the ESP, or in `/mnt/roots/boot/grub/grub.cfg`):

```
set root=(hd0,gpt2)        # ROOTS partition
linux  /boot/vmlinuz-<ver> root=LABEL=ROOTS overlay=LABEL=OVERLAY home=LABEL=HOME quiet
initrd /boot/initramfs-<ver>.img
```

------------------------------------------------
**6. First boot – what the initrd does**
------------------------------------------------

- Opens LUKS if you encrypted anything.
- Mounts **ROOTS** read-only → finds `root.img`.
- Mounts **OVERLAY** read-write.
- Creates the OverlayFS merge:
  ```
  mount -t overlay overlay -o lowerdir=/rootfs,upperdir=/overlay/upper,workdir=/overlay/work /newroot
  ```
- Binds **HOME** onto `/newroot/home`.
- `switch_root` → systemd starts.

You now have a **writable** system whose *base* is still the pristine SquashFS.

------------------------------------------------
**7. Daily life & upgrades**
------------------------------------------------

- Install packages, edit configs, add users – everything lands in **OVERLAY**.
- When a new OS release appears, just drop the new `root-new.img` into the ROOTS partition and reboot; GRUB picks the newest file by mtime.
- If you ever break the overlay, boot with `overlay=disabled` (GRUB menu entry) – you are instantly back to the clean SquashFS.

------------------------------------------------

That is literally all the "magic": four labelled partitions, one SquashFS, one overlay mount.
Once you see the commands above, the phrase "single system image" just means "every machine can share the same SquashFS and only keep its *local* differences in a thin writable layer."

#### 3.4.1 Architecture Benefits

1. **System Integrity**: Base system cannot be accidentally modified
2. **Easy Updates**: Just drop in a new SquashFS image
3. **Fast Rollback**: Boot without overlay to return to clean state
4. **Storage Efficiency**: Single system image shared across multiple machines
5. **Snapshot Support**: Overlay state can be snapshotted and restored
6. **Security**: Immutable root reduces attack surface

### 3.5 Post-Installation

After successful installation:

1. **Remove installation media** and reboot
2. **Complete initial setup** through Cosmic Desktop
3. **Enable AI features** (optional, see Section 6)
4. **Install additional software** through overlay system

---

## 4. System Architecture

### 4.1 BTRFS-Native Architecture

RegicideOS uses BTRFS-native snapshots and sub-volumes to provide the same illusion of "one immutable image + a thin writable layer", while also giving you **copy-on-write snapshots for free**.

#### 4.1.1 Disk Layout

```
/dev/sda1   512 MB  FAT32   label "EFI"
/dev/sda2   rest    BTRFS   label "ROOTS"   ← single big BTRFS partition
```

Inside that second partition we create **five fixed sub-volumes**:

```
ROOTS (top-level BTRFS)
├─ @                ← empty, reserved for root-image *files*
├─ @overlay         ← top-level for overlays
│  ├─ @etc           ← writable layer for /etc
│  ├─ @var           ← writable layer for /var
│  └─ @usr           ← writable layer for /usr
└─ @home             ← /home tree
```

*(The installer calls them `overlay`, `etc`, `var`, `usr`, `home` without the "@" the @ is just a BTRFS convention.)*

#### 4.1.2 Boot Sequence with BTRFS

1. **UEFI → GRUB → kernel + initrd** (loaded from a **SquashFS** that lives in the *root* of ROOTS).
2. **Early user-space** (foxmount helper) does **not** use overlayfs at all; it does:
   ```
   mount -t btrfs -o subvol=@etc  /dev/sda2  /sysroot/etc
   mount -t btrfs -o subvol=@var  /dev/sda2  /sysroot/var
   mount -t btrfs -o subvol=@usr  /dev/sda2  /sysroot/usr
   mount -t btrfs -o subvol=@home /dev/sda2  /sysroot/home
   ```
   The **SquashFS** is loop-mounted **once** on `/sysroot`; the individual **BTRFS sub-volumes are bind-mounted on top** of the directories that must stay writable.
3. **Switch-root** into `/sysroot` → systemd starts.

#### 4.1.3 Why BTRFS is Better for RegicideOS

| classic overlayfs | BTRFS snapshots |
|---|---|
| upperdir + workdir must live on **same fs** as each other, but **can be different** from lowerdir → you still need two partitions (ROOTS + OVERLAY). | Everything is **one partition**; no extra partition for "OVERLAY". |
| No built-in snapshot of the **writable layer**. | Every sub-volume can be snapshotted instantly: `btrfs sub snap -r @etc @etc-2025-09-21` |
| Deleting the old lower SquashFS frees **zero** space until you delete the overlay too. | Deleting an old `root.img` **immediately** frees space (BTRFS reflinks are per-file, not per-dir). |
| Roll-back of **system** means rebooting into old SquashFS; roll-back of **user changes** means wiping the whole upperdir. | You can roll **either** direction independently: `snapper rollback @etc` to yesterday while keeping today's `/usr` layer. |
| Overlayfs white-outs sometimes confuse backup tools. | Plain directories and files—nothing exotic. |

#### 4.1.4 Snapshot Workflow

```
# take a consistent checkpoint before big changes
btrfs subvolume snapshot -r @etc @etc-before-emerge
btrfs subvolume snapshot -r @usr @usr-before-emerge

# do potentially dangerous stuff
emerge -avuDN @world

# if it breaks, roll back in seconds
btrfs subvolume delete @etc
btrfs subvolume snapshot @etc-before-emerge @etc
reboot   # now you're on yesterday's /etc with today's kernel
```

#### 4.1.5 Key Benefits

- **Still boots from a read-only SquashFS** (the "golden image").
- **Writable parts are not an overlayfs layer** any more; they are **individual BTRFS sub-volumes** that get **bind-mounted on top** of the squashfs directories.
- This gives the same "immutable base + disposable changes" behaviour, **plus** instant snapshots, roll-backs, and single-partition simplicity—reason enough for the docs to say *"Use BTRFS only; the old layouts are deprecated."*

### 4.2 BTRFS Command Reference

Here are the essential BTRFS commands for managing your RegicideOS system:

#### 4.2.1 Subvolume Management

```bash
# List all subvolumes
btrfs subvolume list /mnt/roots

# Create a new subvolume
btrfs subvolume create /mnt/roots/@new-subvol

# Delete a subvolume
btrfs subvolume delete /mnt/roots/@old-subvol

# Create a snapshot (read-only)
btrfs subvolume snapshot -r /mnt/roots/@etc /mnt/roots/@etc-2025-09-20

# Create a snapshot (writable)
btrfs subvolume snapshot /mnt/roots/@etc /mnt/roots/@etc-working

# Show subvolume information
btrfs subvolume show /mnt/roots/@etc
```

#### 4.2.2 System Maintenance

```bash
# Check filesystem status
btrfs filesystem df /mnt/roots
btrfs filesystem usage /mnt/roots

# Balance the filesystem (optimizes data distribution)
btrfs balance start /mnt/roots

# Scrub the filesystem (checks for data errors)
btrfs scrub start /mnt/roots
btrfs scrub status /mnt/roots

# Defragment files
btrfs filesystem defrag -r /mnt/roots/@usr

# Enable compression on new files
btrfs property set /mnt/roots/@var compression zstd
```

#### 4.2.3 Snapshot Management

```bash
# Create consistent system snapshot before major changes
btrfs subvolume snapshot -r /mnt/roots/@etc /mnt/roots/@etc-before-emerge
btrfs subvolume snapshot -r /mnt/roots/@usr /mnt/roots/@usr-before-emerge
btrfs subvolume snapshot -r /mnt/roots/@var /mnt/roots/@var-before-emerge

# Roll back after failed operation
btrfs subvolume delete /mnt/roots/@etc
btrfs subvolume snapshot /mnt/roots/@etc-before-emerge /mnt/roots/@etc

# Send/receive snapshots (for backup/transfer)
btrfs send /mnt/roots/@etc-2025-09-20 | btrfs receive /backup/snapshots/

# List snapshots
btrfs subvolume list -s /mnt/roots
```

#### 4.2.4 Migration from OverlayFS

```bash
# If migrating from old overlayfs layout, create subvolumes:
btrfs subvolume create /mnt/roots/@etc
btrfs subvolume create /mnt/roots/@usr
btrfs subvolume create /mnt/roots/@var
btrfs subvolume create /mnt/roots/@home

# Copy existing overlay data to new subvolumes
cp -a /mnt/overlay/etc/* /mnt/roots/@etc/
cp -a /mnt/overlay/usr/* /mnt/roots/@usr/
cp -a /mnt/overlay/var/* /mnt/roots/@var/

# Set default subvolume if needed
btrfs subvolume set-default /mnt/roots/@
```

### 4.3 Security Model

- **Read-Only Root**: Base system cannot be modified during runtime
- **Verified Boot**: System image integrity verified at boot
- **Namespace Isolation**: Containers provide application isolation
- **Memory Safety**: Rust components eliminate entire vulnerability classes

---

## 5. Core Components

### 5.1 Cosmic Desktop Environment

RegicideOS exclusively ships with Cosmic Desktop, System76's next-generation desktop environment:

#### Features:
- **Rust Implementation**: Built with Iced framework for performance
- **Wayland Native**: Modern display protocol support
- **GPU Acceleration**: Hardware-accelerated graphics pipeline
- **Tiling Window Manager**: Efficient workspace organization

#### Configuration:
```bash
# Cosmic settings are stored in
~/.config/cosmic/

# Example: Enable tiling by default
cosmic-settings set tiling.default true

# Configure display scaling
cosmic-settings set display.scale-factor 1.5
```

### 5.2 System Components

#### 5.2.1 Init System
- **systemd**: Service and process management
- **Custom Units**: AI agents run as systemd services

#### 5.2.2 Container Runtime
- **Distrobox**: Application containerization
- **Podman Backend**: Secure, rootless containers
- **Integration**: Seamless desktop application support

### 5.3 Network Management

RegicideOS uses NetworkManager with systemd integration:

```bash
# View network status
nmcli general status

# Connect to WiFi
nmcli dev wifi connect "SSID" password "password"

# Configure static IP
nmcli con add type ethernet ifname eth0 ip4 192.168.1.100/24 gw4 192.168.1.1
```

---

## 6. AI-Powered System Management

RegicideOS implements three primary AI agents for autonomous system management:

### 6.1 PortCL - Package Management Optimization

**PortCL** (Portage Continual Learning) optimizes package management operations using reinforcement learning.

#### Key Features:
- **Build Optimization**: Automatic parallelism adjustment based on system load
- **Dependency Management**: Intelligent build order optimization  
- **Resource Scheduling**: Off-peak operation scheduling for resource-intensive tasks
- **Continual Learning**: Adapts to changing system configurations

#### Configuration:
```toml
# /etc/portcl/config.toml
[monitoring]
poll_interval = 30  # seconds
metrics_history = 24  # hours

[learning]
model_path = "/var/lib/portcl/model.pt"
replay_buffer_size = 10000
learning_rate = 0.001

[actions]
enable_parallelism_adjustment = true
enable_build_reordering = true
enable_scheduling = true
```

#### Usage:
```bash
# Start PortCL service
sudo systemctl enable --now portcl.service

# View current status
portcl status

# Manual action trigger
portcl optimize --task build_optimization
```

### 6.2 BtrMind - Storage Management AI

**BtrMind** proactively manages BTRFS filesystem health using reinforcement learning.

#### Key Features:
- **Space Optimization**: Automatic cleanup of temporary files and caches
- **Compression Management**: Intelligent file compression based on access patterns
- **Metadata Balancing**: BTRFS metadata optimization
- **Snapshot Management**: Automated snapshot cleanup and rotation

#### Configuration:
```toml
# /etc/btrmind/config.toml
[thresholds]
warning_level = 85.0      # Disk usage percentage
critical_level = 95.0     # Critical threshold
emergency_level = 98.0    # Emergency cleanup

[actions]
enable_compression = true
enable_balance = true
enable_snapshot_cleanup = true
enable_temp_cleanup = true

[learning]
model_update_interval = 3600  # 1 hour
reward_smoothing = 0.95
exploration_rate = 0.1
```

#### Usage:
```bash
# Check BtrMind status
sudo systemctl status btrmind.service

# Manual space analysis
btrmind analyze

# Force cleanup action
sudo btrmind cleanup --aggressive
```

### 6.3 System Health Monitoring

#### 6.3.1 Metrics Collection
The AI agents collect comprehensive system metrics:

- **Performance**: CPU, memory, I/O utilization
- **Storage**: Disk usage, fragmentation, access patterns
- **Network**: Bandwidth utilization, connection quality
- **Applications**: Resource consumption, crash rates

#### 6.3.2 Learning and Adaptation
All agents implement continual reinforcement learning:

1. **State Observation**: Continuous system monitoring
2. **Action Selection**: AI-driven decision making
3. **Reward Calculation**: Performance improvement scoring
4. **Model Update**: Continuous learning without forgetting
5. **Knowledge Transfer**: Cross-agent information sharing

### 6.4 AI Agent Management

#### 6.4.1 Service Management
```bash
# View all AI services
sudo systemctl list-units "*mind*" "*portcl*"

# Enable AI monitoring
sudo systemctl enable portcl btrmind

# Disable AI features
sudo systemctl stop portcl btrmind
sudo systemctl disable portcl btrmind
```

#### 6.4.2 Performance Monitoring
```bash
# View agent performance logs
sudo journalctl -u portcl.service -f
sudo journalctl -u btrmind.service -f

# Check learning progress
portcl metrics --learning-progress
btrmind stats --model-performance
```

---

## 7. Package Management

### 7.1 Overlay System

RegicideOS uses a hybrid package management approach combining:

1. **Base System**: Immutable system image from Xenia repositories
2. **GURU Overlay**: Community-maintained packages
3. **Regicide Overlay**: Custom modifications and AI-enhanced packages

### 7.2 User Configuration and Dotfiles

RegicideOS provides official dotfiles for a consistent, modern development experience:

#### 7.2.1 Installing RegicideOS Dotfiles

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

#### 7.2.2 Dotfiles Customization

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

#### 7.2.3 System-wide Configuration

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

### 7.3 Rust Development Environment

#### 7.3.1 Rust Toolchain Management

RegicideOS provides comprehensive Rust development support:

```bash
# Install Rust toolchain (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update to latest stable
rustup update stable

# Install additional targets
rustup target add thumbv6m-none-eabi  # ARM Cortex-M
rustup target add riscv32imc-unknown-none-elf  # RISC-V
rustup target add wasm32-unknown-unknown  # WebAssembly
```

#### 7.3.2 Embedded Development

RegicideOS includes special support for embedded development:

```bash
# Install embedded tools
cargo install cargo-embed cargo-flash probe-run

# Create embedded project
cargo generate --git https://github.com/rust-embedded/cortex-m-quickstart

# Flash to hardware
cargo embed --target thumbv6m-none-eabi
```

### 7.3 Container-Based Applications

#### 7.4.1 Distrobox Integration

Most user applications run in containers for isolation:

```bash
# Create development environment
distrobox create --name dev --image fedora:39
distrobox enter dev

# Install applications in container
sudo dnf install code firefox thunderbird

# Applications appear in desktop menu automatically
```

#### 7.4.2 Flatpak Integration

System-wide applications use Flatpak:

```bash
# Install applications
flatpak install flathub org.mozilla.Firefox
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

**VS Code with Rust Extensions:**
```bash
# Install VS Code via Flatpak
flatpak install flathub com.visualstudio.code

# Recommended extensions
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb
code --install-extension serayuzgur.crates
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

RegicideOS includes Rust-native ML frameworks:

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

RegicideOS uses atomic updates for the base system:

```bash
# Check for updates
regicide-update check

# Download and prepare update
regicide-update download

# Apply update (requires reboot)
sudo regicide-update apply
```

#### 9.1.2 Overlay Updates

Package overlays are updated separately:

```bash
# Sync overlay repositories
sudo emerge --sync

# Update overlay packages
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

#### 10.1.1 EFI Partition Problems

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

#### 10.1.2 Network Connectivity

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

## 11. Advanced Topics

### 11.1 Custom AI Agent Development

#### 11.1.1 Agent Framework

RegicideOS provides a framework for custom AI agents:

```rust
// src/main.rs - Custom Agent Template
use regicide_ai::{Agent, ReinforcementLearner, SystemMetrics};
use anyhow::Result;

struct CustomAgent {
    learner: ReinforcementLearner,
    metrics: SystemMetrics,
}

impl Agent for CustomAgent {
    async fn observe(&mut self) -> Result<Vec<f64>> {
        // Collect system observations
        Ok(self.metrics.collect_all().await?)
    }
    
    async fn act(&self, action_id: usize) -> Result<()> {
        // Execute system actions
        match action_id {
            0 => self.no_action(),
            1 => self.optimize_memory(),
            2 => self.cleanup_caches(),
            _ => self.default_action(),
        }
    }
    
    fn calculate_reward(&self, prev_state: &[f64], curr_state: &[f64]) -> f64 {
        // Define reward function
        let improvement = curr_state[0] - prev_state[0];
        improvement * 10.0
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut agent = CustomAgent::new()?;
    agent.run_learning_loop().await
}
```

#### 11.1.2 Agent Integration

Register custom agents with the system:

```bash
# Install custom agent
cargo build --release
sudo cp target/release/custom-agent /usr/local/bin/

# Create systemd service
sudo tee /etc/systemd/system/custom-agent.service > /dev/null << EOF
[Unit]
Description=Custom AI Agent
After=network.target

[Service]
Type=simple
User=nobody
ExecStart=/usr/local/bin/custom-agent
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# Enable service
sudo systemctl enable custom-agent.service
sudo systemctl start custom-agent.service
```

### 11.2 Kernel Migration Preparation

#### 11.2.1 Asterinas Kernel Support

RegicideOS is designed for future migration to the Asterinas kernel:

```bash
# Check Asterinas compatibility
regicide-kernel-check --asterinas

# Enable Asterinas boot option (future)
sudo regicide-kernel-switch --target asterinas

# Fallback to Linux kernel
sudo regicide-kernel-switch --target linux
```

### 11.3 Performance Optimization

#### 11.3.1 System Tuning

```bash
# Enable performance governor
sudo cpupower frequency-set -g performance

# Optimize BTRFS
sudo btrfs filesystem balance start -dusage=50 /
sudo btrfs filesystem defragment -r -v /home

# Memory optimization
echo 'vm.swappiness = 10' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

#### 11.3.2 AI Agent Tuning

```bash
# Adjust PortCL learning parameters
sudo nano /etc/portcl/config.toml
# Increase learning_rate for faster adaptation
# Decrease replay_buffer_size for less memory usage

# BtrMind optimization
sudo nano /etc/btrmind/config.toml
# Lower poll_interval for more responsive monitoring
# Adjust thresholds based on usage patterns
```

### 11.4 Security Hardening

#### 11.4.1 System Hardening

```bash
# Enable firewall
sudo systemctl enable --now firewalld
sudo firewall-cmd --set-default-zone=public

# Secure boot verification
sudo mokutil --sb-state

# Audit system
sudo systemctl enable --now auditd
```

#### 11.4.2 Container Security

```bash
# Secure container defaults
echo 'cgroup_enable=memory swapaccount=1' | sudo tee -a /boot/cmdline.txt

# SELinux/AppArmor profiles (TBD)
# Future: Rust-based security framework
```

---

## Appendices

### Appendix A: Configuration Files

#### A.1 PortCL Configuration Template
```toml
# /etc/portcl/config.toml
[monitoring]
poll_interval = 30
metrics_history = 24

[learning]
model_path = "/var/lib/portcl/model.pt"
replay_buffer_size = 10000
learning_rate = 0.001
discount_factor = 0.99

[actions]
enable_parallelism_adjustment = true
enable_build_reordering = true
enable_scheduling = true
max_parallel_jobs = 8

[thresholds]
cpu_high = 90.0
memory_high = 85.0
disk_critical = 95.0
```

#### A.2 BtrMind Configuration Template
```toml
# /etc/btrmind/config.toml
[monitoring]
poll_interval = 60
trend_analysis_window = 24

[thresholds]
warning_level = 85.0
critical_level = 95.0
emergency_level = 98.0

[actions]
enable_compression = true
enable_balance = true
enable_snapshot_cleanup = true
enable_temp_cleanup = true

[learning]
model_update_interval = 3600
reward_smoothing = 0.95
exploration_rate = 0.1
```

### Appendix B: Command Reference

#### B.1 System Commands
```bash
# System information
regicide-info --version
regicide-info --components
regicide-info --ai-status

# Update system
regicide-update check
regicide-update download
regicide-update apply

# Snapshots
regicide-snapshot create
regicide-snapshot list
regicide-rollback --snapshot <id>
```

#### B.2 AI Agent Commands
```bash
# PortCL
portcl status
portcl metrics
portcl optimize --task <task>
portcl config validate

# BtrMind  
btrmind status
btrmind analyze
btrmind cleanup [--aggressive]
btrmind stats
```

---

### Appendix C: Future Development Roadmap

#### C.1 Short-term Goals (6 months)
- [ ] Complete Cosmic Desktop integration
- [ ] Stable AI agent implementations
- [ ] Comprehensive testing suite
- [ ] Community overlay repository

#### C.2 Medium-term Goals (1-2 years)
- [ ] Advanced AI capabilities (natural language control)
- [ ] Multi-agent coordination
- [ ] ARM64 architecture support
- [ ] Enhanced security framework

#### C.3 Long-term Vision (2+ years)
- [ ] Asterinas kernel migration
- [ ] Distributed system capabilities  
- [ ] Quantum-resistant cryptography
- [ ] Neural network hardware acceleration

---

**Document Version**: 1.0  
**Last Updated**: 2024  
**License**: GNU General Public License v3.0

---

<div align="center">

**RegicideOS - The Future of Operating Systems**

*Built with Rust 🦀 • Powered by AI 🤖 • Designed for Tomorrow 🚀*

</div>
