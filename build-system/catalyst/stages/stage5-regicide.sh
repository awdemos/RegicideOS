#!/bin/bash
# Stage 5: install RegicideOS tools.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage5-regicide"

log_status "start" "installing RegicideOS tools"

# The regicide-rust overlay ships live 9999 ebuilds with empty KEYWORDS.
# Allow them to be emerged without a full overlay-wide keyword bump.
mkdir -p "${ROOTFS}/etc/portage/package.accept_keywords"
cat > "${ROOTFS}/etc/portage/package.accept_keywords/regicide" << 'EOF'
regicide-tools/* **
EOF

REGICIDE_PACKAGES=(
    regicide-tools/regicide-installer
    app-misc/fastfetch
)

for pkg in "${REGICIDE_PACKAGES[@]}"; do
    echo "Installing ${pkg}..."
    log_status "package" "${pkg}"
    run_in_chroot emerge -q "$pkg"
done

# btrfs-assistant is optional: upstream 2.2 often fails with newer Qt6/gcc.
if run_in_chroot emerge -q sys-fs/btrfs-assistant; then
    log_status "package" "sys-fs/btrfs-assistant"
else
    echo "WARNING: sys-fs/btrfs-assistant failed to build; continuing without it" >&2
    log_status "warning" "sys-fs/btrfs-assistant failed to build; continuing"
fi

clean_rootfs_transient
log_status "complete" "RegicideOS tools installed"
echo "Stage 5 complete."
