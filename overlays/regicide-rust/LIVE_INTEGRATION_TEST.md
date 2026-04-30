# RegicideOS Overlay Live Integration Test Guide

This guide provides comprehensive testing procedures for validating the Regicide-Rust overlay integration with Xenia Linux infrastructure.

## Prerequisites

### System Requirements
- Xenia Linux installation (or compatible Gentoo-based system)
- Internet connection for package downloads
- Sufficient disk space for Rust toolchain (~10GB)
- Python 3.10+ for TOML validation

### Tools Required
```bash
# Gentoo package management tools
emerge eselect repoman ebuild

# Rust toolchain (will be installed by overlay)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Development tools
git wget tar
```

## Integration Testing Steps

### 1. Overlay Installation

#### Method A: Manual Installation
```bash
# Create overlay directory
sudo mkdir -p /var/db/repos/regicide-rust

# Clone the overlay
sudo git clone https://github.com/awdemos/regicide-rust-overlay /var/db/repos/regicide-rust

# Add overlay to portage
echo "regicide-rust /var/db/repos/regicide-rust" | sudo tee -a /etc/portage/repos.conf/regicide-rust.conf

# Update repository cache
sudo emaint sync -r regicide-rust
```

#### Method B: Using Layman
```bash
# Install layman if not available
sudo emerge layman

# Add overlay
sudo layman -o https://raw.githubusercontent.com/awdemos/regicide-rust-overlay/main/repositories.xml -f -a regicide-rust

# Sync overlays
sudo layman -S
```

### 2. Validation Tests

#### Run Automated Tests
```bash
# Change to overlay directory
cd /var/db/repos/regicide-rust

# Run overlay validation
sudo ./test-overlay.sh

# Run AI package validation
sudo ./validate-ai-packages.sh
```

#### Expected Test Results
- All overlay structure tests should pass
- Ebuild syntax validation should succeed
- Toolchain configuration should be valid
- AI package sets should be properly defined

### 3. Package Installation Tests

#### Test Base Rust Toolchain
```bash
# Install base Rust toolchain
sudo emerge -av @regicide-rust-base

# Verify installation
rustc --version
cargo --version

# Test cross-compilation helper
regicide-cross-compile --list-targets
```

#### Test AI Packages
```bash
# Install AI package set
sudo emerge -av @regicide-rust-ai

# Test Candle ML installation
if command -v candle-examples >/dev/null 2>&1; then
    echo "✅ Candle ML installed successfully"
else
    echo "❌ Candle ML installation failed"
fi
```

#### Test Embedded Development
```bash
# Install embedded package set
sudo emerge -av @regicide-rust-embedded

# Test cross-compilation to embedded target
rustc --target thumbv7em-none-eabihf --version || echo "Target not installed"

# Install cross-toolchain if needed
sudo emerge cross-arm-none-eabi/newlib
```

### 4. Cross-compilation Validation

#### Test Embedded Targets
```bash
# Create test embedded project
mkdir -p /tmp/embedded-test
cd /tmp/embedded-test

cargo new embedded_blinky --bin
cd embedded_blinky

# Add embedded target to Cargo.toml
cat >> Cargo.toml << EOF

[target.thumbv7em-none-eabihf]
runner = "qemu-system-arm -machine lm3s6965evb -nographic -semihosting-config enable=on,target=native"

[dependencies]
cortex-m = "0.7"
cortex-m-rt = "0.7"
panic-halt = "0.2"
EOF

# Create simple embedded program
cat > src/main.rs << 'EOF'
#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    loop {
        // Simple embedded program
    }
}
EOF

# Build for embedded target
cargo build --target thumbv7em-none-eabihf

# Verify binary was created
ls -la target/thumbv7em-none-eabihf/debug/embedded_blinky
```

#### Test WebAssembly AI
```bash
# Create WebAssembly test project
mkdir -p /tmp/wasm-test
cd /tmp/wasm-test

cargo new wasm_inference --bin
cd wasm_inference

# Add WebAssembly dependencies
cat >> Cargo.toml << 'EOF'

[dependencies]
candle-core = "0.6"
candle-nn = "0.6"
wasm-bindgen = "0.2"
EOF

# Create simple inference example
cat > src/main.rs << 'EOF'
use candle_core::{Device, Tensor, Result};

fn main() -> Result<()> {
    let device = Device::Cpu;

    // Create a simple tensor
    let tensor = Tensor::randn(0f32, 1.0, (2, 3), &device)?;
    println!("Tensor shape: {:?}", tensor.shape());
    println!("Tensor values: {}", tensor);

    Ok(())
}
EOF

# Build for WebAssembly
cargo build --target wasm32-unknown-unknown --release

# Verify WASM output
ls -la target/wasm32-unknown-unknown/release/wasm_inference.wasm
```

### 5. Performance Testing

#### Rust Toolchain Performance
```bash
# Test compilation speed
mkdir -p /tmp/perf-test
cd /tmp/perf-test

cargo new perf_test --bin
cd perf_test

# Add computational workload
cat >> src/main.rs << 'EOF'
use std::time::Instant;

fn main() {
    let start = Instant::now();

    // Computational workload
    let mut sum = 0u64;
    for i in 0..10_000_000u64 {
        sum += i * i;
    }

    let duration = start.elapsed();
    println!("Computation completed in {:?}", duration);
    println!("Sum: {}", sum);
}
EOF

# Time the compilation
time cargo build --release

# Time the execution
time ./target/release/perf_test
```

