# RegicideOS Rust Overlay

A Gentoo overlay providing enhanced Rust support, AI-powered system tools, and embedded development capabilities for RegicideOS.

## Overview

This overlay provides:
- Enhanced Rust toolchains with embedded and AI optimization
- AI-powered system management tools (BtrMind, PortCL)
- Modern Rust system utilities
- Cross-compilation support for embedded development
- Performance-optimized packages

## Installation

### Quick Start
```bash
# Add the overlay
eselect repository enable regicide-rust

# Sync overlays
emaint sync -a

# Install base Rust tools
emerge @regicide-rust-base
```

### Detailed Installation
See [INSTALLATION.md](INSTALLATION.md) for complete setup instructions.

## Package Categories

### dev-rust/
- `rust` - Enhanced Rust toolchain with embedded targets
- `rust-analyzer` - LSP for Rust development
- `rust-cross-*` - Cross-compilation toolchains

### sys-apps/rust-utils
- `exa` - Modern `ls` replacement
- `bat` - Modern `cat` with syntax highlighting
- `ripgrep` - Fast `grep` replacement
- `fd` - Fast `find` replacement
- `dust` - Modern `du` with tree view
- `procs` - Modern `ps` and `top` replacement
- `bottom` - System monitor

### sci-libs/rust-ml
- `candle-rs` - ML inference engine
- `tch-rs` - PyTorch bindings
- `ndarray-rs` - Scientific computing
- `polars-rs` - Data processing

### app-misc/regicide-ai-tools
- `btrmind` - AI-powered BTRFS optimization
- `portcl` - AI package management (future)

### embedded-rust/
- Cross-compilation toolchains for ARM Cortex-M and RISC-V
- Embedded development tools and libraries

## Usage

### Cross-Compilation
```bash
# List available targets
regicide-cross-compile --list-targets

# Build for embedded target
regicide-cross-compile --target thumbv7em-none-eabihf
```

### AI Tools
```bash
# Install BtrMind
emerge app-misc/regicide-ai-tools

# Start AI storage optimization
systemctl enable btrmind
systemctl start btrmind

# Analyze storage
btrmind analyze
```

### Rust Utilities
```bash
# Install modern utilities
emerge sys-apps/rust-utils

# Use enhanced commands
exa -la      # instead of ls -la
bat file.txt  # instead of cat file.txt
rg pattern    # instead of grep pattern
```

## Package Sets

- `@regicide-rust-base` - Core Rust development tools
- `@regicide-rust-embedded` - Embedded development tools
- `@regicide-rust-ai` - AI/ML development tools

## Repository Structure

```
├── dev-rust/              # Rust toolchain packages
├── sys-apps/rust-utils   # Modern system utilities
├── sci-libs/rust-ml      # AI/ML libraries
├── app-misc/regicide-ai-tools # AI system tools
├── embedded-rust/         # Embedded development tools
├── profiles/              # Profile configurations
├── metadata/              # Overlay metadata
├── sets/                  # Package sets
├── files/                 # Support files
└── doc/                   # Documentation
```

## Contributing

1. Fork the repository
2. Create your feature branch
3. Follow Gentoo ebuild conventions
4. Test your changes locally
5. Submit a pull request

### Development Workflow
```bash
# Clone the overlay
git clone https://github.com/regicideos/regicide-rust-overlay.git
cd regicide-rust-overlay

# Create new ebuild
mkdir -p category/package
cp template.ebuild category/package/package-version.ebuild

# Test locally
ebuild package-version.ebuild manifest
emerge package
```

## License

GPL-3.0 - See [LICENSE](LICENSE) for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/regicideos/RegicideOS/issues)
- **Documentation**: [RegicideOS Handbook](https://docs.regicideos.com)
- **Community**: [Discord Server](https://discord.gg/regicideos)

---

**Built for the future of operating systems with Rust and AI** 🦀🤖