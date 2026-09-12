#!/bin/bash
# RegicideOS Installer Test Suite
# Tests the Rust installer binary via cargo test and Python CLI wrappers

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
INSTALLER_DIR="$PROJECT_DIR/installer"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

run_test() {
    local name="$1"
    shift
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -e "${BLUE}Running:${NC} $name"
    if "$@"; then
        echo -e "  ${GREEN}✓ PASSED${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "  ${RED}✗ FAILED${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    echo
}

echo "=== RegicideOS Installer Test Suite ==="
echo "Testing Rust installer (installer/)"
echo

# Resolve the installer binary. Order: plain in-repo installer/target dir,
# then the repo's .cargo/config.toml build.target-dir, then any
# target*/release|debug/installer under the project root.
resolve_installer_bin() {
    local candidate
    for candidate in \
        "$INSTALLER_DIR/target/release/installer" \
        "$INSTALLER_DIR/target/debug/installer"; do
        if [[ -f "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done

    local cargo_config="$PROJECT_DIR/.cargo/config.toml"
    if [[ -f "$cargo_config" ]]; then
        local target_dir
        target_dir=$(sed -n 's/.*target-dir[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$cargo_config" | head -n1)
        if [[ -n "$target_dir" ]]; then
            # Relative target-dir is resolved against the workspace root.
            [[ "$target_dir" != /* ]] && target_dir="$PROJECT_DIR/$target_dir"
            for candidate in \
                "$target_dir/release/installer" \
                "$target_dir/debug/installer"; do
                if [[ -f "$candidate" ]]; then
                    printf '%s\n' "$candidate"
                    return 0
                fi
            done
        fi
    fi

    for candidate in \
        "$PROJECT_DIR"/target*/release/installer \
        "$PROJECT_DIR"/target*/debug/installer; do
        if [[ -f "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done

    return 1
}

# Ensure installer binary exists
INSTALLER_BIN="$(resolve_installer_bin || true)"
if [[ -z "$INSTALLER_BIN" ]]; then
    echo -e "${YELLOW}Building installer binary...${NC}"
    (cd "$INSTALLER_DIR" && cargo build --release)
    INSTALLER_BIN="$(resolve_installer_bin || true)"
fi
if [[ -z "$INSTALLER_BIN" ]]; then
    echo -e "${RED}Installer binary not found; build it with: cd installer && cargo build --release${NC}"
    exit 1
fi
echo -e "${BLUE}Using installer binary:${NC} $INSTALLER_BIN"

# Run Rust unit tests
run_test "Rust unit tests (cargo test --lib)" \
    bash -c "cd '$INSTALLER_DIR' && cargo test --lib --quiet"

# Run Rust CLI tests via Python
if command -v python3 &>/dev/null; then
    run_test "Installer CLI tests (Python)" \
        python3 "$SCRIPT_DIR/installer/test_rust_cli.py"
else
    echo -e "${YELLOW}python3 not found, skipping CLI tests${NC}"
fi

# Run self-contained Python safety tests
for test_file in "$SCRIPT_DIR"/installer/safety/test_destructive_operations.py; do
    if [[ -f "$test_file" ]]; then
        run_test "Python safety tests ($(basename "$test_file"))" \
            python3 "$test_file"
    fi
done

echo "======================================"
echo -e "${BLUE}Test Report${NC}"
echo "Total:  $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"

if [[ $FAILED_TESTS -eq 0 ]]; then
    echo -e "${GREEN}✓ All installer tests passed${NC}"
    exit 0
else
    echo -e "${RED}✗ $FAILED_TESTS test(s) failed${NC}"
    exit 1
fi