#### AI Inference Performance
```bash
# Test AI inference if Candle is available
if command -v candle-examples >/dev/null 2>&1; then
    echo "Testing AI inference performance..."

    # Create simple neural network test
    mkdir -p /tmp/ai-test
    cd /tmp/ai-test

    cat > ai_benchmark.rs << 'EOF'
use candle_core::{Device, Tensor, Result};
use std::time::Instant;

fn simple_nn_inference() -> Result<()> {
    let device = Device::Cpu;

    // Create synthetic data
    let batch_size = 32;
    let input_size = 784;
    let hidden_size = 128;
    let output_size = 10;

    let start = Instant::now();

    // Create random tensors
    let input = Tensor::randn(0f32, 1.0, (batch_size, input_size), &device)?;
    let w1 = Tensor::randn(0f32, 0.1, (input_size, hidden_size), &device)?;
    let b1 = Tensor::zeros((hidden_size,), &device)?;
    let w2 = Tensor::randn(0f32, 0.1, (hidden_size, output_size), &device)?;
    let b2 = Tensor::zeros((output_size,), &device)?;

    // Forward pass
    let hidden = input.matmul(&w1)?.add(&b1)?.relu()?;
    let output = hidden.matmul(&w2)?.add(&b2)?;

    let duration = start.elapsed();
    println!("Neural network inference: {:?}", duration);
    println!("Output shape: {:?}", output.shape());

    Ok(())
}

fn main() {
    match simple_nn_inference() {
        Ok(_) => println!("AI inference test completed successfully"),
        Err(e) => println!("AI inference test failed: {}", e),
    }
}
EOF

    # Compile and run
    rustc ai_benchmark.rs -L /usr/lib -lcandle_core -lcandle_nn
    time ./ai_benchmark
fi
```

### 6. Integration Verification

#### Check Package Dependencies
```bash
# Verify all Rust-related packages are installed
qlist -I | grep -E "(rust|candle|regicide)"

# Check for conflicting packages
emerge -pv @regicide-rust-base @regicide-rust-ai @regicide-rust-embedded

# Verify no circular dependencies
emerge --deep --with-bdeps=y --pretend @regicide-rust-base
```

#### Test System Integration
```bash
# Test that Rust toolchain works with system
rustc --print sysroot

# Test cargo package management
cargo search candle-core

# Test cross-compilation toolchain
rustup target list --installed

# Verify eselect integration
eselect rust list
```

## Troubleshooting

### Common Issues

#### Overlay Not Syncing
```bash
# Check overlay configuration
cat /etc/portage/repos.conf/regicide-rust.conf

# Manual sync
sudo emaint sync -r regicide-rust

# Check repository permissions
ls -la /var/db/repos/regicide-rust
```

#### Package Installation Failures
```bash
# Check for masked packages
emerge --searchdesc rust | grep masked

# Check for keyword issues
emerge -pv --autounmask-use @regicide-rust-base

# Check for missing dependencies
emerge --deep --with-bdeps=y --pretend @regicide-rust-base
```

#### Cross-compilation Issues
```bash
# Install cross-toolchain
sudo emerge cross-arm-none-eabi/newlib
sudo emerge cross-aarch64-linux-gnu/gcc

# Verify toolchain availability
arm-none-eabi-gcc --version
aarch64-linux-gnu-gcc --version
```

### Performance Issues

#### Slow Compilation
```bash
# Enable parallel compilation
export MAKEOPTS="-j$(nproc)"
export CARGO_BUILD_JOBS=$(nproc)

# Use compiler cache
emerge distcc
```

#### Large Binary Sizes
```bash
# Optimize for size
export RUSTFLAGS="-C opt-level=s -C lto -C panic=abort"

# Strip binaries
sudo emerge strip
```

## Success Criteria

The overlay integration is successful when:

### ✅ Package Management
- Overlay syncs correctly with portage
- All package sets install without errors
- No circular dependencies exist
- Package conflicts are resolved

### ✅ Toolchain Functionality
- Rust compiler works correctly
- Cargo package manager functions
- Cross-compilation targets are available
- AI/ML packages install and run

### ✅ Performance
- Compilation times are reasonable
- Cross-compilation works efficiently
- AI inference performs acceptably
- Memory usage is within expected bounds

### ✅ Integration
- System-wide Rust toolchain integration
- Proper eselect support
- Environment variables are set correctly
- Documentation is accessible

## Reporting Results

Create a test report with:
```bash
# Generate system information
emerge --info > system-info.txt
rustc --version -v > rust-info.txt
cargo --version > cargo-info.txt

# Package inventory
qlist -I | grep -E "(rust|candle|regicide)" > installed-packages.txt

# Performance benchmarks
# Include compilation times and inference speeds
```

## Next Steps

After successful integration testing:

1. **Documentation**: Update installation guides with verified procedures
2. **Optimization**: Fine-tune package configurations for performance
3. **Expansion**: Add more AI/ML packages and embedded targets
4. **Community**: Publish overlay for broader adoption

---

This guide ensures comprehensive testing of the Regicide-Rust overlay in live environments, validating both functionality and performance.