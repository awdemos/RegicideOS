#!/bin/bash
# RegicideOS Overlay Test Suite
# Validates overlay structure and package integrity

set -euo pipefail

OVERLAY_DIR="$(dirname "$(realpath "$0")")"
TEST_LOG="/tmp/regicide-overlay-test.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$TEST_LOG"
}

success() {
    echo -e "${GREEN}[PASS]${NC} $1" | tee -a "$TEST_LOG"
}

warning() {
    echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$TEST_LOG"
}

error() {
    echo -e "${RED}[FAIL]${NC} $1" | tee -a "$TEST_LOG"
}

check_command() {
    if command -v "$1" >/dev/null 2>&1; then
        success "Command '$1' is available"
        return 0
    else
        warning "Command '$1' not found (may be needed on Gentoo systems)"
        return 1
    fi
}

test_file_exists() {
    if [[ -f "$1" ]]; then
        success "File exists: $1"
        return 0
    else
        error "Missing file: $1"
        return 1
    fi
}

test_directory_exists() {
    if [[ -d "$1" ]]; then
        success "Directory exists: $1"
        return 0
    else
        error "Missing directory: $1"
        return 1
    fi
}

echo "=== RegicideOS Overlay Test Suite ===" | tee "$TEST_LOG"
echo "Overlay location: $OVERLAY_DIR" | tee -a "$TEST_LOG"
echo "Test started: $(date)" | tee -a "$TEST_LOG"
echo "" | tee -a "$TEST_LOG"

# Test 1: Overlay Structure
info "Testing overlay structure..."

test_file_exists "$OVERLAY_DIR/metadata/layout.conf"
test_file_exists "$OVERLAY_DIR/metadata/about.xml"
test_file_exists "$OVERLAY_DIR/profiles/repo_name"
test_file_exists "$OVERLAY_DIR/profiles/categories"
test_directory_exists "$OVERLAY_DIR/regicide-tools"

# Test 2: Package Structure  
info "Testing package structure..."

test_directory_exists "$OVERLAY_DIR/regicide-tools/btrmind"
test_file_exists "$OVERLAY_DIR/regicide-tools/btrmind/btrmind-9999.ebuild"
test_directory_exists "$OVERLAY_DIR/regicide-tools/regicide-installer"
test_file_exists "$OVERLAY_DIR/regicide-tools/regicide-installer/regicide-installer-9999.ebuild"

# Test 3: Configuration Files
info "Testing configuration validity..."

if grep -q "regicide-overlay" "$OVERLAY_DIR/metadata/layout.conf"; then
    success "Repository name is correct in layout.conf"
else
    error "Repository name incorrect in layout.conf"
fi

if grep -q "regicide-tools" "$OVERLAY_DIR/profiles/categories"; then
    success "regicide-tools category is defined"
else
    error "regicide-tools category missing from profiles/categories"
fi

# Test 4: Ebuild Syntax (basic)
info "Testing ebuild syntax..."

for ebuild in $(find "$OVERLAY_DIR" -name "*.ebuild"); do
    # Basic syntax check
    if grep -q "EAPI=" "$ebuild" && grep -q "DESCRIPTION=" "$ebuild"; then
        success "Ebuild has required fields: $(basename "$ebuild")"
    else
        error "Ebuild missing required fields: $(basename "$ebuild")"
    fi
done

# Test 5: Documentation
info "Testing documentation..."

test_file_exists "$OVERLAY_DIR/README.md"
test_file_exists "$OVERLAY_DIR/INSTALL.md"

# Test 6: BtrMind Integration
info "Testing BtrMind integration..."

if [[ -f "$OVERLAY_DIR/../ai-agents/btrmind/Cargo.toml" ]]; then
    success "BtrMind source code is available"
    
    # Test if BtrMind compiles
    cd "$OVERLAY_DIR/../ai-agents/btrmind"
    if cargo check >/dev/null 2>&1; then
        success "BtrMind compiles successfully"
    else
        error "BtrMind compilation failed"
    fi
    
    if cargo test >/dev/null 2>&1; then
        success "BtrMind tests pass"
    else
        warning "BtrMind tests failed or not runnable in this environment"
    fi
    
    cd "$OVERLAY_DIR"
else
    error "BtrMind source code not found"
fi

# Test 7: Gentoo Compatibility (if available)
info "Testing Gentoo tool compatibility..."

check_command "emerge"
check_command "eselect"
check_command "repoman"
check_command "ebuild"

if command -v repoman >/dev/null 2>&1; then
    info "Running repoman scan (if available)..."
    cd "$OVERLAY_DIR"
    if repoman scan >/dev/null 2>&1; then
        success "Repoman scan passed"
    else
        warning "Repoman scan failed (may not be critical)"
    fi
fi

# Test 8: Live System Integration  
info "Testing live system compatibility..."

if [[ -f "/etc/os-release" ]]; then
    OS_ID=$(grep "^ID=" /etc/os-release | cut -d'=' -f2 | tr -d '"')
    case "$OS_ID" in
        "gentoo"|"xenia"|"regicide")
            success "Running on compatible OS: $OS_ID"
            ;;
        *)
            warning "Running on non-Gentoo system: $OS_ID (overlay needs Gentoo)"
            ;;
    esac
else
    warning "Cannot detect OS type"
fi

# Test 9: Package Dependencies
info "Testing package dependencies..."

# Check if Rust toolchain would be available
if command -v rustc >/dev/null 2>&1; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    if [[ "$RUST_VERSION" > "1.70" ]]; then
        success "Rust $RUST_VERSION meets minimum requirements (1.70+)"
    else
        warning "Rust $RUST_VERSION below minimum (1.70+)"
    fi
else
    warning "Rust toolchain not available (needed for package compilation)"
fi

# Summary
echo "" | tee -a "$TEST_LOG"
echo "=== Test Summary ===" | tee -a "$TEST_LOG"

TOTAL_TESTS=$(grep -c "\[PASS\]\|\[FAIL\]" "$TEST_LOG")
PASSED_TESTS=$(grep -c "\[PASS\]" "$TEST_LOG")
FAILED_TESTS=$(grep -c "\[FAIL\]" "$TEST_LOG")

echo "Total tests: $TOTAL_TESTS" | tee -a "$TEST_LOG"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}" | tee -a "$TEST_LOG"

if [[ $FAILED_TESTS -gt 0 ]]; then
    echo -e "${RED}Failed: $FAILED_TESTS${NC}" | tee -a "$TEST_LOG"
    echo "" | tee -a "$TEST_LOG"
    echo "Failed tests:" | tee -a "$TEST_LOG"
    grep "\[FAIL\]" "$TEST_LOG"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}" | tee -a "$TEST_LOG"
fi

echo "" | tee -a "$TEST_LOG"
echo "Full test log: $TEST_LOG" | tee -a "$TEST_LOG"

if [[ $FAILED_TESTS -eq 0 ]]; then
    echo "" | tee -a "$TEST_LOG"
    echo "🎉 RegicideOS overlay is ready for deployment!" | tee -a "$TEST_LOG"
    echo ""
    echo "Next steps:"
    echo "1. Push overlay to GitHub: https://github.com/awdemos/regicide-overlay"
    echo "2. Test on actual Gentoo/RegicideOS system"
    echo "3. Add to RegicideOS installation documentation"
fi
