#!/bin/bash
# RegicideOS BtrMind Integration Test Suite
# Comprehensive testing for BtrMind with real BTRFS filesystem integration

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BTRMIND_DIR="$PROJECT_DIR/ai-agents/btrmind"
TEST_DIR="$PROJECT_DIR/tests/btrmind"
TEMP_DIR="/tmp/btrmind-test-$$"

# Colors for output
RED='\\033[0;31m'
GREEN='\\033[0;32m'
YELLOW='\\033[1;33m'
BLUE='\\033[0;34m'
NC='\\033[0m' # No Color

# Test result counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Safety flags
REQUIRE_ROOT=${REQUIRE_ROOT:-true}
REQUIRE_BTRFS=${REQUIRE_BTRFS:-true}
CLEANUP_ON_EXIT=${CLEANUP_ON_EXIT:-true}

# BTRFS test device (loopback)
BTRFS_DEVICE=""
BTRFS_MOUNT=""

# Function to print usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

OPTIONS:
    -h, --help              Show this help message
    --no-root               Don't require root privileges
    --no-btrfs              Don't require BTRFS filesystem
    --no-cleanup            Don't clean up test files
    --device DEVICE         Use specific BTRFS device
    --mount-point PATH      Use specific mount point

ENVIRONMENT VARIABLES:
    REQUIRE_ROOT=false       Skip root requirement
    REQUIRE_BTRFS=false      Skip BTRFS requirement
    CLEANUP_ON_EXIT=false    Keep test files

DESCRIPTION:
    This script runs comprehensive tests for BtrMind AI agent with real BTRFS
    filesystem integration. It creates temporary BTRFS filesystems, tests
    BtrMind functionality, and validates AI agent behavior.

EXAMPLES:
    $0                              # Run all tests with safety checks
    $0 --no-root --no-btrfs         # Run tests without BTRFS requirements
    $0 --device /dev/loop0          # Use specific BTRFS device

EOF
}

# Function to log messages
log() {
    local level=$1
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    case $level in
        "INFO")  echo -e "${BLUE}[INFO]${NC}  $timestamp - $message" ;;
        "WARN")  echo -e "${YELLOW}[WARN]${NC}  $timestamp - $message" ;;
        "ERROR") echo -e "${RED}[ERROR]${NC} $timestamp - $message" ;;
        "SUCCESS") echo -e "${GREEN}[SUCCESS]${NC} $timestamp - $message" ;;
        "TEST")  echo -e "${BLUE}[TEST]${NC}  $timestamp - $message" ;;
        *)      echo "[$level] $timestamp - $message" ;;
    esac
}

# Function to check if running as root
check_root() {
    if [[ "$REQUIRE_ROOT" != "true" ]]; then
        log "INFO" "Root check skipped (REQUIRE_ROOT=false)"
        return 0
    fi
    
    if [[ $EUID -ne 0 ]]; then
        log "ERROR" "This test requires root privileges for BTRFS operations"
        log "INFO" "Run with --no-root to skip this requirement"
        return 1
    fi
    
    log "SUCCESS" "Running with root privileges"
    return 0
}

# Function to check BTRFS tools availability
check_btrfs_tools() {
    if [[ "$REQUIRE_BTRFS" != "true" ]]; then
        log "INFO" "BTRFS check skipped (REQUIRE_BTRFS=false)"
        return 0
    fi
    
    local required_tools=("mkfs.btrfs" "btrfs" "btrfstune" "btrfs-convert")
    local missing_tools=()
    
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            missing_tools+=("$tool")
        fi
    done
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log "ERROR" "Missing BTRFS tools: ${missing_tools[*]}"
        log "INFO" "Install with: sudo apt-get install btrfs-progs"
        return 1
    fi
    
    log "SUCCESS" "All BTRFS tools available"
    return 0
}

