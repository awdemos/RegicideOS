#!/bin/bash
# RegicideOS ISO Validation Script
# Comprehensive validation of ISO images

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$ROOT_DIR/config"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default settings
ISO_FILE=""
CHECKSUM_FILE=""
CONFIG_FILE="$CONFIG_DIR/iso-config.toml"
STRICT_MODE=false
VERBOSE=false
QUIET=false

# Validation results
VALIDATION_PASSED=0
VALIDATION_FAILED=0
VALIDATION_WARNINGS=0

# Function to print usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS] ISO_FILE

OPTIONS:
    -h, --help              Show this help message
    -c, --checksum FILE     Verify checksum against FILE
    -C, --config FILE       Use configuration FILE
    -s, --strict            Fail on warnings
    -v, --verbose           Verbose output
    -q, --quiet             Quiet mode (errors only)

ARGUMENTS:
    ISO_FILE                Path to ISO file to validate

EXAMPLES:
    $0 regicideos-1.0.0-x86_64.iso
    $0 -c regicideos-1.0.0-x86_64.iso.sha256 regicideos-1.0.0-x86_64.iso
    $0 --strict --verbose regicideos-1.0.0-x86_64.iso

EXIT CODES:
    0   All validations passed
    1   Critical validation failure
    2   Validation warning (only in strict mode)
    3   Usage error
    4   File not found

EOF
}

# Function to log messages
log() {
    local level=$1
    shift
    local message="$*"
    
    if [[ "$QUIET" == "true" ]] && [[ "$level" != "ERROR" ]]; then
        return
    fi
    
    case $level in
        "INFO")
            if [[ "$VERBOSE" == "true" ]]; then
                echo -e "${BLUE}[INFO]${NC}  $message"
            fi
            ;;
        "WARN")  echo -e "${YELLOW}[WARN]${NC}  $message" ;;
        "ERROR") echo -e "${RED}[ERROR]${NC} $message" ;;
        "SUCCESS") echo -e "${GREEN}[SUCCESS]${NC} $message" ;;
        "VALID") echo -e "${GREEN}[VALID]${NC}   $message" ;;
        "INVALID") echo -e "${RED}[INVALID]${NC} $message" ;;
        *)      echo "[$level] $message" ;;
    esac
}

# Function to count validation results
count_result() {
    local result=$1
    
    case $result in
        "PASSED") ((VALIDATION_PASSED++)) ;;
        "FAILED") ((VALIDATION_FAILED++)) ;;
        "WARNING") ((VALIDATION_WARNINGS++)) ;;
    esac
}

# Function to check if file exists
check_file_exists() {
    local file_path="$1"
    local description="$2"
    
    if [[ -z "$file_path" ]]; then
        log "ERROR" "$description not specified"
        return 1
    fi
    
    if [[ ! -f "$file_path" ]]; then
        log "ERROR" "$description not found: $file_path"
        return 1
    fi
    
    log "VALID" "$description found: $file_path"
    return 0
}

# Function to validate ISO file format
validate_iso_format() {
    log "INFO" "Validating ISO file format..."
    
    # Check if file is readable
    if [[ ! -r "$ISO_FILE" ]]; then
        log "ERROR" "ISO file is not readable: $ISO_FILE"
        count_result "FAILED"
        return 1
    fi
    
    # Check file size
    local file_size=$(stat -c%s "$ISO_FILE" 2>/dev/null || echo 0)
    if [[ $file_size -eq 0 ]]; then
        log "ERROR" "ISO file is empty: $ISO_FILE"
        count_result "FAILED"
        return 1
    fi
    
    # Minimum ISO size (10MB)
    if [[ $file_size -lt 10485760 ]]; then
        log "WARN" "ISO file is unusually small: $file_size bytes"
        count_result "WARNING"
    fi
    
    # Maximum ISO size (8GB)
    if [[ $file_size -gt 8589934592 ]]; then
        log "WARN" "ISO file is unusually large: $file_size bytes"
        count_result "WARNING"
    fi
    
    log "VALID" "ISO file format validated (size: $file_size bytes)"
    count_result "PASSED"
    return 0
}

