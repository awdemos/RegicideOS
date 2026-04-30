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

RegicideOS is a specialized fork of Xenia Linux focused on:

- **Rust-First Architecture**: System components migrated to Rust for memory safety and performance
- **Immutable System Architecture**: Read-only base filesystem for enhanced security and reliability
- **AI-Integrated**: AI capabilities at the system level for predictive maintenance and context-aware assistance

### 1.2 Key Differentiators from Xenia Linux

| Feature | Xenia Linux | RegicideOS |
|---------|-------------|------------|
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
- **Firmware**: UEFI or Legacy BIOS

#### Recommended Specifications
- **Processor**: Multi-core x86-64
- **Memory**: 8GB+ RAM
- **Storage**: 30GB+ SSD storage
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
# Clone RegicideOS repository
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS

# Install all build dependencies (git, gcc, rust, btrfs-progs, cryptsetup, etc.)
# This auto-detects your distro and uses the correct package manager
./scripts/install-dependencies.sh

# Build installer (now without warnings)
cd installer
cargo build --release

# Verify build completed successfully
./target/release/installer --version
```

> **⚠️ IMPORTANT**: Prevent system suspend during installation to avoid state corruption, especially when using LUKS encryption. The installer now handles this automatically, but manual prevention is recommended for reliability.

### 3.2 Installation Methods

#### 3.2.1 Using Pre-built Installer (Recommended)

The easiest installation method is to download the pre-built installer binary from GitHub releases:

```bash
# Download the pre-built installer
curl -L -o regicide-installer \
  https://github.com/awdemos/RegicideOS/releases/latest/download/regicide-installer

# Make executable and run
chmod +x regicide-installer
sudo ./regicide-installer

# Or run with configuration file
sudo ./regicide-installer -c regicide-config.toml
```

The installer will guide you through:
1. **Drive Selection**: Choose target installation drive
2. **Filesystem Layout**: BTRFS with LUKS encryption (recommended)
3. **User Setup**: Create administrative user account
4. **Application Sets**: Choose minimal or recommended packages

#### 3.2.2 Building from Source

If you prefer to build the installer from source or need to customize it:

```bash
# Build the installer from source
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
repository = "https://repo.regicideos.org/releases/"
flavour = "cosmic-fedora"
release_branch = "main"
filesystem = "btrfs_encryption_dev"
username = "admin"
applications = "recommended"
EOF

# Run with pre-built installer (recommended)
curl -L -o regicide-installer \
  https://github.com/awdemos/RegicideOS/releases/latest/download/regicide-installer
chmod +x regicide-installer
sudo ./regicide-installer -c regicide-config.toml

# Or run with source-built installer
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

4. **System Image Download**
   - Download compressed `root.img` tarball from RegicideOS repositories
   - Uses `cosmic-fedora` flavor
   - Verify image integrity with checksums

5. **Bootloader Installation**
   - Install GRUB for EFI or Legacy BIOS
   - Configure boot parameters for immutable system
   - **Configure LUKS initramfs support BEFORE GRUB installation**
   - Install GRUB with crypto modules for encrypted boot

6. **Post-Installation Configuration**
   - Set up overlay filesystem mounts (/etc, /var, /usr)
   - Configure LUKS initramfs scripts and crypttab
   - Generate GRUB configuration with dynamic UUID detection

7. **User Account Creation**
   - Create administrative user with sudo access
   - Setup user home directory
   - Configure user groups (wheel, video, audio, etc.)

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

RegicideOS uses a **4-Partition Overlayfs architecture** inherited from its upstream project:

```
/dev/sda1   512 MB  FAT32   label "EFI"
/dev/sda2   ~12-20GB  BTRFS    label "ROOTS"  (read-only base system)
/dev/sda3   ~4-8GB   BTRFS    label "OVERLAY"  (writable overlay layers)
/dev/sda4   Remaining  LUKS-encrypted BTRFS label "HOME"  (user data)
```

**Overlay Structure:**
```
/mnt/gentoo/           # Read-only base system from ROOTS partition
/mnt/root/overlay/      # Writable overlay for /etc, /var, /usr
/mnt/root/home/         # User home directory (bind-mounted to overlay/home)
```