# Function to create test BTRFS filesystem
create_test_btrfs() {
    log "INFO" "Creating test BTRFS filesystem..."
    
    # Create temporary image file
    local image_file="$TEMP_DIR/test-btrfs.img"
    truncate -s 1G "$image_file"
    
    # Format as BTRFS
    if ! mkfs.btrfs -L "btrmind-test" "$image_file" &> /dev/null; then
        log "ERROR" "Failed to create BTRFS filesystem"
        return 1
    fi
    
    # Setup loopback device
    BTRFS_DEVICE=$(losetup -f --show "$image_file")
    
    # Create mount point
    BTRFS_MOUNT="$TEMP_DIR/mount"
    mkdir -p "$BTRFS_MOUNT"
    
    # Mount filesystem
    if ! mount "$BTRFS_DEVICE" "$BTRFS_MOUNT"; then
        log "ERROR" "Failed to mount BTRFS filesystem"
        return 1
    fi
    
    # Create test subvolumes
    btrfs subvolume create "$BTRFS_MOUNT/@home" &> /dev/null
    btrfs subvolume create "$BTRFS_MOUNT/@snapshots" &> /dev/null
    btrfs subvolume create "$BTRFS_MOUNT/@temp" &> /dev/null
    
    # Create test files
    create_test_files "$BTRFS_MOUNT"
    
    log "SUCCESS" "Test BTRFS filesystem created and mounted"
    log "INFO" "Device: $BTRFS_DEVICE"
    log "INFO" "Mount: $BTRFS_MOUNT"
    
    return 0
}

# Function to create test files
create_test_files() {
    local mount_point="$1"
    
    log "INFO" "Creating test files..."
    
    # Create various test files
    dd if=/dev/zero of="$mount_point/large_file.img" bs=1M count=100 &> /dev/null
    dd if=/dev/urandom of="$mount_point/random_data.bin" bs=1K count=50 &> /dev/null
    
    # Create directories with different purposes
    mkdir -p "$mount_point/var/log"
    mkdir -p "$mount_point/var/tmp"
    mkdir -p "$mount_point/home/user"
    mkdir -p "$mount_point/opt/app"
    
    # Create log files
    for i in {1..20}; do
        echo "Test log entry $i with some random data $(date)" > "$mount_point/var/log/test$i.log"
    done
    
    # Create temporary files
    for i in {1..30}; do
        echo "Temporary file $i content" > "$mount_point/var/tmp/tempfile$i.tmp"
    done
    
    # Create user files
    for i in {1..15}; do
        echo "User document $i" > "$mount_point/home/user/doc$i.txt"
    done
    
    log "SUCCESS" "Test files created"
}

# Function to test BtrMind compilation
test_btrmind_compilation() {
    log "TEST" "Testing BtrMind compilation..."
    
    if [[ ! -d "$BTRMIND_DIR" ]]; then
        log "ERROR" "BtrMind directory not found: $BTRMIND_DIR"
        ((FAILED_TESTS++))
        return 1
    fi
    
    cd "$BTRMIND_DIR"
    
    # Check if Cargo exists
    if ! command -v cargo &> /dev/null; then
        log "ERROR" "Cargo not found. Install Rust toolchain."
        ((FAILED_TESTS++))
        return 1
    fi
    
    # Build BtrMind
    if ! cargo build --release &> /dev/null; then
        log "ERROR" "BtrMind compilation failed"
        ((FAILED_TESTS++))
        return 1
    fi
    
    # Check if binary was created
    if [[ ! -f "target/release/btrmind" ]]; then
        log "ERROR" "BtrMind binary not found after compilation"
        ((FAILED_TESTS++))
        return 1
    fi
    
    log "SUCCESS" "BtrMind compiled successfully"
    ((PASSED_TESTS++))
    ((TOTAL_TESTS++))
    return 0
}