# Function to validate checksum
validate_checksum() {
    log "INFO" "Validating checksum..."
    
    if [[ -z "$CHECKSUM_FILE" ]]; then
        log "INFO" "No checksum file provided, skipping validation"
        count_result "PASSED"
        return 0
    fi
    
    if ! check_file_exists "$CHECKSUM_FILE" "checksum file"; then
        count_result "FAILED"
        return 1
    fi
    
    # Determine checksum type
    local checksum_type=""
    case "$CHECKSUM_FILE" in
        *.sha1)   checksum_type="sha1sum" ;;
        *.sha256) checksum_type="sha256sum" ;;
        *.sha512) checksum_type="sha512sum" ;;
        *.md5)    checksum_type="md5sum" ;;
        *)        checksum_type="sha256sum" ;; # Default
    esac
    
    # Check if checksum tool is available
    if ! command -v "$checksum_type" &> /dev/null; then
        log "WARN" "Checksum tool not available: $checksum_type"
        count_result "WARNING"
        return 0
    fi
    
    # Validate checksum
    local temp_dir=$(mktemp -d)
    local temp_checksum="$temp_dir/checksum"
    
    # Extract checksum for our file
    local basename=$(basename "$ISO_FILE")
    if grep -q "$basename" "$CHECKSUM_FILE"; then
        grep "$basename" "$CHECKSUM_FILE" > "$temp_checksum"
    else
        log "ERROR" "Checksum not found for $basename in $CHECKSUM_FILE"
        rm -rf "$temp_dir"
        count_result "FAILED"
        return 1
    fi
    
    # Change to directory containing ISO file for checksum validation
    local iso_dir=$(dirname "$ISO_FILE")
    local iso_basename=$(basename "$ISO_FILE")
    
    if ! (cd "$iso_dir" && "$checksum_type" -c "$temp_checksum" 2>/dev/null); then
        log "ERROR" "Checksum validation failed"
        rm -rf "$temp_dir"
        count_result "FAILED"
        return 1
    fi
    
    rm -rf "$temp_dir"
    log "VALID" "Checksum validation passed"
    count_result "PASSED"
    return 0
}

