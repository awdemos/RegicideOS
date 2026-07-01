#!/bin/bash
# RegicideOS Manual Stage4 Build
# Thin wrapper around the six cacheable stage scripts.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STAGES_DIR="${SCRIPT_DIR}/stages"

"${STAGES_DIR}/stage1-setup.sh"
"${STAGES_DIR}/stage2-sync.sh"
"${STAGES_DIR}/stage3-base.sh"
"${STAGES_DIR}/stage4-cosmic.sh"
"${STAGES_DIR}/stage5-regicide.sh"
"${STAGES_DIR}/stage6-finalize.sh"

echo ""
echo "RegicideOS stage4 build complete!"
echo "Next steps:"
echo "  1. Build QEMU image: ./build-qemu-image.sh ${OUTPUT_DIR}/stage4-amd64-systemd-cosmic.tar.xz"
echo "  2. Or create SquashFS: mksquashfs ${ROOTFS} regicide-cosmic.img -comp zstd"
