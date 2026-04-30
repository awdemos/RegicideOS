# RegicideOS Rust Overlay Installation Guide

This guide covers the installation and configuration of the RegicideOS Rust overlay.

## Quick Start

### 1. Prerequisites

Ensure you have a working Gentoo or RegicideOS system with:
- Portage package manager
- Internet connection
- Sufficient disk space (~500MB for overlay + packages)

### 2. Install the Overlay

#### Method 1: Using Layman (Recommended)
```bash
# Install layman if not already installed
sudo emerge --ask app-portage/layman

# Add the overlay
sudo layman -f -o https://raw.githubusercontent.com/awdemos/regicide-rust-overlay/master/overlay.xml -a regicide-rust

# Sync the overlay
sudo layman -S
```

#### Method 2: Manual Installation
```bash
# Create overlay directory
sudo mkdir -p /var/db/repos/regicide-rust

# Clone the overlay
sudo git clone https://github.com/awdemos/regicide-rust-overlay.git /var/db/repos/regicide-rust

# Add to repositories configuration
cat << 'EOF' | sudo tee /etc/portage/repos.conf/regicide-rust.conf
[regicide-rust]
location = /var/db/repos/regicide-rust
sync-type = git
sync-uri = https://github.com/awdemos/regicide-rust-overlay.git
priority = 10
auto-sync = yes
EOF

# Sync the overlay
sudo emaint sync -r regicide-rust
```

### 3. Enable GURU Overlay (Required Dependencies)

```bash
# Add GURU overlay
sudo layman -a guru

# Or manually if needed
cat << 'EOF' | sudo tee /etc/portage/repos.conf/guru.conf
[guru]
location = /var/db/repos/guru
sync-type = git
sync-uri = https://anongit.gentoo.org/git/repo/proj/guru.git
priority = 20
auto-sync = yes
EOF
```

### 4. Install Package Sets

#### Basic Rust Development
```bash
# Install base Rust toolchain
sudo emerge @regicide-rust-base
```

#### Embedded Development
```bash
# Install embedded development tools
sudo emerge @regicide-rust-embedded
```

#### AI/ML Development
```bash
# Install AI development tools
sudo emerge @regicide-rust-ai
```

#### Complete Installation
```bash
# Install everything
sudo emerge @regicide-rust-base @regicide-rust-embedded @regicide-rust-ai
```

## Configuration

### 1. Set Package Keywords

Add the following to `/etc/portage/package.accept_keywords/regicide-rust`:

```bash
# RegicideOS Rust Overlay
~amd64 ::regicide-rust
```

### 2. Configure USE Flags

Add to `/etc/portage/package.use/regicide-rust`:

```bash
# Enable embedded targets in Rust
dev-lang/rust embedded_targets

# Enable AI tools
app-misc/regicide-ai-tools btrmind systemd

# Enable hardware acceleration for ML
sci-libs/candle-rs cuda metal vulkan
```

### 3. Cross-Compilation Setup

#### ARM Cortex-M Development
```bash
# Install ARM toolchain
sudo emerge cross-arm-none-eabi/gcc

# Verify Rust targets
rustc --print target-list | grep thumbv

# Add specific target
rustup target add thumbv7em-none-eabihf
```

#### RISC-V Development
```bash
# Install RISC-V toolchain
sudo emerge cross-riscv32-unknown-elf/gcc
sudo emerge cross-riscv64-unknown-elf/gcc

# Add targets
rustup target add riscv32imc-unknown-none-elf
rustup target add riscv64gc-unknown-none-elf
```

## Usage Examples

### 1. Rust Project Setup

```bash
# Create new Rust project
cargo new my_project
cd my_project

# Add dependencies to Cargo.toml
[dependencies]
candle-core = "0.6"
candle-nn = "0.6"
```

### 2. Cross-Compilation

```bash
# Using RegicideOS cross-compilation helper
regicide-cross-compile --target thumbv7em-none-eabihf

# Traditional method
cargo build --target thumbv7em-none-eabihf
```

### 3. AI/ML Development