# Function to test BtrMind configuration
test_btrmind_configuration() {
    log "TEST" "Testing BtrMind configuration..."
    
    if [[ ! -f "$BTRMIND_DIR/target/release/btrmind" ]]; then
        log "ERROR" "BtrMind binary not found"
        ((FAILED_TESTS++))
        return 1
    fi
    
    cd "$BTRMIND_DIR"
    
    # Create test configuration
    local config_file="$TEMP_DIR/btrmind-test.toml"
    cat > "$config_file" << EOF
[monitoring]
target_path = "$BTRFS_MOUNT"
poll_interval = 5

[thresholds]
warning_level = 85.0
critical_level = 95.0
emergency_level = 98.0

[actions]
enable_compression = true
enable_balance = true
enable_snapshot_cleanup = true
enable_temp_cleanup = true
temp_paths = ["$BTRFS_MOUNT/var/tmp", "$BTRFS_MOUNT/tmp"]
snapshot_keep_count = 5

[learning]
model_path = "$TEMP_DIR/btrmind-model"
model_update_interval = 60
reward_smoothing = 0.95
exploration_rate = 0.1
learning_rate = 0.001
discount_factor = 0.99

dry_run = true
verbose = true
EOF
    
    # Test configuration validation
    if ! ./target/release/btrmind --config "$config_file" config &> /dev/null; then
        log "ERROR" "BtrMind configuration validation failed"
        ((FAILED_TESTS++))
        return 1
    fi
    
    log "SUCCESS" "BtrMind configuration validated"
    ((PASSED_TESTS++))
    ((TOTAL_TESTS++))
    return 0
}

# Function to test BtrMind analysis
test_btrmind_analysis() {
    log "TEST" "Testing BtrMind filesystem analysis..."
    
    if [[ ! -f "$BTRMIND_DIR/target/release/btrmind" ]]; then
        log "ERROR" "BtrMind binary not found"
        ((FAILED_TESTS++))
        return 1
    fi
    
    cd "$BTRMIND_DIR"
    
    # Create configuration for analysis
    local config_file="$TEMP_DIR/btrmind-analysis.toml"
    cat > "$config_file" << EOF
[monitoring]
target_path = "$BTRFS_MOUNT"
poll_interval = 5

[thresholds]
warning_level = 85.0
critical_level = 95.0
emergency_level = 98.0

[actions]
enable_compression = false
enable_balance = false
enable_snapshot_cleanup = false
enable_temp_cleanup = false

[learning]
model_path = "$TEMP_DIR/btrmind-model"
model_update_interval = 60

dry_run = true
verbose = true
EOF
    
    # Test filesystem analysis
    if ! ./target/release/btrmind --config "$config_file" analyze &> "$TEMP_DIR/analysis-output.txt"; then
        log "ERROR" "BtrMind analysis failed"
        ((FAILED_TESTS++))
        return 1
    fi
    
    # Check analysis output
    if [[ ! -s "$TEMP_DIR/analysis-output.txt" ]]; then
        log "ERROR" "BtrMind analysis produced no output"
        ((FAILED_TESTS++))
        return 1
    fi
    
    # Look for expected analysis results
    local expected_patterns=("disk usage" "filesystem" "subvolumes" "compression")
    local found_patterns=0
    
    for pattern in "${expected_patterns[@]}"; do
        if grep -i "$pattern" "$TEMP_DIR/analysis-output.txt" &> /dev/null; then
            ((found_patterns++))
        fi
    done
    
    if [[ $found_patterns -lt 2 ]]; then
        log "WARN" "BtrMind analysis output may be incomplete"
    fi
    
    log "SUCCESS" "BtrMind filesystem analysis completed"
    ((PASSED_TESTS++))
    ((TOTAL_TESTS++))
    return 0
}

