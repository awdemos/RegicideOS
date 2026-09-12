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
    -f, --force         Skip the confirmation prompt and the USB-transport
                        refusal (use only for unattended flashing)
    -i, --iso PATH      Use a different ISO file
    -n, --no-verify     Skip post-write verification

EXAMPLES:
    $0 /dev/sdX                  # Flash to /dev/sdX
    $0 --force /dev/sdX          # Flash without confirmation
    $0 --iso ~/Downloads/regicide-cosmic-amd64.iso /dev/sdX

WARNING: The target device will be completely overwritten.
EOF
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

# Refuse non-USB targets unless forced. lsblk reports a transport (TRAN) for
# devices it can classify; when TRAN is present it must be "usb". --force is
# the single override: it skips this refusal and the typed confirmation.
DEVICE_CANONICAL="$(readlink -f "$DEVICE")"
DEVICE_TRAN="$(lsblk -dn -o TRAN "$DEVICE" 2>/dev/null | head -n1 || true)"
if [[ -n "$DEVICE_TRAN" && "$DEVICE_TRAN" != "usb" ]]; then
    if [[ "$FORCE" != true ]]; then
        regicide_error "$DEVICE has transport '$DEVICE_TRAN' (not usb). Refusing to write."
        regicide_error "Re-run with --force only if you are certain this is the intended target."
        exit 1
    fi
    regicide_warn "$DEVICE has transport '$DEVICE_TRAN' (not usb); proceeding because --force was given"
elif [[ -z "$DEVICE_TRAN" ]]; then
    regicide_warn "Could not determine the transport type of $DEVICE; make sure it is the intended USB target"
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

# Final confirmation: show the resolved paths and require typing the device
# path so the write cannot proceed on a stray keypress.
if [[ "$FORCE" != true ]]; then
    echo
    regicide_error "ALL DATA ON $DEVICE WILL BE DESTROYED"
    regicide_log "ISO:    $ISO_PATH"
    regicide_log "Device: $DEVICE_CANONICAL"
    printf '%bType the device path (%s) to continue: %b' "${REGICIDE_YELLOW}" "$DEVICE" "${REGICIDE_NC}"
    TYPED_DEVICE=""
    read -r TYPED_DEVICE || TYPED_DEVICE=""
    if [[ "$TYPED_DEVICE" != "$DEVICE" && "$TYPED_DEVICE" != "$DEVICE_CANONICAL" ]]; then
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
    READ_BLOCKS=$(( (ISO_SIZE + 4194303) / 4194304 ))
    ISO_SHA_EXPECTED=$(sha256sum "$ISO_PATH" | awk '{print $1}')

    # Read back bypassing the page cache: a cached read can be satisfied from
    # RAM and would not detect a bad flash. Prefer O_DIRECT on the device;
    # fall back to dropping caches; as a last resort read plainly but say so.
    ISO_SHA_WRITTEN=""
    if ! ISO_SHA_WRITTEN=$(dd if="$DEVICE" bs=4M count="$READ_BLOCKS" iflag=direct status=none 2>/dev/null | sha256sum | awk '{print $1}'); then
        regicide_warn "Direct I/O read-back failed; dropping caches instead"
        sync
        if echo 3 > /proc/sys/vm/drop_caches 2>/dev/null; then
            if ! ISO_SHA_WRITTEN=$(dd if="$DEVICE" bs=4M count="$READ_BLOCKS" status=none 2>/dev/null | sha256sum | awk '{print $1}'); then
                regicide_error "Could not read back from $DEVICE"
                exit 1
            fi
        else
            regicide_warn "Cannot drop caches (requires root); verification read may be satisfied from the page cache"
            if ! ISO_SHA_WRITTEN=$(dd if="$DEVICE" bs=4M count="$READ_BLOCKS" status=none 2>/dev/null | sha256sum | awk '{print $1}'); then
                regicide_error "Could not read back from $DEVICE"
                exit 1
            fi
        fi
    fi

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
