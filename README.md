# 🏰 RegicideOS

<div align="center">
  
![RegicideOS Logo]()

**A Rust-first, AI-powered Linux distribution**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Made%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![AI](https://img.shields.io/badge/Powered%20by-AI-purple.svg)](https://example.com)

</div>

---

## 🌟 Overview

RegicideOS is a forward-looking Linux distribution that has begun as a fork of Xenia Linux with a clear vision: to create an **AI-first, Rust-powered operating system** designed for the future of computing.

Our mission is to build a secure, performant, and intelligent operating system where every component that can be implemented in Rust will be migrated to Rust, and AI capabilities are integrated at the system level.

---

## 🎯 Core Principles

### 🔧 Rust-First Philosophy
- **System-wide Rust adoption**: Every component that can be replaced with Rust binaries will be migrated
- **Memory safety**: Eliminate entire classes of vulnerabilities through Rust's ownership model
- **Performance**: Leverage Rust's zero-cost abstractions for optimal system performance

### 🤖 AI Integration
- **Intelligent system management**: AI-driven optimization and resource allocation
- **Predictive maintenance**: Proactive system health monitoring and issue resolution
- **User assistance**: Context-aware help and automation capabilities

### 🔮 Future-Proof Architecture
- **Kernel transition**: Preparing for migration to the [Asterinas kernel](https://asterinas.github.io/) as it matures
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

### Quick Install

```bash
# Clone the repository
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS

# Run the installer
sudo ./installer.py
```

---

## 🔮 Roadmap

### Phase 1: Foundation (Current)
- [x] Core installer functionality
- [x] BTRFS read-only system
- [ ] Cosmic Desktop integration
- [ ] Rust rewrite of installer
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
- [ ] Asterinas kernel integration
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