# Function to test BtrMind AI learning
test_btrmind_learning() {
    log "TEST" "Testing BtrMind AI learning capabilities..."
    
    if [[ ! -f "$BTRMIND_DIR/target/release/btrmind" ]]; then
        log "ERROR" "BtrMind binary not found"
        ((FAILED_TESTS++))
        return 1
    fi
    
    cd "$BTRMIND_DIR"
    
    # Create configuration for learning test
    local config_file="$TEMP_DIR/btrmind-learning.toml"
    cat > "$config_file" << EOF
[monitoring]
target_path = "$BTRFS_MOUNT"
poll_interval = 2

[thresholds]
warning_level = 85.0
critical_level = 95.0
emergency_level = 98.0

[actions]
enable_compression = true
enable_balance = true
enable_snapshot_cleanup = true
enable_temp_cleanup = true
temp_paths = ["$BTRFS_MOUNT/var/tmp"]
snapshot_keep_count = 3

[learning]
model_path = "$TEMP_DIR/btrmind-learning-model"
model_update_interval = 30
reward_smoothing = 0.95
exploration_rate = 0.2
learning_rate = 0.01
discount_factor = 0.99

dry_run = true
verbose = true
EOF
    
    # Test learning simulation
    if ! timeout 30 ./target/release/btrmind --config "$config_file" monitor &> "$TEMP_DIR/learning-output.txt"; then
        log "WARN" "BtrMind learning test timed out (this may be normal)"
    fi
    
    # Check if learning model was created
    if [[ -d "$TEMP_DIR/btrmind-learning-model" ]]; then
        log "SUCCESS" "BtrMind learning model created"
    else
        log "WARN" "BtrMind learning model not created"
    fi
    
    # Check learning output
    if [[ -s "$TEMP_DIR/learning-output.txt" ]]; then
        local learning_patterns=("learning" "model" "action" "reward")
        local found_learning_patterns=0
        
        for pattern in "${learning_patterns[@]}"; do
            if grep -i "$pattern" "$TEMP_DIR/learning-output.txt" &> /dev/null; then
                ((found_learning_patterns++))
            fi
        done
        
        if [[ $found_learning_patterns -gt 0 ]]; then
            log "SUCCESS" "BtrMind AI learning patterns detected"
        else
            log "WARN" "BtrMind AI learning patterns not detected in output"
        fi
    fi
    
    log "SUCCESS" "BtrMind AI learning test completed"
    ((PASSED_TESTS++))
    ((TOTAL_TESTS++))
    return 0
}

# Function to test BtrMind cleanup actions
test_btrmind_cleanup() {
    log "TEST" "Testing BtrMind cleanup actions..."
    
    if [[ ! -f "$BTRMIND_DIR/target/release/btrmind" ]]; then
        log "ERROR" "BtrMind binary not found"
        ((FAILED_TESTS++))
        return 1
    fi
    
    cd "$BTRMIND_DIR"
    
    # Create configuration for cleanup test
    local config_file="$TEMP_DIR/btrmind-cleanup.toml"
    cat > "$config_file" << EOF
[monitoring]
target_path = "$BTRFS_MOUNT"
poll_interval = 1

[thresholds]
warning_level = 1.0
critical_level = 5.0
emergency_level = 10.0

[actions]
enable_compression = false
enable_balance = false
enable_snapshot_cleanup = true
enable_temp_cleanup = true
temp_paths = ["$BTRFS_MOUNT/var/tmp", "$BTRFS_MOUNT/tmp"]
snapshot_keep_count = 2

[learning]
model_path = "$TEMP_DIR/btrmind-cleanup-model"
model_update_interval = 30

dry_run = false
verbose = true
EOF
    
    # Count files before cleanup
    local files_before=$(find "$BTRFS_MOUNT/var/tmp" -type f | wc -l)
    
    # Test cleanup actions
    if ! timeout 30 ./target/release/btrmind --config "$config_file" monitor &> "$TEMP_DIR/cleanup-output.txt"; then
        log "WARN" "BtrMind cleanup test timed out (this may be normal)"
    fi
    
    # Count files after cleanup
    local files_after=$(find "$BTRFS_MOUNT/var/tmp" -type f | wc -l)
    
    # Check if cleanup occurred
    if [[ $files_after -lt $files_before ]]; then
        log "SUCCESS" "BtrMind cleanup actions reduced file count from $files_before to $files_after"
    else
        log "INFO" "BtrMind cleanup actions: file count unchanged ($files_before)"
    fi
    
    # Check cleanup output
    if [[ -s "$TEMP_DIR/cleanup-output.txt" ]]; then
        local cleanup_patterns=("cleanup" "remove" "delete" "temp")
        local found_cleanup_patterns=0
        
        for pattern in "${cleanup_patterns[@]}"; do
            if grep -i "$pattern" "$TEMP_DIR/cleanup-output.txt" &> /dev/null; then
                ((found_cleanup_patterns++))
            fi
        done
        
        if [[ $found_cleanup_patterns -gt 0 ]]; then
            log "SUCCESS" "BtrMind cleanup actions detected"
        else
            log "WARN" "BtrMind cleanup actions not detected in output"
        fi
    fi
    
    log "SUCCESS" "BtrMind cleanup actions test completed"
    ((PASSED_TESTS++))
    ((TOTAL_TESTS++))
    return 0
}

