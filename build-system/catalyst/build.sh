#!/bin/bash
set -euo pipefail

CATALYST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REGICIDE_DIR="$(cd "${CATALYST_DIR}/../.." && pwd)"
SPEC="${CATALYST_DIR}/stage4-systemd-cosmic.spec"
FSSCRIPT="${CATALYST_DIR}/stage4-systemd-cosmic.sh"

if [[ $EUID -ne 0 ]]; then
    echo "This script must be run as root (Catalyst requires root)"
    exit 1
fi

if ! command -v catalyst &> /dev/null; then
    echo "Catalyst not found. Install it first:"
    echo "  emerge -av dev-util/catalyst"
    exit 1
fi

CATALYST_TMP="/var/tmp/catalyst"
mkdir -p "${CATALYST_TMP}/config/stages"
mkdir -p "${CATALYST_TMP}/builds/default"
mkdir -p "${CATALYST_TMP}/tmp"

ln -sf "${CATALYST_DIR}/overlay" "${CATALYST_TMP}/config/stages/overlay" 2>/dev/null || true
ln -sf "${CATALYST_DIR}/cosmic-overlay" "${CATALYST_TMP}/config/stages/cosmic-overlay" 2>/dev/null || true

if [[ -f /tmp/snapshot_hash ]]; then
    SNAPSHOT_HASH=$(cat /tmp/snapshot_hash)
else
    SNAPSHOT_HASH=$(date +%Y%m%d)
fi
sed -e "s|\[CATALYST_DIR\]|${CATALYST_DIR}/|g" \
    -e "s|\[SNAPSHOT_HASH\]|${SNAPSHOT_HASH}|g" \
    "${SPEC}" > "${CATALYST_TMP}/config/stages/stage4-systemd-cosmic.spec"

echo "Building RegicideOS COSMIC stage4..."
echo "  Spec: ${CATALYST_TMP}/config/stages/stage4-systemd-cosmic.spec"
echo "  Output: ${CATALYST_TMP}/builds/default/"

catalyst -a -f "${CATALYST_TMP}/config/stages/stage4-systemd-cosmic.spec"

TARBALL=$(ls -t "${CATALYST_TMP}/builds/default/"stage4-amd64-systemd-cosmic-*.tar.xz 2>/dev/null | head -n1)
if [[ -f "${TARBALL}" ]]; then
    # Provide a stable filename for downstream tooling (e.g. Dagger pipelines).
    ln -sf "$(basename "${TARBALL}")" "${CATALYST_TMP}/builds/default/stage4-amd64-systemd-cosmic.tar.xz"

    echo ""
    echo "Stage4 tarball complete: ${TARBALL}"
    echo ""
    echo "Creating SquashFS image..."

    IMG_DIR="${REGICIDE_DIR}/build-system/catalyst/output"
    mkdir -p "${IMG_DIR}"
    
    ROOT_DIR=$(mktemp -d)
    tar -C "${ROOT_DIR}" -xpJf "${TARBALL}"
    
    IMG_PATH="${IMG_DIR}/regicide-cosmic.img"
    mksquashfs "${ROOT_DIR}" "${IMG_PATH}" -comp zstd -Xcompression-level 19 -noappend
    rm -rf "${ROOT_DIR}"
    
    echo ""
    echo "SquashFS image created: ${IMG_PATH}"
    echo ""
    echo "To install to a drive:"
    echo "  ./target/release/installer --image ${IMG_PATH} /dev/sdX"
    echo ""
    echo "To deploy to ROOTS partition manually:"
    echo "  cp ${IMG_PATH} /mnt/roots/"
else
    echo "Build may have failed. Check logs:"
    echo "  ${CATALYST_TMP}/logs/"
    exit 1
fi
