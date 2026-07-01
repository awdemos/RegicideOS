#!/bin/bash
# Stage 5: install RegicideOS tools.
set -euo pipefail

source "$(dirname "$0")/common.sh"

REGICIDE_PACKAGES=(
    regicide-tools/btrmind
    regicide-tools/regicide-installer
)

for pkg in "${REGICIDE_PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

echo "Stage 5 complete."