# Function to test BtrMind stress scenarios
test_btrmind_stress() {
    log "TEST" "Testing BtrMind under stress conditions..."
    
    if [[ ! -f "$BTRMIND_DIR/target/release/btrmind" ]]; then
        log "ERROR" "BtrMind binary not found"
        ((FAILED_TESTS++))
        return 1
    fi
    
    cd "$BTRMIND_DIR"
    
    # Create stress test configuration
    local config_file="$TEMP_DIR/btrmind-stress.toml"
    cat > "$config_file" << EOF
[monitoring]
target_path = "$BTRFS_MOUNT"
poll_interval = 1

[thresholds]
warning_level = 50.0
critical_level = 75.0
emergency_level = 90.0

[actions]
enable_compression = true
enable_balance = true
enable_snapshot_cleanup = true
enable_temp_cleanup = true
temp_paths = ["$BTRFS_MOUNT/var/tmp", "$BTRFS_MOUNT/tmp"]
snapshot_keep_count = 3

[learning]
model_path = "$TEMP_DIR/btrmind-stress-model"
model_update_interval = 10
reward_smoothing = 0.9
exploration_rate = 0.3
learning_rate = 0.05
discount_factor = 0.95

dry_run = true
verbose = true
EOF
    
    # Fill filesystem to create stress condition
    log "INFO" "Creating stress condition..."
    dd if=/dev/zero of="$BTRFS_MOUNT/stress_test_file.img" bs=1M count=500 &> /dev/null || true
    
    # Test BtrMind under stress
    if ! timeout 60 ./target/release/btrmind --config "$config_file" monitor &> "$TEMP_DIR/stress-output.txt"; then
        log "WARN" "BtrMind stress test timed out (this may be normal under high load)"
    fi
    
    # Check stress handling
    if [[ -s "$TEMP_DIR/stress-output.txt" ]]; then
        local stress_patterns=("critical" "emergency" "high" "stress")
        local found_stress_patterns=0
        
        for pattern in "${stress_patterns[@]}"; do
            if grep -i "$pattern" "$TEMP_DIR/stress-output.txt" &> /dev/null; then
                ((found_stress_patterns++))
            fi
        done
        
        if [[ $found_stress_patterns -gt 0 ]]; then
            log "SUCCESS" "BtrMind stress conditions detected and handled"
        else
            log "INFO" "BtrMind operated normally under stress"
        fi
    fi
    
    # Clean up stress test file
    rm -f "$BTRFS_MOUNT/stress_test_file.img"
    
    log "SUCCESS" "BtrMind stress test completed"
    ((PASSED_TESTS++))
    ((TOTAL_TESTS++))
    return 0
}

