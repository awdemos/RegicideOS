#!/bin/bash
# RegicideOS USB Flash Script
# Safely writes a RegicideOS live ISO to a USB drive.
#
# Usage:
#   ./scripts/flash-usb.sh /dev/sdX
#
# The device will be DESTRUCTIVELY overwritten. All data on it will be lost.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
ISO_DIR="${ROOT_DIR}/build-system/catalyst/output"
ISO_NAME="regicide-cosmic-amd64.iso"
ISO_PATH="${ISO_DIR}/${ISO_NAME}"

# shellcheck source=lib/log.sh
source "${SCRIPT_DIR}/lib/log.sh"

usage() {
    cat <<EOF
Usage: $0 [OPTIONS] <DEVICE>

Write the RegicideOS live ISO to a USB device.

OPTIONS:
    -h, --help          Show this help message
    -f, --force         Skip confirmation prompt
    -i, --iso PATH      Use a different ISO file
    -n, --no-verify     Skip post-write verification

EXAMPLES:
    $0 /dev/sdX                  # Flash to /dev/sdX
    $0 --force /dev/sdX          # Flash without confirmation
    $0 --iso ~/Downloads/regicide-cosmic-amd64.iso /dev/sdX

WARNING: The target device will be completely overwritten.
EOF
}

confirm() {
    local prompt=$1
    local response
    printf '%b%s%b [y/N] ' "${REGICIDE_YELLOW}" "$prompt" "${REGICIDE_NC}"
    read -r response
    [[ "$response" =~ ^[Yy]$ ]]
}

# Defaults
FORCE=false
VERIFY=true

# Parse arguments
DEVICE=""
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -f|--force)
            FORCE=true
            shift
            ;;
        -n|--no-verify)
            VERIFY=false
            shift
            ;;
        -i|--iso)
            ISO_PATH="$2"
            shift 2
            ;;
        -*)
            regicide_error "Unknown option: $1"
            usage
            exit 1
            ;;
        *)
            DEVICE="$1"
            shift
            ;;
    esac
done

if [[ -z "$DEVICE" ]]; then
    regicide_error "No target device specified"
    usage
    exit 1
fi

# Resolve relative paths
ISO_PATH="$(cd "$(dirname "$ISO_PATH")" && pwd)/$(basename "$ISO_PATH")"

regicide_log "RegicideOS USB Flash"
regicide_log "ISO:   $ISO_PATH"
regicide_log "Device: $DEVICE"

# Validate ISO exists
if [[ ! -f "$ISO_PATH" ]]; then
    regicide_error "ISO not found: $ISO_PATH"
    regicide_error "Build or download the ISO first."
    exit 1
fi

# Validate ISO checksum if available
ISO_SHA="${ISO_PATH}.sha256"
if [[ -f "$ISO_SHA" ]]; then
    regicide_log "Verifying ISO checksum..."
    if ! sha256sum -c "$ISO_SHA" >/dev/null 2>&1; then
        regicide_error "ISO checksum validation failed"
        exit 1
    fi
    regicide_success "ISO checksum OK"
else
    regicide_warn "No checksum file found at $ISO_SHA"
fi

# Validate ISO is bootable
if command -v file >/dev/null 2>&1; then
    ISO_TYPE=$(file -b "$ISO_PATH")
    ISO_TYPE_PATTERN='ISO.9660|ISO 9660'
    if [[ ! "$ISO_TYPE" =~ $ISO_TYPE_PATTERN ]]; then
        regicide_error "File does not appear to be an ISO 9660 image: $ISO_PATH"
        regicide_error "file(1) reports: $ISO_TYPE"
        exit 1
    fi
    regicide_success "ISO format OK ($ISO_TYPE)"
fi

# Validate device
if [[ ! -b "$DEVICE" ]]; then
    regicide_error "$DEVICE is not a block device"
    exit 1
fi

# Refuse to write to obvious system disks
DEVICE_CANONICAL=$(readlink -f "$DEVICE")
if [[ "$DEVICE_CANONICAL" == /dev/nvme* ]] || [[ "$DEVICE_CANONICAL" == /dev/sd[a-z] ]]; then
    regicide_warn "$DEVICE looks like a fixed disk. Make sure it is the USB target."
fi

# Show device info
regicide_log "Device information:"
lsblk -o NAME,SIZE,MODEL,VENDOR,TRAN,MOUNTPOINT "$DEVICE" || true

# Show mounted partitions and warn
MOUNTED=$(lsblk -ln -o MOUNTPOINT "$DEVICE" | grep -v '^$' || true)
if [[ -n "$MOUNTED" ]]; then
    regicide_warn "Device has mounted partitions:"
    echo "$MOUNTED"
    regicide_warn "They will be unmounted before writing."
fi

# Final confirmation
if [[ "$FORCE" != true ]]; then
    echo
    regicide_error "ALL DATA ON $DEVICE WILL BE DESTROYED"
    if ! confirm "Are you sure you want to write $ISO_NAME to $DEVICE?"; then
        regicide_log "Aborted."
        exit 1
    fi
fi

# Unmount any mounted partitions on the device
if [[ -n "$MOUNTED" ]]; then
    regicide_log "Unmounting $DEVICE partitions..."
    lsblk -ln -o NAME,MOUNTPOINT "$DEVICE" | awk '/\// {print "/dev/" $1 " " $2}' | \
    while read -r _ mountpoint; do
        if [[ -n "$mountpoint" ]] && mountpoint -q "$mountpoint" 2>/dev/null; then
            regicide_log "Unmounting $mountpoint"
            umount "$mountpoint" || regicide_warn "Failed to unmount $mountpoint"
        fi
    done
fi

# Determine write command (use pv if available for progress)
ISO_SIZE=$(stat -c%s "$ISO_PATH")
ISO_SIZE_MB=$((ISO_SIZE / 1024 / 1024))

regicide_log "Writing ISO (${ISO_SIZE_MB} MiB) to $DEVICE..."
regicide_log "This may take several minutes. Do not remove the drive."

if command -v pv >/dev/null 2>&1; then
    pv -s "$ISO_SIZE" "$ISO_PATH" | dd of="$DEVICE" bs=4M status=none conv=fsync
else
    dd if="$ISO_PATH" of="$DEVICE" bs=4M status=progress conv=fsync
fi

# Ensure all writes are flushed
regicide_log "Syncing..."
sync

regicide_success "ISO written to $DEVICE"

# Post-write verification
if [[ "$VERIFY" == true ]]; then
    regicide_log "Verifying written image..."
    ISO_SHA_WRITTEN=$(dd if="$DEVICE" bs=4M count=$(( (ISO_SIZE + 4194303) / 4194304 )) status=none | sha256sum | awk '{print $1}')
    ISO_SHA_EXPECTED=$(sha256sum "$ISO_PATH" | awk '{print $1}')
    if [[ "$ISO_SHA_WRITTEN" == "$ISO_SHA_EXPECTED" ]]; then
        regicide_success "Post-write verification passed"
    else
        regicide_error "Post-write verification FAILED"
        regicide_error "Expected: $ISO_SHA_EXPECTED"
        regicide_error "Got:      $ISO_SHA_WRITTEN"
        exit 1
    fi
fi

echo
regicide_success "$DEVICE is ready. You can now eject it and boot the target host."
regicide_log "Boot the target host in UEFI mode and select the USB drive."