**Boot Process:**
1. **UEFI → GRUB → kernel**
2. **initrd** loads and mounts:
   - ROOTS partition as read-only base system (via `root.img` template or direct mount)
   - Overlayfs layers for `/etc`, `/var`, `/usr` (writable)
   - `/home` partition (writable, LUKS-encrypted BTRFS)
3. **systemd** starts with overlays in place

### 4.2 Benefits of Current Architecture

- **Simplicity**: Proven overlayfs approach
- **Reliability**: Read-only base cannot be corrupted during normal operation
- **Instant Rollback**: Simply download previous system image
- **Atomic Updates**: System updates via new `root.img` tarball images
- **LUKS Encryption**: Full LUKS encryption support with dynamic partition detection

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

### 6.1 Current Implementation: Direct Download Model

The installer **does not use Foxmerge** for package management. Instead:

**Base System:**
- Downloads compressed `root.img` tarball from RegicideOS repositories
- Root image contains: `cosmic-fedora` flavor with minimal packages
- No package management during installation
- System updates via atomic image replacement

**Overlay Packages:**
- Installed into overlay directories (`/etc`, `/var`, `/usr`)
- Managed by direct system package tools (dnf, emerge, etc.)
- No overlay-specific package manager

**Architecture Decision:**
- Direct download model was chosen for **simplicity and reliability**
- Package management happens **post-installation** by system package tools
- Foxmerge was described in early planning but was not implemented

### 6.2 Package Installation Workflow

**During Installation:**
```bash
# No package installation - uses pre-built system image

# Post-Installation (user-initiated):
sudo dnf install <package>           # Fedora-style
sudo emerge <package>              # Gentoo-style overlays
flatpak install <app>              # Flatpak applications
```

**System Updates:**
- Atomic: Download new `root.img` tarball, reboot
- Incremental: Overlay packages updated via system tools

---

## 7. Development Environment

### 7.1 Development Environment Setup

RegicideOS is optimized for modern development with comprehensive tooling. After installing official dotfiles (see Section 7.3), you have a complete development environment:

#### 7.1.1 IDE Setup

**Zed Editor:**
```bash
# Install Zed via Flatpak
flatpak install flathub dev.zed.Zed

# Recommended extensions
zed --install-extension rust-analyzer
zed --install-extension lldb
zed --install-extension crates
```

#### 7.1.2 Development Tools

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

### 7.2 AI/ML Development

#### 7.2.1 Machine Learning Frameworks

Some recommended Rust-native ML frameworks:

```bash
# PyTorch bindings for Rust
cargo add tch

# Candle - Rust-native ML framework
cargo add candle-core candle-nn

# Tokenizers and NLP
cargo add tokenizers hf-hub
```

#### 7.2.2 Development Containers

AI development environments in containers:

```bash
# Create ML development environment
distrobox create --name ml-dev --image fedora:39
distrobox enter ml-dev

# Install additional tools
pip install jupyter transformers datasets
```

### 7.3 User Configuration and Dotfiles

RegicideOS provides official dotfiles for a consistent, modern development experience:

#### 7.3.1 Installing RegicideOS Dotfiles

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

#### 7.3.3 Backup and Restore

```bash
# Uninstall dotfiles (creates backup)
uninstall-regicide-dotfiles

# Restore from backup
uninstall-regicide-dotfiles --restore-backup
```

### 7.4 System Development

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

### 7.5 Contributing to RegicideOS

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

## 8. System Administration

### 8.1 System Updates

RegicideOS uses an atomic update system via `root.img` tarball images:

#### 8.1.1 Base System Updates

```bash
# Check for available updates
curl -s https://repo.regicideos.org/releases/Manifest.toml

# Manual update process
sudo mount -L ROOTS /mnt/roots
sudo wget https://repo.regicideos.org/releases/amd64/main/root.img
sudo umount /mnt/roots
sudo reboot
```

#### 8.1.2 Overlay Updates

Package overlays are updated using system package management tools:

```bash
# Update overlay packages
sudo dnf update
sudo emerge --sync @world

# Update specific package
sudo dnf update package-name

# Check for available updates
sudo dnf check-updates

# Clean up old packages
sudo dnf clean
sudo emerge --depclean
```

