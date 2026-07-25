#!/bin/bash
# RegicideOS in-VM image builder
# This script runs inside a KVM appliance that boots the stage4 rootfs.
# It mounts the data disk, locates the stage4 rootfs archive and optional LUKS
# passphrase, then invokes build-qemu-image.sh in direct-device mode against
# /dev/vda.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine guest architecture. The stage4 tarball filename encodes the arch
# (stage4-amd64-* or stage4-arm64-*), or REGICIDE_ARCH can be set explicitly.
REGICIDE_ARCH="${REGICIDE_ARCH:-}"
if [[ -z "${REGICIDE_ARCH}" ]]; then
    if [[ "$(basename "${TARBALL}")" == *arm64* ]]; then
        REGICIDE_ARCH="arm64"
    else
        REGICIDE_ARCH="amd64"
    fi
fi

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

# Locate an optional passphrase file (used when encrypting). Prefer the
# ram-backed fw_cfg copy provided by the host initramfs, falling back to the
# legacy data-disk copy for compatibility.
PASSPHRASE_FILE=""
ENCRYPT_FLAG=""
if [[ -f "/run/regicide-luks-passphrase" ]]; then
    PASSPHRASE_FILE="/run/regicide-luks-passphrase"
    ENCRYPT_FLAG="--encrypt"
elif [[ -f "${DATA_DIR}/luks-passphrase" ]]; then
    PASSPHRASE_FILE="${DATA_DIR}/luks-passphrase"
    ENCRYPT_FLAG="--encrypt"
fi

# The target disk is attached as /dev/vda.
TARGET="/dev/vda"
OUTPUT="/run/regicide-output/regicide-cosmic.qcow2"
DISK_SIZE="30G"
if [[ -f "${DATA_DIR}/disk-size" ]]; then
    DISK_SIZE="$(cat "${DATA_DIR}/disk-size")"
fi

mkdir -p /run/regicide-output

# Invoke the block-device builder.  It expects to run as root (which we are
# inside the VM) and operates directly on the target disk.
if [[ "${REGICIDE_ARCH}" == "arm64" ]]; then
    BUILDER="${SCRIPT_DIR}/build-qemu-image-arm64.sh"
else
    BUILDER="${SCRIPT_DIR}/build-qemu-image.sh"
fi
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