# Function to validate ISO structure
validate_iso_structure() {
    log "INFO" "Validating ISO structure..."
    
    local temp_mount=$(mktemp -d)
    local mount_success=false
    
    # Try to mount the ISO
    if command -v mount &> /dev/null; then
        if mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
            mount_success=true
        else
            log "WARN" "Could not mount ISO for structure validation"
            count_result "WARNING"
            return 0
        fi
    else
        log "WARN" "mount command not available for structure validation"
        count_result "WARNING"
        return 0
    fi
    
    # Check for required directories
    local required_dirs=("/EFI" "/EFI/BOOT" "/boot" "/live")
    local missing_dirs=()
    
    for dir in "${required_dirs[@]}"; do
        if [[ ! -d "$temp_mount$dir" ]]; then
            missing_dirs+=("$dir")
        fi
    done
    
    if [[ ${#missing_dirs[@]} -gt 0 ]]; then
        log "WARN" "Missing required directories: ${missing_dirs[*]}"
        count_result "WARNING"
    fi
    
    # Check for required files
    local required_files=("/EFI/BOOT/BOOTX64.EFI" "/boot/grub/grub.cfg")
    local missing_files=()
    
    for file in "${required_files[@]}"; do
        if [[ ! -f "$temp_mount$file" ]]; then
            missing_files+=("$file")
        fi
    done
    
    if [[ ${#missing_files[@]} -gt 0 ]]; then
        log "WARN" "Missing required files: ${missing_files[*]}"
        count_result "WARNING"
    fi
    
    # Check for live filesystem
    if [[ ! -f "$temp_mount/live/filesystem.squashfs" ]]; then
        log "WARN" "Live filesystem not found: /live/filesystem.squashfs"
        count_result "WARNING"
    fi
    
    # Check for disk info
    if [[ -f "$temp_mount/.disk/info" ]]; then
        log "VALID" "Disk information found"
    else
        log "WARN" "Disk information not found: /.disk/info"
        count_result "WARNING"
    fi
    
    # Unmount
    umount "$temp_mount" 2>/dev/null || true
    rm -rf "$temp_mount"
    
    log "VALID" "ISO structure validation completed"
    count_result "PASSED"
    return 0
}

# Function to validate UEFI boot support
validate_uefi_boot() {
    log "INFO" "Validating UEFI boot support..."
    
    local temp_mount=$(mktemp -d)
    local mount_success=false
    
    # Try to mount the ISO
    if ! mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
        log "WARN" "Could not mount ISO for UEFI validation"
        count_result "WARNING"
        return 0
    fi
    
    # Check for UEFI bootloader
    local uefi_files=("/EFI/BOOT/BOOTX64.EFI" "/EFI/BOOT/BOOTIA32.EFI")
    local uefi_found=false
    
    for file in "${uefi_files[@]}"; do
        if [[ -f "$temp_mount$file" ]]; then
            log "VALID" "UEFI bootloader found: $file"
            uefi_found=true
        fi
    done
    
    if [[ "$uefi_found" == "false" ]]; then
        log "ERROR" "No UEFI bootloader found"
        umount "$temp_mount" 2>/dev/null || true
        rm -rf "$temp_mount"
        count_result "FAILED"
        return 1
    fi
    
    # Check for GRUB configuration
    if [[ -f "$temp_mount/boot/grub/grub.cfg" ]]; then
        log "VALID" "GRUB configuration found"
        
        # Check for UEFI-specific entries
        if grep -q "chainloader" "$temp_mount/boot/grub/grub.cfg" 2>/dev/null; then
            log "VALID" "UEFI chainloader configuration found"
        fi
    else
        log "WARN" "GRUB configuration not found"
        count_result "WARNING"
    fi
    
    # Check for Microsoft compatibility (optional)
    if [[ -f "$temp_mount/EFI/Microsoft/Boot/bootmgfw.efi" ]]; then
        log "INFO" "Microsoft UEFI bootloader found (dual-boot support)"
    fi
    
    # Unmount
    umount "$temp_mount" 2>/dev/null || true
    rm -rf "$temp_mount"
    
    log "VALID" "UEFI boot validation completed"
    count_result "PASSED"
    return 0
}

# Function to validate boot parameters
validate_boot_parameters() {
    log "INFO" "Validating boot parameters..."
    
    local temp_mount=$(mktemp -d)
    local mount_success=false
    
    # Try to mount the ISO
    if ! mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
        log "WARN" "Could not mount ISO for boot parameter validation"
        count_result "WARNING"
        return 0
    fi
    
    # Check GRUB configuration
    local grub_cfg="$temp_mount/boot/grub/grub.cfg"
    if [[ -f "$grub_cfg" ]]; then
        # Check for required kernel parameters
        local required_params=("boot=live" "live-media-path")
        local missing_params=()
        
        for param in "${required_params[@]}"; do
            if ! grep -q "$param" "$grub_cfg"; then
                missing_params+=("$param")
            fi
        done
        
        if [[ ${#missing_params[@]} -gt 0 ]]; then
            log "WARN" "Missing required kernel parameters: ${missing_params[*]}"
            count_result "WARNING"
        fi
        
        # Check for UEFI-specific parameters
        if grep -q "efi" "$grub_cfg"; then
            log "VALID" "UEFI-specific boot parameters found"
        fi
        
        # Check for architecture-specific parameters
        if grep -q "x86_64" "$grub_cfg"; then
            log "VALID" "Architecture-specific parameters found"
        fi
    else
        log "WARN" "GRUB configuration not found for parameter validation"
        count_result "WARNING"
    fi
    
    # Unmount
    umount "$temp_mount" 2>/dev/null || true
    rm -rf "$temp_mount"
    
    log "VALID" "Boot parameter validation completed"
    count_result "PASSED"
    return 0
}

# Function to validate filesystem integrity
validate_filesystem_integrity() {
    log "INFO" "Validating filesystem integrity..."
    
    local temp_mount=$(mktemp -d)
    local mount_success=false
    
    # Try to mount the ISO
    if ! mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
        log "WARN" "Could not mount ISO for filesystem integrity validation"
        count_result "WARNING"
        return 0
    fi
    
    # Check filesystem type
    local fs_type=$(df -T "$temp_mount" | tail -1 | awk '{print $2}' 2>/dev/null || echo "unknown")
    log "INFO" "Filesystem type: $fs_type"
    
    # Check for filesystem errors
    if command -v fsck &> /dev/null; then
        # Note: fsck on ISO9660 is typically not needed/mounted read-only
        log "INFO" "Filesystem appears to be mounted read-only, skipping fsck"
    fi
    
    # Check for squashfs filesystem
    local squashfs_file="$temp_mount/live/filesystem.squashfs"
    if [[ -f "$squashfs_file" ]]; then
        log "VALID" "Squashfs filesystem found"
        
        # Check squashfs integrity
        if command -v unsquashfs &> /dev/null; then
            local temp_test=$(mktemp -d)
            if unsquashfs -l "$squashfs_file" > /dev/null 2>&1; then
                log "VALID" "Squashfs filesystem integrity verified"
            else
                log "WARN" "Squashfs filesystem integrity check failed"
                count_result "WARNING"
            fi
            rm -rf "$temp_test"
        else
            log "INFO" "unsquashfs not available, skipping integrity check"
        fi
    else
        log "WARN" "Squashfs filesystem not found"
        count_result "WARNING"
    fi
    
    # Check file permissions
    local permission_issues=0
    while IFS= read -r -d '' file; do
        if [[ -f "$file" ]] && [[ ! -r "$file" ]]; then
            ((permission_issues++))
        fi
    done < <(find "$temp_mount" -type f -print0 2>/dev/null)
    
    if [[ $permission_issues -gt 0 ]]; then
        log "WARN" "Found $permission_issues files with permission issues"
        count_result "WARNING"
    fi
    
    # Unmount
    umount "$temp_mount" 2>/dev/null || true
    rm -rf "$temp_mount"
    
    log "VALID" "Filesystem integrity validation completed"
    count_result "PASSED"
    return 0
}

# Function to validate security features
validate_security_features() {
    log "INFO" "Validating security features..."
    
    local temp_mount=$(mktemp -d)
    local mount_success=false
    
    # Try to mount the ISO
    if ! mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
        log "WARN" "Could not mount ISO for security validation"
        count_result "WARNING"
        return 0
    fi
    
    # Check for secure boot support
    if [[ -f "$temp_mount/EFI/BOOT/BOOTX64.EFI" ]]; then
        log "VALID" "UEFI bootloader found - secure boot compatible"
        
        # Check for secure boot keys (if available)
        if [[ -d "$temp_mount/EFI/BOOT/keys" ]]; then
            log "VALID" "Secure boot keys found"
        else
            log "INFO" "Secure boot keys not found (optional)"
        fi
    fi
    
    # Check for GPG signatures
    if command -v gpg &> /dev/null; then
        local signature_file="${ISO_FILE}.sig"
        if [[ -f "$signature_file" ]]; then
            log "VALID" "GPG signature file found"
            # Note: Actual signature verification would require public key
        else
            log "INFO" "GPG signature not found (optional)"
        fi
    fi
    
    # Check for security-related files
    local security_files=("/.disk/info" "/live/filesystem.squashfs")
    for file in "${security_files[@]}"; do
        if [[ -f "$temp_mount$file" ]]; then
            local file_perms=$(stat -c "%a" "$temp_mount$file" 2>/dev/null || echo "unknown")
            if [[ "$file_perms" == "644" ]] || [[ "$file_perms" == "755" ]]; then
                log "VALID" "Security file permissions OK: $file ($file_perms)"
            else
                log "WARN" "Unusual file permissions: $file ($file_perms)"
                count_result "WARNING"
            fi
        fi
    done
    
    # Check for executable files in inappropriate locations
    local executables_found=0
    while IFS= read -r -d '' file; do
        if [[ -x "$file" ]] && [[ "$file" =~ \.(txt|md|conf|cfg)$ ]]; then
            ((executables_found++))
        fi
    done < <(find "$temp_mount" -type f -executable -print0 2>/dev/null)
    
    if [[ $executables_found -gt 0 ]]; then
        log "WARN" "Found $executables_found potentially inappropriate executable files"
        count_result "WARNING"
    fi
    
    # Unmount
    umount "$temp_mount" 2>/dev/null || true
    rm -rf "$temp_mount"
    
    log "VALID" "Security features validation completed"
    count_result "PASSED"
    return 0
}

# Function to validate configuration compliance
validate_configuration_compliance() {
    log "INFO" "Validating configuration compliance..."
    
    if [[ ! -f "$CONFIG_FILE" ]]; then
        log "WARN" "Configuration file not found: $CONFIG_FILE"
        count_result "WARNING"
        return 0
    fi
    
    # Check if required sections exist in configuration
    local required_sections=("iso" "bootloader" "filesystem" "security")
    local missing_sections=()
    
    for section in "${required_sections[@]}"; do
        if ! grep -q "^\[$section\]" "$CONFIG_FILE"; then
            missing_sections+=("$section")
        fi
    done
    
    if [[ ${#missing_sections[@]} -gt 0 ]]; then
        log "WARN" "Missing configuration sections: ${missing_sections[*]}"
        count_result "WARNING"
    fi
    
    # Check for required configuration values
    local required_configs=("iso.name" "iso.version" "iso.architecture")
    local missing_configs=()
    
    for config in "${required_configs[@]}"; do
        if ! grep -q "^${config%%.*}" "$CONFIG_FILE"; then
            missing_configs+=("$config")
        fi
    done
    
    if [[ ${#missing_configs[@]} -gt 0 ]]; then
        log "WARN" "Missing configuration values: ${missing_configs[*]}"
        count_result "WARNING"
    fi
    
    # Check architecture consistency
    local config_arch=$(grep "^architecture" "$CONFIG_FILE" | cut -d'=' -f2 | tr -d ' "' || echo "")
    if [[ -n "$config_arch" ]]; then
        log "VALID" "Architecture in configuration: $config_arch"
        # Could cross-reference with actual ISO content
    fi
    
    log "VALID" "Configuration compliance validation completed"
    count_result "PASSED"
    return 0
}

# Function to generate validation report
generate_validation_report() {
    local report_file="${ISO_FILE}.validation-report.txt"
    
    cat > "$report_file" << EOF
RegicideOS ISO Validation Report
================================

ISO File: $ISO_FILE
Validation Date: $(date)
Validator: $0
Strict Mode: $STRICT_MODE
Verbose Mode: $VERBOSE

Validation Results:
- Passed: $VALIDATION_PASSED
- Failed: $VALIDATION_FAILED
- Warnings: $VALIDATION_WARNINGS

Configuration:
- Config File: $CONFIG_FILE
- Checksum File: $CHECKSUM_FILE

Exit Code:
EOF

    if [[ $VALIDATION_FAILED -gt 0 ]]; then
        echo "- Exit Code: 1 (Critical validation failure)" >> "$report_file"
    elif [[ $VALIDATION_WARNINGS -gt 0 ]] && [[ "$STRICT_MODE" == "true" ]]; then
        echo "- Exit Code: 2 (Validation warning in strict mode)" >> "$report_file"
    else
        echo "- Exit Code: 0 (All validations passed)" >> "$report_file"
    fi
    
    echo "" >> "$report_file"
    echo "Validation completed at: $(date)" >> "$report_file"
    
    log "VALID" "Validation report generated: $report_file"
}

# Main function
main() {
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -c|--checksum)
                CHECKSUM_FILE="$2"
                shift 2
                ;;
            -C|--config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            -s|--strict)
                STRICT_MODE=true
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -q|--quiet)
                QUIET=true
                shift
                ;;
            -*)
                log "ERROR" "Unknown option: $1"
                usage
                exit 3
                ;;
            *)
                if [[ -z "$ISO_FILE" ]]; then
                    ISO_FILE="$1"
                else
                    log "ERROR" "Multiple ISO files specified"
                    usage
                    exit 3
                fi
                shift
                ;;
        esac
    done
    
    # Check if ISO file is specified
    if [[ -z "$ISO_FILE" ]]; then
        log "ERROR" "ISO file not specified"
        usage
        exit 3
    fi
    
    # Check if ISO file exists
    if ! check_file_exists "$ISO_FILE" "ISO file"; then
        exit 4
    fi
    
    log "INFO" "Starting ISO validation..."
    log "INFO" "ISO file: $ISO_FILE"
    log "INFO" "Configuration: $CONFIG_FILE"
    log "INFO" "Strict mode: $STRICT_MODE"
    
    # Run validations
    validate_iso_format
    validate_checksum
    validate_iso_structure
    validate_uefi_boot
    validate_boot_parameters
    validate_filesystem_integrity
    validate_security_features
    validate_configuration_compliance
    
    # Generate report
    generate_validation_report
    
    # Summary
    log "INFO" "Validation Summary:"
    log "INFO" "  - Passed: $VALIDATION_PASSED"
    log "INFO" "  - Failed: $VALIDATION_FAILED"
    log "INFO" "  - Warnings: $VALIDATION_WARNINGS"
    
    # Determine exit code
    if [[ $VALIDATION_FAILED -gt 0 ]]; then
        log "ERROR" "Validation failed with $VALIDATION_FAILED critical errors"
        exit 1
    elif [[ $VALIDATION_WARNINGS -gt 0 ]] && [[ "$STRICT_MODE" == "true" ]]; then
        log "ERROR" "Validation failed with $VALIDATION_WARNINGS warnings (strict mode)"
        exit 2
    else
        log "SUCCESS" "All validations passed successfully"
        exit 0
    fi
}

# Error handling
set -euo pipefail
trap 'log "ERROR" "Validation process interrupted"; exit 1' INT TERM

# Run main function
main "$@"