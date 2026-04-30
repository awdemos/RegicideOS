# RegicideOS Rust Overlay

A Gentoo overlay focused on providing cutting-edge Rust toolchains, applications, and libraries optimized for embedded and AI workloads for RegicideOS.

## Overview

This overlay provides:
- Latest stable Rust toolchains
- Cross-compilation targets for embedded systems
- AI/ML libraries and tools
- System utilities rewritten in Rust
- Performance-optimized Rust packages
- Deep integration with RegicideOS architecture

## Installation

### 1. Enable Overlays

Add overlay configuration to your Portage setup:

```bash
# /etc/portage/repos.conf/guru.conf
[guru]
location = /var/db/repos/guru
sync-type = git
sync-uri = https://anongit.gentoo.org/git/repo/proj/guru.git
priority = 20

# /etc/portage/repos.conf/regicide.conf
[regicide-rust]
location = /var/db/repos/regicide-rust
sync-type = git
sync-uri = https://github.com/awdemos/regicide-rust-overlay.git
priority = 10
```

### 2. Sync Overlays

```bash
# Enable repositories
eselect repository enable guru regicide-rust

# Sync all overlays
emaint sync -a
```

### 3. Install Packages

#### Standard Rust Installation
```bash
# Use rustup from GURU (recommended for development)
emerge dev-util/rustup
rustup install stable
```

#### RegicideOS Enhanced Rust
```bash
# Install Rust with embedded targets
echo "dev-lang/rust embedded_targets" >> /etc/portage/package.use/rust
emerge dev-lang/rust

# Verify embedded targets
rustc --print target-list | grep -E "(thumbv|riscv)"
```

#### AI System Tools
```bash
# Install BtrMind storage AI
echo "app-misc/regicide-ai-tools btrmind" >> /etc/portage/package.use/regicide
emerge regicide-ai-tools

# Enable and start services
systemctl enable btrmind
systemctl start btrmind
```

## Package Categories

### Priority System
1. **regicide-rust** (Priority 10) - RegicideOS customizations
2. **guru** (Priority 20) - Community packages
3. **gentoo** (Priority 50) - Main tree

### Package Categories

#### dev-rust/*
Rust toolchain packages:
- `rust` - Latest stable Rust
- `rust-nightly` - Nightly Rust toolchain
- `rust-cross-*` - Cross-compilation toolchains
- `rust-analyzer` - LSP for Rust

#### sys-apps/rust-*
Rust system utilities:
- `rust-utils` - Core utilities (ls, cat, grep replacements)
- `rust-monitor` - System monitoring tools
- `rust-security` - Security-focused utilities

#### sci-libs/rust-*
Rust AI/ML libraries:
- `candle-rs` - ML inference engine
- `tch-rs` - PyTorch bindings
- `ndarray-rs` - Scientific computing
- `polars-rs` - Data processing

#### embedded-rust/*
Embedded development tools:
- `rust-embedded` - Embedded Rust toolchain
- `rust-baremetal` - Bare-metal targets
- `rust-rtos` - Real-time OS support

#### app-misc/regicide-ai-tools
- **Source**: Regicide overlay (exclusive)
- **Components**: BtrMind, PortCL (future), system integration
- **USE flags**: `btrmind`, `portcl`, `systemd`

## Embedded Development

### Supported Targets

#### ARM Cortex-M
```bash
# Install ARM toolchain
emerge cross-arm-none-eabi/gcc

# Available targets
thumbv6m-none-eabi      # Cortex-M0, M0+
thumbv7m-none-eabi      # Cortex-M3  
thumbv7em-none-eabi     # Cortex-M4, M7 (no FPU)
thumbv7em-none-eabihf   # Cortex-M4F, M7F (with FPU)
thumbv8m.base-none-eabi # Cortex-M23
thumbv8m.main-none-eabi # Cortex-M33, M35P
```

#### RISC-V
```bash
# Install RISC-V toolchain
emerge cross-riscv32-unknown-elf/gcc

# Available targets
riscv32i-unknown-none-elf     # RV32I base
riscv32imc-unknown-none-elf   # RV32IMC (compressed)
riscv32imac-unknown-none-elf  # RV32IMAC (atomic)
riscv64gc-unknown-none-elf    # RV64GC full
```

#### WebAssembly
```bash
# WebAssembly targets (no additional toolchain needed)
wasm32-unknown-unknown   # Pure WebAssembly
wasm32-wasi             # WASI (WebAssembly System Interface)
```

### Cross-Compilation Helper

