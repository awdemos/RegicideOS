#!/bin/bash
# RegicideOS QEMU Runner
# Launches a RegicideOS VM from a QCOW2 disk image.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

IMAGE="${1:-}"
if [[ -z "${IMAGE}" ]]; then
    # Look for a QCOW2 image in the same directory
    IMAGE=$(find "${SCRIPT_DIR}" -maxdepth 1 -name '*.qcow2' -type f | head -n1 || true)
    if [[ -z "${IMAGE}" ]]; then
        echo "Usage: $0 <path-to-qcow2-image>"
        echo ""
        echo "Or place a .qcow2 file in the same directory as this script."
        exit 1
    fi
fi

IMAGE="$(realpath -e "${IMAGE}")"

# Pick a free local SSH forwarding port.  A fixed port collides when another
# RegicideOS VM (e.g. RegicideOSArch) is already running on the host.
find_free_port() {
    local port
    for port in $(seq 2222 2999); do
        if ! (command -v ss >/dev/null 2>&1 && ss -Htn "sport = :${port}" | grep -q .) && \
           ! (command -v netstat >/dev/null 2>&1 && netstat -atn 2>/dev/null | grep -q ":${port} ") && \
           ! (timeout 1 bash -c "exec 3<>/dev/tcp/127.0.0.1/${port}" 2>/dev/null); then
            echo "${port}"
            return 0
        fi
    done
    echo "ERROR: no free TCP port found in range 2222-2999" >&2
    return 1
}
SSH_PORT="${REGICIDE_VM_SSH_PORT:-$(find_free_port)}"

echo "Starting RegicideOS QEMU VM..."
echo "  Image:  ${IMAGE}"
echo "  Memory: 4G"
echo "  CPUs:   2"
echo "  SSH:    localhost:${SSH_PORT} -> :22"
echo ""
echo "To connect via SSH:  ssh -p ${SSH_PORT} root@localhost"
echo "To stop:            Ctrl+A then X (in -nographic) or close window"
echo ""

# Check for KVM acceleration
KVM_FLAGS=""
if [[ -e /dev/kvm ]] && [[ -r /dev/kvm ]]; then
    KVM_FLAGS="-enable-kvm -cpu host"
    echo "KVM acceleration enabled."
else
    echo "Warning: KVM not available. VM will be slow."
fi

# Check for OVMF firmware
OVMF_CODE=""
OVMF_VARS=""
for path in \
    /usr/share/OVMF/OVMF_CODE.fd \
    /usr/share/edk2/ovmf/OVMF_CODE.fd \
    /usr/share/qemu/OVMF_CODE.fd \
    /usr/share/ovmf/x64/OVMF_CODE.fd
do
    if [[ -f "${path}" ]]; then
        OVMF_CODE="${path}"
        break
    fi
done

for path in \
    /usr/share/OVMF/OVMF_VARS.fd \
    /usr/share/edk2/ovmf/OVMF_VARS.fd \
    /usr/share/qemu/OVMF_VARS.fd \
    /usr/share/ovmf/x64/OVMF_VARS.fd
do
    if [[ -f "${path}" ]]; then
        OVMF_VARS="${path}"
        break
    fi
done

if [[ -z "${OVMF_CODE}" ]]; then
    echo "Error: OVMF firmware not found. Install ovmf or edk2-ovmf."
    exit 1
fi

UEFI_FLAGS="-drive if=pflash,format=raw,readonly=on,file=${OVMF_CODE}"
if [[ -n "${OVMF_VARS}" ]]; then
    # Use a temporary copy of OVMF_VARS so each run is independent
    TMP_VARS=$(mktemp --suffix=_OVMF_VARS.fd)
    cp "${OVMF_VARS}" "${TMP_VARS}"
    UEFI_FLAGS="${UEFI_FLAGS} -drive if=pflash,format=raw,file=${TMP_VARS}"
fi

qemu-system-x86_64 \
    ${KVM_FLAGS} \
    -m 4G \
    -smp 2 \
    -drive file="${IMAGE}",format=qcow2,if=virtio \
    -netdev "user,id=net0,hostfwd=tcp::${SSH_PORT}-:22" \
    -device virtio-net-pci,netdev=net0 \
    -vga virtio \
    -display sdl,gl=on \
    -machine type=q35 \
    ${UEFI_FLAGS} \
    "$@"