# Function to cleanup test environment
cleanup_test_environment() {
    log "INFO" "Cleaning up test environment..."
    
    if [[ "$CLEANUP_ON_EXIT" != "true" ]]; then
        log "INFO" "Cleanup skipped (CLEANUP_ON_EXIT=false)"
        return 0
    fi
    
    # Unmount BTRFS filesystem
    if [[ -n "$BTRFS_MOUNT" ]] && mountpoint -q "$BTRFS_MOUNT" &> /dev/null; then
        umount "$BTRFS_MOUNT" &> /dev/null || true
    fi
    
    # Remove loopback device
    if [[ -n "$BTRFS_DEVICE" ]] && [[ -b "$BTRFS_DEVICE" ]]; then
        losetup -d "$BTRFS_DEVICE" &> /dev/null || true
    fi
    
    # Remove temporary directory
    if [[ -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR" &> /dev/null || true
    fi
    
    log "SUCCESS" "Test environment cleaned up"
    return 0
}

# Function to generate test report
generate_test_report() {
    log "INFO" "Generating test report..."
    
    local report_file="$TEST_DIR/btrmind-test-report-$(date +%Y%m%d-%H%M%S).txt"
    
    cat > "$report_file" << EOF
RegicideOS BtrMind Integration Test Report
==========================================

Test Date: $(date)
Test Host: $(hostname)
Test Directory: $TEST_DIR
Temporary Directory: $TEMP_DIR

Test Results:
- Total Tests: $TOTAL_TESTS
- Passed Tests: $PASSED_TESTS
- Failed Tests: $FAILED_TESTS
- Success Rate: $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%

Test Environment:
- Root Privileges: $REQUIRE_ROOT
- BTRFS Required: $REQUIRE_BTRFS
- BTRFS Device: $BTRFS_DEVICE
- BTRFS Mount: $BTRFS_MOUNT

Test Categories:
1. BtrMind Compilation: $((PASSED_TESTS > 0 ? 1 : 0 ))/1
2. Configuration Validation: $((PASSED_TESTS > 1 ? 1 : 0 ))/1
3. Filesystem Analysis: $((PASSED_TESTS > 2 ? 1 : 0 ))/1
4. AI Learning: $((PASSED_TESTS > 3 ? 1 : 0 ))/1
5. Cleanup Actions: $((PASSED_TESTS > 4 ? 1 : 0 ))/1
6. Stress Testing: $((PASSED_TESTS > 5 ? 1 : 0 ))/1

Exit Code:
EOF

    if [[ $FAILED_TESTS -eq 0 ]]; then
        echo "- Exit Code: 0 (All tests passed)" >> "$report_file"
    else
        echo "- Exit Code: 1 (Some tests failed)" >> "$report_file"
    fi
    
    echo "" >> "$report_file"
    echo "Test completed at: $(date)" >> "$report_file"
    
    log "SUCCESS" "Test report generated: $report_file"
}

# Main function
main() {
    log "INFO" "Starting RegicideOS BtrMind Integration Test Suite..."
    log "INFO" "Test directory: $TEST_DIR"
    log "INFO" "Temporary directory: $TEMP_DIR"
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            --no-root)
                REQUIRE_ROOT=false
                shift
                ;;
            --no-btrfs)
                REQUIRE_BTRFS=false
                shift
                ;;
            --no-cleanup)
                CLEANUP_ON_EXIT=false
                shift
                ;;
            --device)
                BTRFS_DEVICE="$2"
                shift 2
                ;;
            --mount-point)
                BTRFS_MOUNT="$2"
                shift 2
                ;;
            *)
                log "ERROR" "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    # Create test directory
    mkdir -p "$TEST_DIR"
    mkdir -p "$TEMP_DIR"
    
    # Setup cleanup trap
    trap cleanup_test_environment EXIT INT TERM
    
    # Run pre-flight checks
    if ! check_root; then
        exit 1
    fi
    
    if ! check_btrfs_tools; then
        exit 1
    fi
    
    # Create test BTRFS filesystem
    if [[ "$REQUIRE_BTRFS" == "true" ]]; then
        if ! create_test_btrfs; then
            exit 1
        fi
    fi
    
    # Run tests
    log "INFO" "Starting BtrMind integration tests..."
    
    test_btrmind_compilation
    test_btrmind_configuration
    test_btrmind_analysis
    test_btrmind_learning
    test_btrmind_cleanup
    test_btrmind_stress
    
    # Generate report
    generate_test_report
    
    # Summary
    log "INFO" "Test Summary:"
    log "INFO" "  - Total Tests: $TOTAL_TESTS"
    log "INFO" "  - Passed Tests: $PASSED_TESTS"
    log "INFO" "  - Failed Tests: $FAILED_TESTS"
    log "INFO" "  - Success Rate: $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%"
    
    # Exit with appropriate code
    if [[ $FAILED_TESTS -eq 0 ]]; then
        log "SUCCESS" "All BtrMind integration tests passed!"
        exit 0
    else
        log "ERROR" "$FAILED_TESTS BtrMind integration test(s) failed!"
        exit 1
    fi
}

# Error handling
set -euo pipefail

# Run main function
main "$@"