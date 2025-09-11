#!/bin/bash
# RegicideOS ISO Creation Safety Test Suite
# Comprehensive testing for ISO creation and validation

set -euo pipefail

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
            test_files=(tests/iso/unit/test_*.py)
            ;;
        "Integration")
            test_files=(tests/iso/integration/test_*.py)
            ;;
        "Validation")
            test_files=(tests/iso/validation/test_*.py)
            ;;
        "All")
            test_files=(tests/iso/**/test_*.py)
            ;;
        *)
            echo "Unknown test category: $category"
            return 1
            ;;
    esac
    
    for test_file in ${test_files[@]}; do
        if [[ -f "$test_file" ]]; then
            echo -e "${YELLOW}Testing: $(basename $test_file)${NC}"
            
            # Run the test and capture output
            local test_output
            if test_output=$(python3 -m pytest "$test_file" -v 2>&1); then
                echo -e "  ${GREEN}✓ PASSED${NC}"
                ((PASSED_TESTS++))
            else
                echo -e "  ${RED}✗ FAILED${NC}"
                echo "$test_output" | head -20
                ((FAILED_TESTS++))
            fi
            ((TOTAL_TESTS++))
        fi
    done
    
    echo
}

# Function to check ISO creation dependencies
check_dependencies() {
    echo -e "${BLUE}Checking ISO creation dependencies...${NC}"
    
    local deps=(["python3"]="Python 3" ["pytest"]="Pytest testing framework" ["xorriso"]="Xorriso ISO creation tool" ["squashfs-tools"]="Squashfs compression" ["mksquashfs"]="Squashfs filesystem creator")
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
    
    # Test configuration validation
    echo -e "${YELLOW}Testing ISO configuration validation...${NC}"
    if python3 -c "
import sys
sys.path.insert(0, '.')
from tests.iso.unit.test_iso_config import TestISOConfig
import unittest
suite = unittest.TestLoader().loadTestsFromTestCase(TestISOConfig)
runner = unittest.TextTestRunner(verbosity=2)
result = runner.run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
    "; then
        echo -e "  ${GREEN}✓ ISO configuration validation passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "  ${RED}✗ ISO configuration validation failed${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test build script validation
    echo -e "${YELLOW}Testing ISO build script validation...${NC}"
    if python3 -c "
import sys
sys.path.insert(0, '.')
from tests.iso.unit.test_iso_build import TestISOBuild
import unittest
suite = unittest.TestLoader().loadTestsFromTestCase(TestISOBuild)
runner = unittest.TextTestRunner(verbosity=2)
result = runner.run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
    "; then
        echo -e "  ${GREEN}✓ ISO build script validation passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "  ${RED}✗ ISO build script validation failed${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    echo
}

# Function to test ISO validation
test_iso_validation() {
    echo -e "${BLUE}Testing ISO validation...${NC}"
    
    # Test checksum validation
    echo -e "${YELLOW}Testing ISO checksum validation...${NC}"
    if python3 -c "
import sys
sys.path.insert(0, '.')
from tests.iso.validation.test_iso_checksums import TestISOChecksums
import unittest
suite = unittest.TestLoader().loadTestsFromTestCase(TestISOChecksums)
runner = unittest.TextTestRunner(verbosity=2)
result = runner.run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
    "; then
        echo -e "  ${GREEN}✓ ISO checksum validation passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "  ${RED}✗ ISO checksum validation failed${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test boot validation
    echo -e "${YELLOW}Testing ISO boot validation...${NC}"
    if python3 -c "
import sys
sys.path.insert(0, '.')
from tests.iso.validation.test_iso_boot import TestISOBootValidation
import unittest
suite = unittest.TestLoader().loadTestsFromTestCase(TestISOBootValidation)
runner = unittest.TextTestRunner(verbosity=2)
result = runner.run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
    "; then
        echo -e "  ${GREEN}✓ ISO boot validation passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "  ${RED}✗ ISO boot validation failed${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    echo
}

# Function to test ISO safety
test_iso_safety() {
    echo -e "${BLUE}Testing ISO safety...${NC}"
    
    # Test secure boot validation
    echo -e "${YELLOW}Testing secure boot validation...${NC}"
    if python3 -c "
import sys
sys.path.insert(0, '.')
from tests.iso.safety.test_iso_security import TestISOSecurity
import unittest
suite = unittest.TestLoader().loadTestsFromTestCase(TestISOSecurity)
runner = unittest.TextTestRunner(verbosity=2)
result = runner.run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
    "; then
        echo -e "  ${GREEN}✓ Secure boot validation passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "  ${RED}✗ Secure boot validation failed${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test artifact validation
    echo -e "${YELLOW}Testing ISO artifact validation...${NC}"
    if python3 -c "
import sys
sys.path.insert(0, '.')
from tests.iso.safety.test_iso_artifacts import TestISOArtifacts
import unittest
suite = unittest.TestLoader().loadTestsFromTestCase(TestISOArtifacts)
runner = unittest.TextTestRunner(verbosity=2)
result = runner.run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
    "; then
        echo -e "  ${GREEN}✓ ISO artifact validation passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "  ${RED}✗ ISO artifact validation failed${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    echo
}

# Function to generate test report
generate_report() {
    echo -e "${BLUE}=== Test Report ===${NC}"
    echo "Total tests run: $TOTAL_TESTS"
    echo "Tests passed: $PASSED_TESTS"
    echo "Tests failed: $FAILED_TESTS"
    
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
    
    # Generate final report
    generate_report
}

# Parse command line arguments
case "${1:-}" in
    "unit")
        check_dependencies
        run_test_category "Unit" "Unit tests for ISO creation components"
        generate_report
        ;;
    "integration")
        check_dependencies
        run_test_category "Integration" "Integration tests for complete ISO workflows"
        generate_report
        ;;
    "validation")
        check_dependencies
        test_iso_validation
        generate_report
        ;;
    "safety")
        check_dependencies
        test_iso_safety
        generate_report
        ;;
    "build")
        check_dependencies
        test_iso_build_process
        generate_report
        ;;
    *)
        main
        ;;
esac