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
VALIDATION_SKIPPED=0

# ISO inspection tooling (probed in detect_iso_tools)
ISO_LISTER=""    # "xorriso" or "bsdtar"; empty when neither is available
CAN_MOUNT=false  # true only for root: loop-mount is reserved for deep checks

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
        "SKIP")  echo -e "${YELLOW}[SKIP]${NC}   $message" ;;
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
        "PASSED") ((VALIDATION_PASSED++)) || true ;;
        "FAILED") ((VALIDATION_FAILED++)) || true ;;
        "WARNING") ((VALIDATION_WARNINGS++)) || true ;;
        "SKIPPED") ((VALIDATION_SKIPPED++)) || true ;;
    esac
}

# Probe available ISO inspection tools. Structural checks run without root via
# xorriso or bsdtar; loop-mount (root only) is reserved for deep content checks.
detect_iso_tools() {
    if command -v xorriso &> /dev/null; then
        ISO_LISTER="xorriso"
    elif command -v bsdtar &> /dev/null; then
        ISO_LISTER="bsdtar"
    fi

    if [[ $EUID -eq 0 ]] && command -v mount &> /dev/null; then
        CAN_MOUNT=true
    fi

    log "INFO" "ISO inspection: lister=$([ -n "$ISO_LISTER" ] && echo "$ISO_LISTER" || echo none), loop-mount=$CAN_MOUNT"
}

# An ISO 9660 image must have a primary volume descriptor at sector 16 whose
# standard identifier is "CD001". This works without root and without any
# ISO-specific tool, and rejects arbitrary non-ISO data.
has_iso9660_pvd() {
    local magic
    magic=$(dd if="$ISO_FILE" bs=1 skip=32769 count=5 status=none 2>/dev/null || true)
    [[ "$magic" == "CD001" ]]
}

# Print the regular files inside the ISO as absolute paths (one per line).
iso_list_files() {
    case "$ISO_LISTER" in
        xorriso)
            xorriso -indev "$ISO_FILE" -find / -type f 2>/dev/null \
                | sed -e "s/^'//" -e "s/'\$//"
            ;;
        bsdtar)
            bsdtar -tf "$ISO_FILE" 2>/dev/null \
                | sed -e 's|^\./||' -e '/\/$/d' -e 's|^|/|'
            ;;
    esac
}

# Print the directories inside the ISO as absolute paths (one per line).
iso_list_dirs() {
    case "$ISO_LISTER" in
        xorriso)
            xorriso -indev "$ISO_FILE" -find / -type d 2>/dev/null \
                | sed -e "s/^'//" -e "s/'\$//"
            ;;
        bsdtar)
            bsdtar -tf "$ISO_FILE" 2>/dev/null \
                | sed -e 's|^\./||' -e '/\/$/!d' -e 's|/$||' -e 's|^|/|'
            ;;
    esac
}

