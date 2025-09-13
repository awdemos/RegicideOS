# 🏰 RegicideOS

<div align="center">
  
![RegicideOS Logo](regicideos_poster.png)

**A Rust-first, AI-powered Linux distribution**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Made%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![AI](https://img.shields.io/badge/Powered%20by-AI-purple.svg)](https://example.com)

</div>

---

## 🌟 Overview

RegicideOS is a forward-looking Linux distribution that has begun as a fork of Xenia Linux with a clear vision: to create an **AI-first, Rust-powered operating system** designed for the future of computing.

Our mission is to build a secure, performant, and intelligent operating system where every component that can be implemented in Rust will be migrated to Rust, and AI capabilities are integrated at the system level.

`Regicide` in this case refers strictly to the `"kings"` of the current operating system marketplace: Microsoft, unsafe programming languages, and human-centric system administration, and has no further social or political ramifications
implied.

Motivational slogan:

> "The commits will keep coming until every single Red Hat Enterprise customer cancels their subscription."

---

## 🎯 Core Principles

### 🔧 Rust-First Philosophy
- **System-wide Rust adoption**: Every component that can be replaced with Rust binaries will be migrated
- **Memory safety**: Eliminate entire classes of vulnerabilities through Rust's ownership model
- **Performance**: Leverage Rust's zero-cost abstractions for optimal system performance

### 🤖 AI Integration
- **Intelligent system management**: AI-driven optimization and resource allocation utilizing Reenforcement Learning
- **Predictive maintenance**: Proactive system health monitoring and issue resolution 
- **User assistance**: Context-aware help and automation capabilities
- **Continual Reinforcement Learning**: Inspired by the techniques listed in [A Survey of Continual Reinforcement Learning
](https://arxiv.org/abs/2506.21872)

### 🔮 Future-Proof Architecture
- **Kernel transition**: Readiness for migration to the [Asterinas kernel](https://asterinas.github.io/) as it matures
- **Immutable system**: Read-only BTRFS filesystem for enhanced security and stability
- **Container-native**: Built around containerized workflows for application isolation

---

## 🏗️ System Architecture

### Core Components

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Kernel** | Linux (→ Asterinas) | System foundation |
| **Init System** | systemd | Service management |
| **Filesystem** | BTRFS (read-only) | Immutable system image |
| **Container Runtime** | Distrobox | Application isolation |
| **Desktop Environment** | Cosmic Desktop | User interface |
| **Package Management** | Overlays | Software distribution |

### Filesystem Structure

```
/
├── boot/efi          # EFI System Partition
├── root/             # Read-only system image (squashfs)
│   ├── usr/          # System binaries
│   ├── etc/          # Configuration (overlay)
│   └── var/          # Variable data (overlay)
├── home/             # User data (separate subvolume)
└── overlay/          # Writable overlays
    ├── etc/          # Configuration overlay
    ├── var/          # Variable data overlay
    └── usr/          # User software overlay
```

---

## 🚀 Key Features

### ✨ Immutable System
- **Read-only root**: System files protected from accidental modification
- **Atomic updates**: Safe, transactional system updates
- **Rollback capability**: Easy system state restoration

### 📦 Container-First Workflow
- **Distrobox integration**: Seamless containerized application environment
- **Isolated workspaces**: Clean separation between system and user applications
- **Compatibility layer**: Run applications from any Linux distribution

### 🎨 Cosmic Desktop
- **Modern interface**: Built with Iced for a native Rust experience
- **Wayland native**: Next-generation display protocol
- **GPU-accelerated**: Hardware-accelerated graphics pipeline

### 🧩 Overlay System
- **Community-driven**: User-submitted package collections
- **Curated sets**: Pre-configured application bundles for specific workflows
- **Easy sharing**: Simple format for distributing software collections

---

## 📦 Installation

### Prerequisites
- 64-bit x86 processor
- 12GB disk space minimum (20GB recommended)
- UEFI or Legacy BIOS firmware
- Internet connection

### Installation Steps

**Step 1: Boot Live Environment**

**⚠️ IMPORTANT**: You must boot into a Linux live CD/USB environment to install RegicideOS. The minimal installer requires Rust toolchain to be available.

**Recommended Live Environments:**
- **Fedora Workstation Live**: https://getfedora.org/en/workstation/download/
- **Ubuntu Live**: https://ubuntu.com/download/desktop
- **Arch Linux Live**: https://archlinux.org/download/

Boot your target machine from the live environment and connect to the internet.

**Step 2: Quick Install (Recommended)**

For the fastest installation, use our pre-built minimal installer binary:

```bash
# Download the pre-built installer
curl -L -o regicide-installer https://github.com/awdemos/RegicideOS/releases/latest/download/regicide-installer

# Make executable and run
chmod +x regicide-installer
sudo ./regicide-installer
```

**Step 3: Manual Install (Advanced)**

If you need to build from source or use a custom configuration:

```bash
# Install required packages in live environment
# For Fedora:
sudo dnf install git curl gcc btrfs-progs

# For Ubuntu/Debian:
sudo apt update && sudo apt install git curl gcc btrfs-progs

# For Arch:
sudo pacman -S git curl gcc btrfs-progs

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS/installer
cargo build --release
sudo ./target/release/installer
```

### Installation Configuration

The installer defaults to minimal packages only (no recommended packages) and BTRFS filesystem. Common configurations:

#### Interactive Mode (Default)
```bash
sudo ./regicide-installer
# or for manual build:
sudo ./target/release/installer
```
The installer will guide you through:
- Disk selection and partitioning
- Username and password setup
- Minimal package installation (no recommended packages)
- BTRFS subvolume configuration

#### Automated Installation
```bash
# Create minimal configuration
cat > regicide-config.toml << EOF
drive = "/dev/sda"
repository = "https://repo.xenialinux.com/releases/"
flavour = "minimal"  # Uses minimal packages only
release_branch = "main"
filesystem = "btrfs"
username = "your-username"
applications = "minimal"  # IMPORTANT: No recommended packages
EOF

# Run automated installation
sudo ./regicide-installer -c regicide-config.toml
```

#### Live Environment Requirements
- **Live OS**: Any modern Linux live environment (Fedora recommended)
- **Storage**: Target drive will be completely erased and reformatted
- **Network**: Required for downloading system image from Xenia repository
- **Time**: 15-30 minutes depending on internet speed

### Post-Installation

After installation completes:

1. **Reboot** into your new RegicideOS system
2. **Login** with your created username and password
3. **Verify Installation**:
   ```bash
   # Check system status
   systemctl status
   cat /etc/os-release

   # Verify BTRFS setup
   sudo btrfs filesystem df /
   sudo btrfs subvolume list /

   # Check BtrMind service (if AI tools installed)
   systemctl status btrmind
   ```

### Troubleshooting

#### Rust Toolchain Issues
If the pre-built installer fails, ensure Rust is properly installed:
```bash
# Verify Rust installation
rustc --version
cargo --version

# If not installed:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### BTRFS Creation Issues
The installer includes comprehensive BTRFS validation:
- Subvolume structure verification
- Proper mount point configuration
- Compression and optimization settings
- Read-only root with writable overlays

#### Network Issues
If downloading from Xenia repository fails:
```bash
# Test connectivity
ping repo.xenialinux.com

# Use alternative mirror if needed
# In config file, set: repository = "https://mirror.xenialinux.com/releases/"
```

---

## 🔮 Roadmap

### Phase 1: Foundation (Current)
- [x] Core installer functionality
- [x] BTRFS read-only system
- [x] Rust rewrite of installer
- [ ] Cosmic Desktop integration
- [ ] Rust replacements of core utilities
- [ ] Basic local-only Rust-based AI integrations

### Phase 2: Rust Migration
- [ ] Rust-based system utilities
- [ ] Memory-safe package manager
- [ ] Rust-based system components

### Phase 3: AI Integration
- [ ] Predictive system maintenance
- [ ] Intelligent resource allocation
- [ ] Context-aware user assistance
- [ ] Natural language system control

### Phase 4: Future Architecture
- [ ] Asterinas kernel integration. Probably in 2026-2027.
- [ ] Complete Rust system stack
- [ ] Advanced AI capabilities
- [ ] Distributed system features

---

## 🤝 Contributing

We welcome contributions to RegicideOS! Areas where we particularly need help:

- **Rust development**: Rewriting system components in Rust
- **AI integration**: Implementing intelligent system features
- **Overlay creation**: Developing useful package collections
- **Documentation**: Improving guides and references
- **Testing**: Bug reports and verification

See our [Contribution Guidelines](CONTRIBUTING.md) for details.

---

## 📊 Comparison with Xenia Linux

| Feature | Xenia Linux | RegicideOS |
|---------|-------------|------------|
| **Primary Language** | Mixed | Rust-first |
| **AI Integration** | Limited | Core focus |
| **Kernel** | Linux | Linux → Asterinas |
| **Filesystem** | Multiple options | BTRFS (read-only) |
| **Desktop Environments** | Multiple | Cosmic Desktop only |
| **Package Management** | Traditional | Overlay-based |
| **System Philosophy** | General purpose | AI/Rust-focused |
| **Update Model** | Traditional | Immutable |

---

## 📄 License

RegicideOS is licensed under the GNU General Public License v3.0. See the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- **Xenia Linux**: For providing the excellent foundation upon which RegicideOS is built
- **Rust Foundation**: For creating the language that powers our vision
- **System76**: For the Cosmic Desktop environment
- **Asterinas Team**: For their groundbreaking kernel research
- **Container Community**: For the tools that make our container-first approach possible

---

<div align="center">

**Join us in building the future of operating systems!**

[🌐 Website]() • [💬 Discord]() • [🐙 GitHub](https://github.com/awdemos/RegicideOS)

</div>
