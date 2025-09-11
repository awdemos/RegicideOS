#!/bin/bash
# RegicideOS Installer Safety Test Suite
# Comprehensive testing for installer safety and reliability

set -euo pipefail

echo "=== RegicideOS Installer Safety Test Suite ==="
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
            test_files=(tests/installer/unit/test_*.py)
            ;;
        "Safety")
            test_files=(tests/installer/safety/test_*.py)
            ;;
        "Integration")
            test_files=(tests/installer/integration/test_*.py)
            ;;
        "All")
            test_files=(tests/installer/**/test_*.py)
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

# Function to check test dependencies
check_dependencies() {
    echo -e "${BLUE}Checking test dependencies...${NC}"
    
    local deps=("python3" "pytest")
    local missing_deps=()
    
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            missing_deps+=("$dep")
        fi
    done
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        echo -e "${RED}Missing dependencies:${NC}"
        for dep in "${missing_deps[@]}"; do
            echo "  - $dep"
        done
        echo
        echo "Please install missing dependencies:"
        echo "  sudo apt-get install python3-pytest  # Debian/Ubuntu"
        echo "  pip install pytest                    # pip"
        return 1
    fi
    
    echo -e "${GREEN}✓ All dependencies available${NC}"
    echo
    return 0
}

# Function to run safety-critical tests first
run_critical_tests() {
    echo -e "${RED}=== CRITICAL SAFETY TESTS ===${NC}"
    echo "These tests verify that dangerous operations cannot be executed accidentally."
    echo
    
    # Test UEFI detection (most critical)
    echo -e "${YELLOW}Testing UEFI detection (critical safety gate)...${NC}"
    if python3 -c "
import sys
sys.path.insert(0, '.')
from tests.installer.unit.test_uefi_detection import TestUEFIDetection
import unittest
suite = unittest.TestLoader().loadTestsFromTestCase(TestUEFIDetection)
runner = unittest.TextTestRunner(verbosity=2)
result = runner.run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
    "; then
        echo -e "  ${GREEN}✓ UEFI detection tests passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "  ${RED}✗ UEFI detection tests failed${NC}"
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
    
    # Test destructive operations safety
    echo -e "${YELLOW}Testing destructive operations safety...${NC}"
    if python3 -c "
import sys
sys.path.insert(0, '.')
from tests.installer.safety.test_destructive_operations import TestDestructiveOperationSafety
import unittest
suite = unittest.TestLoader().loadTestsFromTestCase(TestDestructiveOperationSafety)
runner = unittest.TextTestRunner(verbosity=2)
result = runner.run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
    "; then
        echo -e "  ${GREEN}✓ Destructive operations safety tests passed${NC}"
        ((PASSED_TESTS++))
    else
        echo -e "  ${RED}✗ Destructive operations safety tests failed${NC}"
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
        echo -e "${GREEN}Installer is safe for production use.${NC}"
        return 0
    else
        echo -e "${RED}✗ $FAILED_TESTS test(s) failed!${NC}"
        echo -e "${RED}Installer is NOT SAFE for production use!${NC}"
        echo
        echo "Please fix failing tests before using the installer."
        return 1
    fi
}

# Main execution
main() {
    echo "RegicideOS Installer Safety Test Suite"
    echo "======================================"
    echo
    
    # Check dependencies first
    if ! check_dependencies; then
        exit 1
    fi
    
    # Run critical safety tests first
    run_critical_tests
    
    # If critical tests failed, stop here
    if [[ $FAILED_TESTS -gt 0 ]]; then
        echo -e "${RED}Critical safety tests failed. Stopping test suite.${NC}"
        generate_report
        exit 1
    fi
    
    # Run all test categories
    run_test_category "Unit" "Unit tests for individual components"
    run_test_category "Safety" "Safety-critical operation tests"
    run_test_category "Integration" "Integration tests for complete workflows"
    
    # Generate final report
    generate_report
}

# Parse command line arguments
case "${1:-}" in
    "unit")
        check_dependencies
        run_test_category "Unit" "Unit tests for individual components"
        generate_report
        ;;
    "safety")
        check_dependencies
        run_critical_tests
        run_test_category "Safety" "Safety-critical operation tests"
        generate_report
        ;;
    "integration")
        check_dependencies
        run_test_category "Integration" "Integration tests for complete workflows"
        generate_report
        ;;
    "critical")
        check_dependencies
        run_critical_tests
        generate_report
        ;;
    *)
        main
        ;;
esac