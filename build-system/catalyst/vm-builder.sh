#!/bin/bash
# RegicideOS in-VM image builder
# This script runs inside a KVM appliance that boots the stage4 rootfs.
# It mounts the data disk, locates the stage4 rootfs archive and optional LUKS
# passphrase, then invokes build-qemu-image.sh in direct-device mode against
# /dev/vda.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# /data is mounted by the host initramfs overlay as a SquashFS and contains
# the stage4 archive, optional passphrase file, and the in-VM builder script.
DATA_DIR="/data"

# Locate the stage4 rootfs archive.  The wrapper may place either a legacy
# .tar.xz tarball or the stage4 SquashFS image on the data disk.
TARBALL=""
for ext in .tar.xz .tar.zst .tzst .img .squashfs; do
    CANDIDATE="$(find "${DATA_DIR}" -maxdepth 1 -name "*${ext}" -type f | head -n1 || true)"
    if [[ -n "${CANDIDATE}" && -f "${CANDIDATE}" ]]; then
        TARBALL="${CANDIDATE}"
        break
    fi
done
if [[ -z "${TARBALL}" || ! -f "${TARBALL}" ]]; then
    echo "Error: stage4 archive not found on data disk (looked for *.tar.xz, *.img, *.squashfs)."
    exit 1
fi

# Locate an optional passphrase file (used when encrypting).
PASSPHRASE_FILE=""
ENCRYPT_FLAG=""
CANDIDATE="$(find "${DATA_DIR}" -maxdepth 1 -name 'luks-passphrase*' -type f | head -n1 || true)"
if [[ -n "${CANDIDATE}" ]]; then
    PASSPHRASE_FILE="${CANDIDATE}"
    ENCRYPT_FLAG="--encrypt"
fi

# The target disk is attached as /dev/vda.
TARGET="/dev/vda"
OUTPUT="/run/regicide-output/regicide-cosmic.qcow2"
DISK_SIZE="20G"

mkdir -p /run/regicide-output

# Invoke the block-device builder.  It expects to run as root (which we are
# inside the VM) and operates directly on the target disk.
BUILDER="${SCRIPT_DIR}/build-qemu-image.sh"
if [[ -n "${ENCRYPT_FLAG}" ]]; then
    exec "${BUILDER}" \
        --direct-device "${TARGET}" \
        --no-convert \
        --encrypt \
        --passphrase-file "${PASSPHRASE_FILE}" \
        "${TARBALL}" \
        "${OUTPUT}" \
        "${DISK_SIZE}"
else
    exec "${BUILDER}" \
        --direct-device "${TARGET}" \
        --no-convert \
        "${TARBALL}" \
        "${OUTPUT}" \
        "${DISK_SIZE}"
fi
