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

sed -e "s|\[CATALYST_DIR\]|${CATALYST_DIR}/|g" \
    -e "s|\[SNAPSHOT_HASH\]|$(date +%Y%m%d)|g" \
    "${SPEC}" > "${CATALYST_TMP}/config/stages/stage4-systemd-cosmic.spec"

echo "Building RegicideOS COSMIC stage4..."
echo "  Spec: ${CATALYST_TMP}/config/stages/stage4-systemd-cosmic.spec"
echo "  Output: ${CATALYST_TMP}/builds/default/"

catalyst -a -f "${CATALYST_TMP}/config/stages/stage4-systemd-cosmic.spec"

TARBALL=$(ls -t "${CATALYST_TMP}/builds/default/"stage4-amd64-systemd-cosmic-*.tar.xz 2>/dev/null | head -n1)
if [[ -f "${TARBALL}" ]]; then
    echo ""
    echo "Build complete: ${TARBALL}"
    echo ""
    echo "To create a SquashFS image for live ISO:"
    echo "  mkdir /tmp/cosmic-root"
    echo "  tar -C /tmp/cosmic-root -xpJf ${TARBALL}"
    echo "  mksquashfs /tmp/cosmic-root regicide-cosmic.img -comp zstd -Xcompression-level 19"
    echo ""
    echo "To deploy to ROOTS partition:"
    echo "  cp regicide-cosmic.img /mnt/roots/"
else
    echo "Build may have failed. Check logs:"
    echo "  ${CATALYST_TMP}/logs/"
    exit 1
fi