```bash
# Create ML project
cargo new ml_project --bin
cd ml_project

# Add ML dependencies
echo '[dependencies]' >> Cargo.toml
echo 'candle-core = "0.6"' >> Cargo.toml
echo 'candle-nn = "0.6"' >> Cargo.toml

# Build with CUDA support
RUSTFLAGS="-C target-cuda=native" cargo build --release
```

### 4. System Utility Replacement

```bash
# Install Rust system utilities
sudo emerge sys-apps/rust-utils

# Verify symlinks
ls -l /usr/bin/ls  # Should point to exa
ls -l /usr/bin/cat # Should point to bat
ls -l /usr/bin/grep # Should point to rg
```

## BtrMind AI Agent Setup

### 1. Installation
```bash
# Install BtrMind
sudo emerge app-misc/regicide-ai-tools
```

### 2. Configuration
```bash
# Copy example configuration
sudo cp /etc/btrmind/config.toml.example /etc/btrmind/config.toml

# Edit configuration
sudo nano /etc/btrmind/config.toml
```

### 3. Service Management
```bash
# Enable and start the service
sudo systemctl enable btrmind
sudo systemctl start btrmind

# Check status
sudo systemctl status btrmind

# View logs
sudo journalctl -u btrmind -f
```

### 4. Manual Operations
```bash
# Analyze current storage state
sudo btrmind analyze

# Run cleanup operations
sudo btrmind cleanup

# View learning statistics
sudo btrmind stats

# Force aggressive cleanup
sudo btrmind cleanup --aggressive
```

## Troubleshooting

### 1. Overlay Conflicts

If you encounter package conflicts:
```bash
# Check package sources
equery which dev-lang/rust

# Force repository priority
echo "dev-lang/rust::regicide-rust" >> /etc/portage/package.use/priorities

# Clean up package database
sudo emerge --regen
```

### 2. Missing Dependencies

```bash
# Install missing cross-compilation toolchains
sudo emerge cross-arm-none-eabi/gcc
sudo emerge cross-riscv32-unknown-elf/gcc

# Add Rust targets manually
rustup target add thumbv7em-none-eabihf
rustup target add riscv32imc-unknown-none-elf
```

### 3. Build Failures

```bash
# Clean build
sudo emerge --clean app-misc/regicide-ai-tools

# Check Rust installation
rustc --version
cargo --version

# Verify toolchain
rustc --print target-list
```

### 4. Service Issues

```bash
# Check BtrMind service status
sudo systemctl status btrmind

# Restart service
sudo systemctl restart btrmind

# Reset AI learning model
sudo systemctl stop btrmind
sudo rm /var/lib/btrmind/model.json
sudo systemctl start btrmind
```

## Maintenance

### 1. Regular Updates

```bash
# Sync overlays
sudo emaint sync -a

# Update packages
sudo emerge --update --deep @world

# Clean up old packages
sudo emerge --depclean
```

### 2. Overlay Updates

```bash
# Update RegicideOS overlay specifically
sudo emaint sync -r regicide-rust

# Check for broken packages
sudo emerge -uDv --keep-going @world
```

### 3. Performance Monitoring

```bash
# Check Rust utilities performance
hyperfine 'exa -la' 'ls -la'

# Monitor system resources
btm

# Check disk usage
dust
```

## Contributing

1. Fork the repository on GitHub
2. Create your feature branch
3. Follow Gentoo ebuild conventions
4. Test your changes locally
5. Submit a pull request

### Development Workflow

```bash
# Clone overlay repository
git clone https://github.com/awdemos/regicide-rust-overlay.git
cd regicide-rust-overlay

# Create new branch
git checkout -b my-feature

# Add/modify ebuilds
# Test locally with:
ebuild package-version.ebuild manifest
emerge package

# Commit and push
git add .
git commit -m "Add my feature"
git push origin my-feature
```

## Support

- **Issues**: [GitHub Issues](https://github.com/awdemos/RegicideOS/issues)
- **Documentation**: [RegicideOS Handbook](../../Handbook.md)
- **Community**: [GitHub Discussions](https://github.com/awdemos/RegicideOS/discussions)

---

**Next Steps**: After installing the overlay, explore the [Rust Development Guide](RUST_DEVELOPMENT.md) for advanced usage patterns and examples.