#### 8.1.3 Update Workflow

The installer automates base system updates:
1. Download newest `root.img` from RegicideOS repositories
2. Copy to ROOTS partition
3. Touch to ensure newest timestamp
4. Sync filesystem
5. Reboot

GRUB's `10_linux` helper automatically picks the newest `root-*.img` by modification time.

### 8.2 System Configuration

System configuration persists in overlay filesystems:

```bash
# Edit configuration (automatically goes to overlay)
sudo nano /etc/systemd/system.conf

# View overlay changes
sudo ls -la /overlay/etc

# View package status
sudo dnf list installed
sudo qlist -I  # if using Portage
```

### 8.3 System Snapshots

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

### 8.4 OS Personality Swapping

The phrase "OS personality" refers to ability to replace the read-only base system image (`root.img`) while keeping your local changes and home directory intact. This is one of the most powerful features of RegicideOS's architecture.

#### 8.4.1 Understanding OS Personalities

An "OS personality" is essentially the complete system environment contained in a compressed tarball (`root.img`). This includes:

- **Base system**: `/usr`, `/bin`, `/lib`, `/sbin`, and other core directories
- **Desktop environment**: Cosmic Desktop, GNOME, or other environments
- **System configuration**: Default settings and skeletons
- **Package set**: Pre-installed applications and libraries

Your personal data remains in separate locations:
- **Overlay**: Local modifications to `/etc`, `/var`, and `/usr`
- **Home directory**: User files in `/home`
- **Persistent configuration**: Custom settings and installed packages

#### 8.4.2 The Personality Swap Mechanism

Because RegicideOS's boot loader (GRUB) always picks the **newest** `root-*.img` by modification time, swapping personalities is as simple as copying a new image file to ROOTS partition.

#### 8.4.3 Step-by-Step Personality Swap

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

#### 8.4.4 Rolling Back Personalities

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

#### 8.4.5 Managing Multiple Personalities

You can keep multiple personalities available:

```bash
# List available personalities
ls -la /mnt/root-*.img

# Switch to a specific personality
sudo touch /mnt/root-desired-personality.img
reboot
```

#### 8.4.6 Deleting Obsolete Images

Clean up old personalities to save space:

```bash
# Remove old personality
sudo rm /mnt/root-old.img

# Free space immediately (BTRFS-specific)
sudo btrfs filesystem sync /mnt
```

### 8.5 Managing Multiple Personalities

You can create your own personalities:

```bash
# Create a custom personality from current system
# Create a custom overlay template from current system
sudo tar czf /mnt/root-custom.img -C /tmp/rootfs .

# Ensure it's newest for next boot
sudo touch /mnt/root-custom.img
```

---

## 9. Troubleshooting

### 9.1 Installation Issues

#### 9.1.1 Common Installation Problems

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

#### 9.1.2 LUKS-Specific Issues

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

### 9.2 Boot Issues

#### 9.2.1 GRUB Recovery

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

#### 9.2.2 Snapshot Recovery

Since current architecture doesn't support granular snapshots, rollback requires personality swapping (see Section 8.4).

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

### 9.3 Service Management

#### 9.3.1 System Services

```bash
# Manage core services
sudo systemctl status networking
sudo systemctl restart bluetooth

# View service logs
sudo journalctl -u systemd-networkd
```

#### 9.3.2 AI Agent Management

```bash
# Control AI agents
sudo systemctl enable portcl
sudo systemctl start portcl

# Monitor agent performance
portcl dashboard
btrmind monitor
```

### 9.4 System Health

```bash
# Check filesystem status
sudo btrfs filesystem df /

# Balance filesystem
sudo btrfs balance start /

# Scrub for errors
sudo btrfs scrub start /
```

### 9.5 Debug Information

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

## 10. FAQ

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

**A:** Foxmerge was described in early planning but was not implemented in favor of a simpler direct download model. The installer downloads compressed `root.img` tarballs from RegicideOS repositories, and post-installation package management is handled by standard system tools (dnf, emerge).

### Q7: How do I verify LUKS is working correctly?

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

## 11. References

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
- Xenia Linux base system
- Basic LUKS encryption support
- Basic installer functionality

---

*Document Version: 2.1*
*Last Updated: April 2026*
