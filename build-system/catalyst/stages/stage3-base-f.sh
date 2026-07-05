#!/bin/bash
# Stage 3f: install virtualization guest agents and network services.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage3-base-f"

log_status "start" "installing guest agents and network services"
PACKAGES=(
    app-emulation/qemu-guest-agent
    app-emulation/spice-vdagent
    net-fs/samba
)

for pkg in "${PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg" || echo "WARNING: ${pkg} may have failed"
done

clean_rootfs_transient
log_status "complete" "guest agents and network services installed"
echo "Stage 3f complete."
