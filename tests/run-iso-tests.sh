#!/bin/bash
# RegicideOS ISO Creation Safety Test Suite
# Comprehensive testing for ISO creation and validation

set -euo pipefail

# Anchor everything to the repository root so the runner works no matter where
# it is invoked from (the globs and pytest paths below are all relative to it).
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

shopt -s globstar nullglob

echo "=== RegicideOS ISO Creation Safety Test Suite ==="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test result counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to run a test category
run_test_category() {
    local category=$1
    local description=$2

    echo -e "${BLUE}Running $category tests...${NC}"
    echo "Description: $description"
    echo

    local test_files=()
    case $category in
        "Unit")
            test_files=("$REPO_ROOT"/tests/iso/unit/test_*.py)
            ;;
        "Integration")
            test_files=("$REPO_ROOT"/tests/iso/integration/test_*.py)
            ;;
        "Validation")
            test_files=("$REPO_ROOT"/tests/iso/validation/test_*.py)
            ;;
        "Safety")
            test_files=("$REPO_ROOT"/tests/iso/safety/test_*.py)
            ;;
        "All")
            test_files=("$REPO_ROOT"/tests/iso/**/test_*.py)
            ;;
        *)
            echo "Unknown test category: $category"
            return 1
            ;;
    esac

    local matched=0
    for test_file in "${test_files[@]}"; do
        if [[ -f "$test_file" ]]; then
            matched=$((matched + 1))
            echo -e "${YELLOW}Testing: $(basename "$test_file")${NC}"

            # Run the test and capture output
            local test_output
            if test_output=$(python3 -m pytest "$test_file" -q 2>&1); then
                echo -e "  ${GREEN}✓ PASSED${NC}"
                PASSED_TESTS=$((PASSED_TESTS + 1))
            else
                echo -e "  ${RED}✗ FAILED${NC}"
                echo "$test_output" | tail -20
                FAILED_TESTS=$((FAILED_TESTS + 1))
            fi
            TOTAL_TESTS=$((TOTAL_TESTS + 1))
        fi
    done

    if [[ $matched -eq 0 ]]; then
        echo -e "${RED}✗ No test files found for category '$category' under $REPO_ROOT/tests/iso${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    fi

    echo
}

# Run one pytest target (file or file::Class) and record the outcome.
run_test_target() {
    local description=$1
    local target=$2

    echo -e "${YELLOW}Testing $description...${NC}"

    if [[ ! -f "${target%%::*}" ]]; then
        echo -e "  ${RED}✗ FAILED${NC} (test file not found: ${target%%::*})"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
        return 1
    fi

    local test_output
    if test_output=$(python3 -m pytest "$target" -q 2>&1); then
        echo -e "  ${GREEN}✓ PASSED${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "  ${RED}✗ FAILED${NC}"
        echo "$test_output" | tail -20
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
}

# Function to check ISO creation dependencies
check_dependencies() {
    echo -e "${BLUE}Checking ISO creation dependencies...${NC}"
    
    local -A deps=(["python3"]="Python 3" ["pytest"]="Pytest testing framework" ["xorriso"]="Xorriso ISO creation tool" ["mksquashfs"]="Squashfs filesystem creator")
    local missing_deps=()
    
    for dep in "${!deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            missing_deps+=("$dep (${deps[$dep]})")
        fi
    done
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        echo -e "${RED}Missing dependencies:${NC}"
        for dep in "${missing_deps[@]}"; do
            echo "  - $dep"
        done
        echo
        echo "Please install missing dependencies:"
        echo "  sudo apt-get install xorriso squashfs-tools python3-pytest  # Debian/Ubuntu"
        echo "  sudo dnf install xorriso squashfs-tools python3-pytest     # Fedora"
        return 1
    fi
    
    echo -e "${GREEN}✓ All dependencies available${NC}"
    echo
    return 0
}

# Function to test ISO build process
test_iso_build_process() {
    echo -e "${BLUE}Testing ISO build process...${NC}"

    run_test_target "ISO configuration validation" \
        "$REPO_ROOT/tests/iso/unit/test_iso_config.py::TestISOConfig"
    run_test_target "ISO build script validation" \
        "$REPO_ROOT/tests/iso/unit/test_iso_build.py::TestISOBuild"

    echo
}

# Function to test ISO validation
test_iso_validation() {
    echo -e "${BLUE}Testing ISO validation...${NC}"

    run_test_target "ISO checksum validation" \
        "$REPO_ROOT/tests/iso/validation/test_iso_validation.py::TestISOChecksumValidation"
    run_test_target "ISO boot validation" \
        "$REPO_ROOT/tests/iso/validation/test_iso_validation.py::TestISOBootValidation"

    echo
}

# Function to test ISO safety
test_iso_safety() {
    echo -e "${BLUE}Testing ISO safety...${NC}"

    run_test_target "secure boot validation" \
        "$REPO_ROOT/tests/iso/safety/test_iso_safety.py::TestISOCreationSafety"
    run_test_target "ISO artifact validation" \
        "$REPO_ROOT/tests/iso/safety/test_iso_safety.py::TestISOBuildProcessSafety"

    echo
}

# Function to generate test report
generate_report() {
    echo -e "${BLUE}=== Test Report ===${NC}"
    echo "Total tests run: $TOTAL_TESTS"
    echo "Tests passed: $PASSED_TESTS"
    echo "Tests failed: $FAILED_TESTS"

    # Zero tests collected is a failure, not a success: it means the suite
    # exercised nothing and any "pass" would be vacuous.
    if [[ $TOTAL_TESTS -eq 0 ]]; then
        echo -e "${RED}✗ No tests were collected!${NC}"
        echo -e "${RED}ISO creation process is NOT SAFE for production use!${NC}"
        return 1
    fi

    if [[ $FAILED_TESTS -eq 0 ]]; then
        echo -e "${GREEN}✓ All tests passed!${NC}"
        echo -e "${GREEN}ISO creation process is safe for production use.${NC}"
        return 0
    else
        echo -e "${RED}✗ $FAILED_TESTS test(s) failed!${NC}"
        echo -e "${RED}ISO creation process is NOT SAFE for production use!${NC}"
        echo
        echo "Please fix failing tests before creating ISO images."
        return 1
    fi
}

# Main execution
main() {
    echo "RegicideOS ISO Creation Safety Test Suite"
    echo "========================================="
    echo
    
    # Check dependencies first
    if ! check_dependencies; then
        exit 1
    fi
    
    # Run critical ISO build tests first
    test_iso_build_process
    
    # If build tests failed, stop here
    if [[ $FAILED_TESTS -gt 0 ]]; then
        echo -e "${RED}Critical build tests failed. Stopping test suite.${NC}"
        generate_report
        exit 1
    fi
    
    # Run validation tests
    test_iso_validation
    
    # Run safety tests
    test_iso_safety

    # Generate final report (non-zero exit if any category failed or nothing ran)
    generate_report
    exit $?
}

# Parse command line arguments
case "${1:-}" in
    "unit")
        check_dependencies
        run_test_category "Unit" "Unit tests for ISO creation components"
        generate_report
        exit $?
        ;;
    "integration")
        check_dependencies
        run_test_category "Integration" "Integration tests for complete ISO workflows"
        generate_report
        exit $?
        ;;
    "validation")
        check_dependencies
        test_iso_validation
        generate_report
        exit $?
        ;;
    "safety")
        check_dependencies
        test_iso_safety
        generate_report
        exit $?
        ;;
    "build")
        check_dependencies
        test_iso_build_process
        generate_report
        exit $?
        ;;
    *)
        main
        ;;
esac