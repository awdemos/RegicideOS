<div align="center">

# 🖥️ RegicideOS

### AI-Native · Rust-First · Immutable Linux Distribution

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://kernel.org/)
[![Btrfs](https://img.shields.io/badge/Btrfs-8db600?style=for-the-badge&logo=linux&logoColor=white)](https://btrfs.wiki.kernel.org/)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)](https://www.gnu.org/licenses/gpl-3.0)

**A forward-looking Linux distribution with a clear mission:** every component that can be implemented in Rust will be migrated to Rust, and AI capabilities are integrated at the system level.

[📥 Install](#installation) · [🏗️ Architecture](#architecture) · [🗺️ Roadmap](#roadmap) · [🤝 Contributing](#contributing)

</div>

---

## 🎯 Why RegicideOS?

> *"Regicide" refers to the "kings" of the current operating system marketplace: unsafe programming languages, bloated legacy stacks, and human-centric system administration.*

**The commits will keep coming until every single Red Hat Enterprise customer cancels their subscription.**

### Core Principles

- 🦀 **System-wide Rust adoption** — Replace C/C++ system components with memory-safe Rust binaries
- 🛡️ **Memory safety by default** — Eliminate entire classes of vulnerabilities through Rust's ownership model
- ⚡ **Zero-cost performance** — Leverage Rust's abstractions without runtime overhead
- 🤖 **AI-native from day one** — RL-driven optimization, predictive maintenance, and intelligent resource allocation
- 🔒 **Immutable foundation** — Read-only Btrfs root with atomic updates and instant rollback

---

## 🏗️ Architecture

| Component | Technology | Purpose |
|-----------|------------|---------|
| Kernel | Linux (→ [Asterinas](https://asterinas.github.io/)) | System foundation |
| Init System | systemd | Service management |
| Filesystem | Btrfs (read-only) | Immutable system image with overlay writes |
| Container Runtime | Distrobox | Application isolation and compatibility |
| Desktop Environment | [Cosmic Desktop](https://github.com/pop-os/cosmic-epoch) | GPU-accelerated, Wayland-native UI |
| Package Management | Overlays | Community-driven, curated software bundles |

### Directory Layout

```
/
├── boot/efi          # EFI System Partition
├── root/             # Read-only system image (squashfs)
│   ├── usr/          # System binaries
│   ├── etc/          # Base configuration
│   └── var/          # Variable data templates
├── home/             # User data (separate Btrfs subvolume)
└── overlay/          # Writable overlays
    ├── etc/          # Configuration overlay
    ├── var/          # Variable data overlay
    └── usr/          # User software overlay
```

### Key Design Decisions

- **Read-only root** — System files protected from accidental or malicious modification
- **Atomic updates** — Safe, transactional system updates via Btrfs snapshots
- **Rollback capability** — Boot into any previous system state instantly
- **Distrobox integration** — Seamless containerized application environment with full distro compatibility

---

## 📥 Installation

### Requirements

- 64-bit x86 processor
- 12GB disk space minimum (20GB recommended)
- UEFI or Legacy BIOS firmware
- Internet connection

### Quick Install (Recommended)

Boot any modern Linux live environment (Fedora Workstation recommended), then:

```bash
# Download the pre-built installer
curl -L -o regicide-installer \
  https://github.com/awdemos/RegicideOS/releases/latest/download/regicide-installer

# Make executable and run
chmod +x regicide-installer
sudo ./regicide-installer
```

### Manual Build (Advanced)

```bash
# Install build dependencies
sudo dnf -y install curl gcc btrfs-progs sgdisk rust cargo

# Clone and build
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS/installer
cargo build --release
sudo ./target/release/installer
```

### Automated Configuration

```bash
cat > regicide-config.toml << 'EOF'
drive = "/dev/sda"
repository = "https://repo.xenialinux.com/releases/"
flavour = "minimal"
release_branch = "main"
filesystem = "btrfs"
username = "your-username"
applications = "minimal"
EOF

sudo ./regicide-installer -c regicide-config.toml
```

---

## 🤖 AI Integration Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| Now | BtrMind local AI assistant | 🔄 In Progress |
| 2026 | Predictive system maintenance | 📋 Planned |
| 2026 | Intelligent resource allocation | 📋 Planned |
| 2027 | Natural language system control | 📋 Planned |
| 2027 | Asterinas kernel migration | 📋 Planned |

---

## 🗺️ Roadmap

- [x] Core installer with Btrfs validation
- [x] Read-only root filesystem
- [x] Rust rewrite of installer
- [ ] Cosmic Desktop integration
- [ ] Rust replacements of core GNU utilities
- [ ] Memory-safe package manager
- [ ] Advanced AI capabilities (predictive maintenance, NL control)
- [ ] Asterinas kernel integration

---

## 🤝 Contributing

We particularly need help with:

- 🦀 **Rust development** — Rewriting system components in Rust
- 🤖 **AI integration** — Implementing intelligent system features
- 📦 **Overlay creation** — Developing useful package collections
- 📝 **Documentation** — Improving guides and references
- 🧪 **Testing** — Bug reports and verification

---

## 📜 License

RegicideOS is licensed under the **GNU General Public License v3.0**.

Built on the excellent foundation of [Xenia Linux](https://xenialinux.com/).

---

<div align="center">

**© 2026 Andrew White · RegicideOS Project**

</div>
