#!/bin/bash
# Rebuild all RegicideOS Rust modules
# This script rebuilds: installer, btrmind, and portcl

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Track build results
BUILD_RESULTS=()

build_module() {
    local module_name="$1"
    local module_path="$2"
    
    log_info "Building ${module_name}..."
    
    if [ ! -d "${module_path}" ]; then
        log_error "Module directory not found: ${module_path}"
        BUILD_RESULTS+=("${module_name}: FAILED (directory not found)")
        return 1
    fi
    
    if [ ! -f "${module_path}/Cargo.toml" ]; then
        log_error "Cargo.toml not found in: ${module_path}"
        BUILD_RESULTS+=("${module_name}: FAILED (no Cargo.toml)")
        return 1
    fi
    
    cd "${module_path}"
    
    # Clean previous build
    log_info "Cleaning previous build artifacts for ${module_name}..."
    cargo clean
    
    # Build release
    log_info "Compiling ${module_name} in release mode..."
    if cargo build --release; then
        log_info "${module_name} built successfully!"
        BUILD_RESULTS+=("${module_name}: SUCCESS")
        return 0
    else
        log_error "${module_name} build failed!"
        BUILD_RESULTS+=("${module_name}: FAILED")
        return 1
    fi
}

# Main build process
main() {
    log_info "Starting RegicideOS module rebuild..."
    log_info "Project root: ${PROJECT_ROOT}"
    echo ""
    
    # Build installer
    build_module "installer" "${PROJECT_ROOT}/installer"
    echo ""
    
    # Build btrmind
    build_module "btrmind" "${PROJECT_ROOT}/ai-agents/btrmind"
    echo ""
    
    # Build portcl
    build_module "portcl" "${PROJECT_ROOT}/ai-agents/portcl"
    echo ""
    
    # Summary
    log_info "Build Summary:"
    log_info "=============="
    for result in "${BUILD_RESULTS[@]}"; do
        if [[ "$result" == *"SUCCESS"* ]]; then
            echo -e "  ${GREEN}✓${NC} ${result}"
        else
            echo -e "  ${RED}✗${NC} ${result}"
        fi
    done
    
    # Check if any builds failed
    local failed=0
    for result in "${BUILD_RESULTS[@]}"; do
        if [[ "$result" == *"FAILED"* ]]; then
            failed=1
            break
        fi
    done
    
    if [ $failed -eq 0 ]; then
        log_info "All modules built successfully!"
        exit 0
    else
        log_error "Some modules failed to build. Check the output above."
        exit 1
    fi
}

main "$@"