```bash
# Use RegicideOS cross-compilation helper
regicide-cross-compile --target thumbv7em-none-eabi

# With additional cargo arguments
regicide-cross-compile --target riscv32imc-unknown-none-elf -- --release

# List available targets
regicide-cross-compile --list-targets
```

## AI Development

### BtrMind - Storage Monitoring AI

```bash
# Install and configure
emerge regicide-ai-tools
systemctl enable btrmind

# Configuration
nano /etc/btrmind/config.toml

# Manual operations
btrmind analyze              # Check current state
btrmind cleanup --aggressive # Force cleanup
btrmind stats               # Learning statistics
```

### Custom AI Agent Development

```rust
// Create new agent using RegicideOS framework
use regicide_ai::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let agent = MyAgent::new()?;
    agent.run().await
}
```

## Maintenance

### Overlay Updates

```bash
# Sync overlays
emaint sync -r regicide-rust
emaint sync -r guru

# Update packages
emerge --update --deep @world

# Check for overlay conflicts
equery list -o | grep regicide-rust
```

### Package Development

#### Adding New Packages

1. **Fork this repository**
2. **Create ebuild in appropriate category**:
   ```bash
   mkdir -p category/package
   cp template.ebuild category/package/package-version.ebuild
   ```
3. **Test locally**:
   ```bash
   ebuild package-version.ebuild manifest
   emerge package
   ```
4. **Submit pull request**

#### Testing Changes

```bash
# Test overlay integrity
repoman scan

# Test package installation
emerge --pretend --verbose package

# Test package removal
emerge --unmerge package
```

## Integration with RegicideOS

### System Image Integration

The overlay is integrated into RegicideOS system images:

```bash
# In RegicideOS build system
emerge --root=/mnt/regicide regicide-ai-tools
systemctl --root=/mnt/regicide enable btrmind
```

### Container Support

AI agents work with RegicideOS's container-first approach:

```bash
# Install tools in Distrobox container
distrobox create --name rust-dev --image fedora:39
distrobox enter rust-dev
# Install Rust toolchain normally
```

## Troubleshooting

### Common Issues

**Overlay conflicts**:
```bash
# Check package sources
equery which dev-lang/rust
# Should show regicide-rust overlay

# Force repository priority
echo "dev-lang/rust::regicide-rust" >> /etc/portage/package.use/priorities
```

**Missing embedded toolchains**:
```bash
# Install cross-compilation toolchains
emerge cross-arm-none-eabi/gcc
emerge cross-riscv32-unknown-elf/gcc
```

**AI service issues**:
```bash
# Check service status
systemctl status btrmind

# View logs
journalctl -u btrmind -f

# Reset AI learning
systemctl stop btrmind
rm /var/lib/btrmind/model.json
systemctl start btrmind
```

### Support Channels

- **Issues**: [GitHub Issues](https://github.com/awdemos/RegicideOS/issues)
- **Discussions**: [GitHub Discussions](https://github.com/awdemos/RegicideOS/discussions)
- **Documentation**: [RegicideOS Handbook](../../Handbook.md)

## Contributing

### Development Workflow

1. **Set up development environment**:
   ```bash
   git clone https://github.com/awdemos/regicide-rust-overlay.git
   cd regicide-rust-overlay
   ```

2. **Make changes**:
   - Add/modify ebuilds
   - Update package.use defaults
   - Test changes locally

3. **Validation**:
   ```bash
   # Check ebuild syntax
   repoman scan

   # Test package builds
   ebuild package.ebuild manifest
   emerge package
   ```

4. **Submit pull request**

### Code Style

- **Ebuild format**: Follow Gentoo ebuild standards
- **Variable naming**: Use clear, descriptive names
- **Comments**: Document RegicideOS-specific modifications
- **Testing**: Include test cases for new functionality

## Roadmap

### Current Status ✅
- [x] Basic overlay structure
- [x] Rust toolchain with embedded targets
- [x] BtrMind AI agent integration
- [x] GURU overlay compatibility

### Near-term (1-2 months)
- [ ] PortCL package management AI
- [ ] Additional embedded toolchains (AVR, Xtensa)
- [ ] AI model sharing infrastructure
- [ ] Performance optimization packages

### Long-term (3-6 months)  
- [ ] Custom kernel integration packages
- [ ] Advanced AI development tools
- [ ] Multi-agent coordination packages
- [ ] Distributed system components

## License

GPL-3.0 - See [LICENSE](../../LICENSE) for details.

---

**regicide-rust** - Powering the Rust-first, AI-driven future of RegicideOS 🦀🤖
