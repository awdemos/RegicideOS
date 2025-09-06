#!/bin/bash
# RegicideOS Overlay Docker Test Script
# Runs inside Gentoo container to test overlay functionality

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

echo "=== RegicideOS Overlay Docker Test ==="
echo "Running in Gentoo container"
echo "Date: $(date)"
echo ""

# Test 1: Verify Gentoo environment
info "Testing Gentoo environment..."

if command -v emerge >/dev/null 2>&1; then
    success "emerge command available"
else
    error "emerge command not found"
    exit 1
fi

if command -v eselect >/dev/null 2>&1; then
    success "eselect command available"
else
    error "eselect command not found"  
    exit 1
fi

# Test 2: Check overlay registration
info "Testing overlay registration..."

if [[ -f /etc/portage/repos.conf/regicide.conf ]]; then
    success "Overlay configuration exists"
else
    error "Overlay configuration missing"
    exit 1
fi

# Test 3: Verify overlay is recognized
info "Testing overlay recognition..."

if eselect repository list | grep -q regicide-overlay; then
    success "Overlay is recognized by eselect"
else
    error "Overlay not recognized by eselect"
    
    # Try to add it manually
    info "Attempting to add overlay manually..."
    eselect repository add regicide-overlay git https://github.com/awdemos/regicide-overlay || true
fi

# Test 4: Test package visibility
info "Testing package visibility..."

if emerge --search btrmind | grep -q "regicide-tools/btrmind"; then
    success "BtrMind package is visible"
else
    warning "BtrMind package not visible (expected - remote repo)"
fi

if emerge --search regicide-installer | grep -q "regicide-tools/regicide-installer"; then
    success "RegicideOS installer package is visible"
else
    warning "RegicideOS installer package not visible"
fi

# Test 5: Verify package metadata
info "Testing package metadata..."

if [[ -f /var/db/repos/regicide-overlay/regicide-tools/btrmind/btrmind-9999.ebuild ]]; then
    success "BtrMind ebuild exists in overlay"
    
    # Test ebuild syntax
    if grep -q "EAPI=8" /var/db/repos/regicide-overlay/regicide-tools/btrmind/btrmind-9999.ebuild; then
        success "BtrMind ebuild has correct EAPI"
    else
        error "BtrMind ebuild has incorrect EAPI"
    fi
    
    if grep -q "cargo" /var/db/repos/regicide-overlay/regicide-tools/btrmind/btrmind-9999.ebuild; then
        success "BtrMind ebuild inherits cargo"
    else
        error "BtrMind ebuild missing cargo inherit"
    fi
else
    error "BtrMind ebuild not found in overlay"
fi

# Test 6: Test pretend installation (dry run)
info "Testing package installation (dry run)..."

# This tests if the dependencies and ebuild structure is valid
if emerge --pretend --verbose regicide-tools/btrmind 2>/dev/null; then
    success "BtrMind package installation dry-run successful"
else
    warning "BtrMind package installation dry-run failed (may need dependencies)"
fi

# Test 7: Test Rust toolchain availability
info "Testing Rust toolchain..."

if command -v rustc >/dev/null 2>&1; then
    RUST_VERSION=$(rustc --version)
    success "Rust available: $RUST_VERSION"
    
    if command -v cargo >/dev/null 2>&1; then
        success "Cargo available"
    else
        error "Cargo not available"
    fi
else
    warning "Rust not available (will be installed by ebuilds)"
fi

# Test 8: Test systemd integration
info "Testing systemd integration..."

if command -v systemctl >/dev/null 2>&1; then
    success "systemd available"
else
    warning "systemd not available (may use OpenRC)"
fi

# Test 9: Create a simple BTRFS test filesystem
info "Testing BTRFS compatibility..."

if command -v mkfs.btrfs >/dev/null 2>&1; then
    success "BTRFS tools available"
    
    # Create a test BTRFS filesystem in a file
    TEST_IMG="/tmp/test-btrfs.img"
    truncate -s 100M "$TEST_IMG"
    
    if mkfs.btrfs -f "$TEST_IMG" >/dev/null 2>&1; then
        success "BTRFS filesystem creation works"
        
        # Test mounting
        mkdir -p /tmp/test-mount
        if mount -o loop "$TEST_IMG" /tmp/test-mount 2>/dev/null; then
            success "BTRFS mounting works"
            
            # Test BTRFS commands
            if btrfs filesystem usage /tmp/test-mount >/dev/null 2>&1; then
                success "BTRFS commands work"
            else
                warning "BTRFS commands failed"
            fi
            
            umount /tmp/test-mount 2>/dev/null || true
        else
            warning "BTRFS mounting failed (may need root privileges)"
        fi
        
        rm -f "$TEST_IMG"
    else
        error "BTRFS filesystem creation failed"
    fi
else
    error "BTRFS tools not available"
fi

# Test 10: BtrMind Source Integration
info "Testing BtrMind integration..."

if [[ -f "/regicide/ai-agents/btrmind/Cargo.toml" ]]; then
    success "BtrMind source code is available (mounted)"
    
    # Test if BtrMind compiles
    cd /regicide/ai-agents/btrmind
    if cargo check >/dev/null 2>&1; then
        success "BtrMind compiles successfully"
    else
        error "BtrMind compilation failed"
    fi
    
    if timeout 30 cargo test >/dev/null 2>&1; then
        success "BtrMind tests pass"
    else
        warning "BtrMind tests failed or timed out"
    fi
    
    # Test binary functionality
    if cargo build --release >/dev/null 2>&1; then
        success "BtrMind release build successful"
        
        # Test CLI
        if ./target/release/btrmind --help >/dev/null 2>&1; then
            success "BtrMind CLI works"
        else
            error "BtrMind CLI failed"
        fi
    else
        error "BtrMind release build failed"
    fi
    
    cd /var/db/repos/regicide-overlay
else
    error "BtrMind source code not found at /regicide/ai-agents/btrmind/"
fi

# Test 11: Validate overlay structure
info "Testing overlay structure compliance..."

REQUIRED_FILES=(
    "/var/db/repos/regicide-overlay/metadata/layout.conf"
    "/var/db/repos/regicide-overlay/metadata/about.xml"
    "/var/db/repos/regicide-overlay/profiles/repo_name"
    "/var/db/repos/regicide-overlay/profiles/categories"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        success "Required file exists: $(basename "$file")"
    else
        error "Missing required file: $file"
    fi
done

# Final summary
echo ""
echo "=== Docker Test Complete ==="
echo ""
echo "The RegicideOS overlay has been tested in a Gentoo environment."
echo "Any failures above need to be addressed before production deployment."
echo ""
echo "To test manually:"
echo "  docker run -it gentoo/stage3 /bin/bash"
echo "  # Then follow steps in INSTALL.md"
echo ""
echo "To deploy on RegicideOS:"
echo "  1. Clone overlay to GitHub"
echo "  2. Add overlay using eselect repository"
echo "  3. Install AI agents with emerge"