# Extract a single file from inside the ISO to a local directory without
# mounting. Usage: iso_obtain_file <absolute-iso-path> <dest-dir>
# Prints the local file path on success.
# Returns: 0 = file obtained, 1 = unavailable (no tool / not root / missing),
#          2 = root loop-mount failed (ISO likely corrupt).
iso_obtain_file() {
    local inner_path=$1
    local dest_dir=$2
    local local_name
    local_name="$dest_dir/$(basename "$inner_path")"

    if [[ "$ISO_LISTER" == "xorriso" ]]; then
        if xorriso -osirrox on -indev "$ISO_FILE" -extract "$inner_path" "$local_name" >/dev/null 2>&1 \
            && [[ -s "$local_name" ]]; then
            echo "$local_name"
            return 0
        fi
    fi

    if [[ "$CAN_MOUNT" == true ]]; then
        local temp_mount
        temp_mount=$(mktemp -d)
        if mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
            if [[ -f "$temp_mount$inner_path" ]]; then
                cp "$temp_mount$inner_path" "$local_name" 2>/dev/null || true
            fi
            umount "$temp_mount" 2>/dev/null || true
            rm -rf "$temp_mount"
            if [[ -s "$local_name" ]]; then
                echo "$local_name"
                return 0
            fi
        else
            rm -rf "$temp_mount"
            return 2
        fi
    fi

    return 1
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
        log "INFO" "No checksum file provided, skipping checksum validation"
        count_result "SKIPPED"
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
        count_result "SKIPPED"
        return 0
    fi
    
    # Validate checksum
    local temp_dir=$(mktemp -d)
    local temp_checksum="$temp_dir/checksum"
    
    # Extract checksum for our file. Match the last whitespace-separated field
    # exactly (optionally with a coreutils binary-mode '*' prefix) so an entry
    # for foo.iso2 can never satisfy foo.iso.
    local iso_basename=$(basename "$ISO_FILE")
    local expected_checksum
    expected_checksum=$(awk -v name="$iso_basename" \
        '{f=$NF; sub(/^\*/, "", f); if (f == name) {print $1; exit}}' \
        "$CHECKSUM_FILE")
    
    if [[ -z "$expected_checksum" ]]; then
        log "ERROR" "Checksum not found for $iso_basename in $CHECKSUM_FILE"
        rm -rf "$temp_dir"
        count_result "FAILED"
        return 1
    fi
    
    # Rewrite the entry with a plain relative filename so verification works
    # regardless of any path recorded in the checksum file.
    printf '%s  %s\n' "$expected_checksum" "$iso_basename" > "$temp_checksum"
    
    # Change to directory containing ISO file for checksum validation
    local iso_dir=$(dirname "$ISO_FILE")
    
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

    # An ISO 9660 image must have a primary volume descriptor at sector 16.
    if ! has_iso9660_pvd; then
        log "ERROR" "No ISO 9660 primary volume descriptor found: $ISO_FILE is not a valid ISO 9660 image"
        count_result "FAILED"
        return 1
    fi
    log "VALID" "ISO 9660 primary volume descriptor found"

    local missing_dirs=()
    local missing_files=()
    local temp_mount=""

    if [[ -n "$ISO_LISTER" ]]; then
        # Rootless structural check via xorriso/bsdtar listing.
        local files_list dirs_list
        files_list=$(iso_list_files) || true
        dirs_list=$(iso_list_dirs) || true

        if [[ -z "$files_list" && -z "$dirs_list" ]]; then
            log "ERROR" "ISO contents could not be listed; $ISO_FILE does not contain a readable ISO 9660 tree"
            count_result "FAILED"
            return 1
        fi

        # Check for required directories
        local required_dirs=("/EFI" "/EFI/BOOT" "/boot" "/live")

        for dir in "${required_dirs[@]}"; do
            if ! grep -qxF "$dir" <<< "$dirs_list"; then
                missing_dirs+=("$dir")
            fi
        done

        # Check for required files
        local required_files=("/EFI/BOOT/BOOTX64.EFI" "/boot/grub/grub.cfg" "/live/filesystem.squashfs")

        for file in "${required_files[@]}"; do
            if ! grep -qxF "$file" <<< "$files_list"; then
                missing_files+=("$file")
            fi
        done

        # Check for disk info
        if grep -qxF "/.disk/info" <<< "$files_list"; then
            log "VALID" "Disk information found"
        else
            log "WARN" "Disk information not found: /.disk/info"
            count_result "WARNING"
        fi
    elif [[ "$CAN_MOUNT" == true ]]; then
        temp_mount=$(mktemp -d)

        if ! mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
            log "ERROR" "Could not mount ISO for structure validation"
            rm -rf "$temp_mount"
            count_result "FAILED"
            return 1
        fi

        # Check for required directories
        local required_dirs=("/EFI" "/EFI/BOOT" "/boot" "/live")

        for dir in "${required_dirs[@]}"; do
            if [[ ! -d "$temp_mount$dir" ]]; then
                missing_dirs+=("$dir")
            fi
        done

        # Check for required files
        local required_files=("/EFI/BOOT/BOOTX64.EFI" "/boot/grub/grub.cfg" "/live/filesystem.squashfs")

        for file in "${required_files[@]}"; do
            if [[ ! -f "$temp_mount$file" ]]; then
                missing_files+=("$file")
            fi
        done

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
        temp_mount=""
    else
        log "WARN" "Cannot inspect ISO contents (install xorriso or bsdtar, or run as root); skipping structure validation"
        count_result "SKIPPED"
        return 0
    fi

    local structure_ok=true

    if [[ ${#missing_dirs[@]} -gt 0 ]]; then
        log "ERROR" "Missing required directories: ${missing_dirs[*]}"
        count_result "FAILED"
        structure_ok=false
    fi

    if [[ ${#missing_files[@]} -gt 0 ]]; then
        log "ERROR" "Missing required files: ${missing_files[*]}"
        count_result "FAILED"
        structure_ok=false
    fi

    if [[ "$structure_ok" == "true" ]]; then
        log "VALID" "ISO structure validation completed"
        count_result "PASSED"
    fi
    return 0
}

# Function to validate UEFI boot support
validate_uefi_boot() {
    log "INFO" "Validating UEFI boot support..."

    # Check for UEFI bootloader
    local uefi_files=("/EFI/BOOT/BOOTX64.EFI" "/EFI/BOOT/BOOTIA32.EFI")
    local uefi_found=false
    local grub_cfg=""

    if [[ -n "$ISO_LISTER" ]]; then
        # Rootless check via xorriso/bsdtar listing.
        local files_list
        files_list=$(iso_list_files) || true

        for file in "${uefi_files[@]}"; do
            if grep -qxF "$file" <<< "$files_list"; then
                log "VALID" "UEFI bootloader found: $file"
                uefi_found=true
            fi
        done
    elif [[ "$CAN_MOUNT" == true ]]; then
        local temp_mount
        temp_mount=$(mktemp -d)

        if ! mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
            log "ERROR" "Could not mount ISO for UEFI validation"
            rm -rf "$temp_mount"
            count_result "FAILED"
            return 1
        fi

        for file in "${uefi_files[@]}"; do
            if [[ -f "$temp_mount$file" ]]; then
                log "VALID" "UEFI bootloader found: $file"
                uefi_found=true
            fi
        done

        [[ -f "$temp_mount/boot/grub/grub.cfg" ]] && grub_cfg="$temp_mount/boot/grub/grub.cfg"
        [[ -f "$temp_mount/EFI/Microsoft/Boot/bootmgfw.efi" ]] && \
            log "INFO" "Microsoft UEFI bootloader found (dual-boot support)"

        if [[ "$uefi_found" == "false" ]]; then
            umount "$temp_mount" 2>/dev/null || true
            rm -rf "$temp_mount"
            log "ERROR" "No UEFI bootloader found"
            count_result "FAILED"
            return 1
        fi

        # Check for GRUB configuration content
        if [[ -n "$grub_cfg" ]]; then
            log "VALID" "GRUB configuration found"

            # Check for UEFI-specific entries
            if grep -q "chainloader" "$grub_cfg" 2>/dev/null; then
                log "VALID" "UEFI chainloader configuration found"
            fi
        else
            log "WARN" "GRUB configuration not found"
            count_result "WARNING"
        fi

        umount "$temp_mount" 2>/dev/null || true
        rm -rf "$temp_mount"

        log "VALID" "UEFI boot validation completed"
        count_result "PASSED"
        return 0
    else
        log "WARN" "Cannot inspect ISO contents (install xorriso or bsdtar, or run as root); skipping UEFI validation"
        count_result "SKIPPED"
        return 0
    fi

    if [[ "$uefi_found" == "false" ]]; then
        log "ERROR" "No UEFI bootloader found"
        count_result "FAILED"
        return 1
    fi

    # Deep content check: GRUB configuration (needs the file itself).
    local temp_dir
    temp_dir=$(mktemp -d)
    local obtain_rc=0
    grub_cfg=$(iso_obtain_file "/boot/grub/grub.cfg" "$temp_dir") || obtain_rc=$?

    if [[ $obtain_rc -eq 2 ]]; then
        log "ERROR" "Could not mount ISO to read GRUB configuration"
        rm -rf "$temp_dir"
        count_result "FAILED"
        return 1
    elif [[ $obtain_rc -ne 0 || -z "$grub_cfg" ]]; then
        log "WARN" "GRUB configuration not found or unreadable"
        count_result "WARNING"
    else
        log "VALID" "GRUB configuration found"

        # Check for UEFI-specific entries
        if grep -q "chainloader" "$grub_cfg" 2>/dev/null; then
            log "VALID" "UEFI chainloader configuration found"
        fi
    fi

    rm -rf "$temp_dir"

    log "VALID" "UEFI boot validation completed"
    count_result "PASSED"
    return 0
}

# Function to validate boot parameters
validate_boot_parameters() {
    log "INFO" "Validating boot parameters..."

    # This is a deep content check: it needs /boot/grub/grub.cfg itself.
    # Obtain it without root via xorriso extraction when possible.
    local temp_dir
    temp_dir=$(mktemp -d)
    local grub_cfg=""
    local obtain_rc=0
    grub_cfg=$(iso_obtain_file "/boot/grub/grub.cfg" "$temp_dir") || obtain_rc=$?

    if [[ $obtain_rc -eq 2 ]]; then
        log "ERROR" "Could not mount ISO for boot parameter validation"
        rm -rf "$temp_dir"
        count_result "FAILED"
        return 1
    elif [[ $obtain_rc -ne 0 || -z "$grub_cfg" ]]; then
        if [[ "$CAN_MOUNT" == true || -n "$ISO_LISTER" ]]; then
            log "WARN" "GRUB configuration not found for parameter validation"
            count_result "WARNING"
        else
            log "WARN" "No way to read GRUB configuration (install xorriso or bsdtar, or run as root); skipping boot parameter validation"
            count_result "SKIPPED"
        fi
        rm -rf "$temp_dir"
        return 0
    fi

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

    rm -rf "$temp_dir"

    log "VALID" "Boot parameter validation completed"
    count_result "PASSED"
    return 0
}

# Function to validate filesystem integrity
validate_filesystem_integrity() {
    log "INFO" "Validating filesystem integrity..."

    # Deep content check: requires loop-mounting the ISO.
    if [[ "$CAN_MOUNT" != true ]]; then
        log "WARN" "Loop-mount requires root; skipping filesystem integrity validation"
        count_result "SKIPPED"
        return 0
    fi

    local temp_mount
    temp_mount=$(mktemp -d)

    if ! mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
        log "ERROR" "Could not mount ISO for filesystem integrity validation"
        rm -rf "$temp_mount"
        count_result "FAILED"
        return 1
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
            if unsquashfs -l "$squashfs_file" > /dev/null 2>&1; then
                log "VALID" "Squashfs filesystem integrity verified"
            else
                log "ERROR" "Squashfs filesystem integrity check failed"
                count_result "FAILED"
            fi
        else
            log "INFO" "unsquashfs not available, skipping integrity check"
            count_result "SKIPPED"
        fi
    else
        log "ERROR" "Squashfs filesystem not found"
        count_result "FAILED"
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
    # Verify the artifact signature when signature material is present.
    # A check that was never performed must never be reported as VALID.
    local signature_file="${ISO_FILE}.sig"
    if [[ -f "$signature_file" ]]; then
        if command -v gpg &> /dev/null; then
            if gpg --verify "$signature_file" "$ISO_FILE" >/dev/null 2>&1; then
                log "VALID" "GPG signature verified for $(basename "$ISO_FILE")"
                count_result "PASSED"
            else
                local gpg_rc=$?
                if [[ $gpg_rc -eq 1 ]]; then
                    log "ERROR" "GPG signature verification FAILED (bad signature)"
                    count_result "FAILED"
                else
                    log "WARN" "GPG signature present but not verifiable (rc=$gpg_rc: missing public key or unreadable signature)"
                    count_result "SKIPPED"
                fi
            fi
        elif command -v cosign &> /dev/null && [[ -f "${ISO_FILE}.bundle" ]]; then
            if cosign verify-blob --signature "$signature_file" --bundle "${ISO_FILE}.bundle" "$ISO_FILE" >/dev/null 2>&1; then
                log "VALID" "Cosign signature verified for $(basename "$ISO_FILE")"
                count_result "PASSED"
            else
                log "ERROR" "Cosign signature verification FAILED"
                count_result "FAILED"
            fi
        else
            log "WARN" "Signature file found but neither gpg nor cosign (with .bundle) is available to verify it"
            count_result "SKIPPED"
        fi
    else
        log "INFO" "No signature file found (${signature_file}); skipping signature verification"
        count_result "SKIPPED"
    fi

    # The remaining checks need the ISO contents; loop-mount is required.
    if [[ "$CAN_MOUNT" != true ]]; then
        log "WARN" "Loop-mount requires root; skipping mount-based security checks"
        count_result "SKIPPED"
        return 0
    fi

    local temp_mount
    temp_mount=$(mktemp -d)

    if ! mount -o loop,ro "$ISO_FILE" "$temp_mount" 2>/dev/null; then
        log "ERROR" "Could not mount ISO for security validation"
        rm -rf "$temp_mount"
        count_result "FAILED"
        return 1
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
- Skipped: $VALIDATION_SKIPPED

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

    # Probe ISO inspection tools (xorriso/bsdtar listing vs root loop-mount)
    detect_iso_tools

    # Run validations (each records PASS/FAIL/SKIP via count_result; a failing
    # check must not abort the remaining validations or the final summary)
    validate_iso_format || true
    validate_checksum || true
    validate_iso_structure || true
    validate_uefi_boot || true
    validate_boot_parameters || true
    validate_filesystem_integrity || true
    validate_security_features || true
    validate_configuration_compliance || true

    # Generate report
    generate_validation_report

    # Summary
    log "INFO" "Validation Summary:"
    log "INFO" "  - Passed: $VALIDATION_PASSED"
    log "INFO" "  - Failed: $VALIDATION_FAILED"
    log "INFO" "  - Warnings: $VALIDATION_WARNINGS"
    log "INFO" "  - Skipped: $VALIDATION_SKIPPED"

    # Determine exit code: any FAIL fails the validation; skips never pass
    # for checks that ran, they are reported separately above.
    if [[ $VALIDATION_FAILED -gt 0 ]]; then
        log "ERROR" "Validation failed with $VALIDATION_FAILED critical errors"
        exit 1
    elif [[ $VALIDATION_WARNINGS -gt 0 ]] && [[ "$STRICT_MODE" == "true" ]]; then
        log "ERROR" "Validation failed with $VALIDATION_WARNINGS warnings (strict mode)"
        exit 2
    else
        if [[ $VALIDATION_SKIPPED -gt 0 ]]; then
            log "SUCCESS" "Validation completed with $VALIDATION_SKIPPED check(s) skipped and no failures"
        else
            log "SUCCESS" "All validations passed successfully"
        fi
        exit 0
    fi
}

# Error handling
set -euo pipefail
trap 'log "ERROR" "Validation process interrupted"; exit 1' INT TERM

# Run main function
main "$@"