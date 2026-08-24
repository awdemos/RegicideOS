#!/bin/bash
# RegicideOS ISO Rebuild Script
# Run this on the host (outside Toolbx) to do a fresh source build of the
# USB-flashable RegicideOS COSMIC live ISO.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
VENV_DIR="${ROOT_DIR}/.venv"

# shellcheck source=lib/log.sh
source "${SCRIPT_DIR}/lib/log.sh"
# shellcheck source=lib/container-runtime.sh
source "${SCRIPT_DIR}/lib/container-runtime.sh"

usage() {
    cat <<EOF
Usage: $0

Do a fresh source build of the USB-flashable RegicideOS COSMIC live ISO.
Requires Dagger and a rootful container runtime on the host (rootless Podman
is not supported by the Dagger engine v0.21.8 used by this project).

The script creates a Python venv, installs a matching dagger-io version,
and runs the Dagger pipeline with --iso --skip-sign.
EOF
}

if [[ "${1:-}" == "-h" ]] || [[ "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

find_dagger() {
    # Prefer dagger on PATH, then fall back to common locations and the
    # original user's home when running under sudo.
    if command -v dagger >/dev/null 2>&1; then
        command -v dagger
        return 0
    fi
    if [[ -n "${SUDO_USER:-}" ]]; then
        local user_home
        user_home=$(getent passwd "$SUDO_USER" | cut -d: -f6)
        if [[ -x "${user_home}/.local/bin/dagger" ]]; then
            echo "${user_home}/.local/bin/dagger"
            return 0
        fi
    fi
    for path in /usr/local/bin/dagger /usr/bin/dagger /opt/dagger/bin/dagger; do
        if [[ -x "$path" ]]; then
            echo "$path"
            return 0
        fi
    done
    return 1
}

DAGGER_BIN=$(find_dagger) || {
    regicide_error "dagger CLI not found in PATH or common locations"
    regicide_error "Install it from https://dagger.io/install"
    exit 1
}
regicide_log "Using dagger CLI: $DAGGER_BIN"

DAGGER_VERSION=$($DAGGER_BIN version | awk '{print $2}' | sed 's/^v//')
regicide_log "Dagger CLI version: $DAGGER_VERSION"

# Ensure venv exists with matching dagger-io
if [[ ! -d "$VENV_DIR" ]]; then
    regicide_log "Creating Python venv at $VENV_DIR"
    python3 -m venv "$VENV_DIR"
fi

regicide_log "Activating venv and ensuring dagger-io==$DAGGER_VERSION"
# shellcheck source=/dev/null
source "${VENV_DIR}/bin/activate"
pip install --upgrade pip >/dev/null
pip install --upgrade "dagger-io==${DAGGER_VERSION}" >/dev/null

# Verify import works
python3 -c "import dagger; _ = dagger.Config" || {
    regicide_error "dagger-io import failed; check for conflicting 'dagger' packages"
    exit 1
}

regicide_require_rootful_runtime

regicide_log "Starting fresh RegicideOS ISO build..."
cd "$ROOT_DIR"
DAGGER_PROGRESS=plain "$DAGGER_BIN" run python build-system/dagger_pipeline.py --plain --iso --skip-sign

ISO_PATH="${ROOT_DIR}/build-system/catalyst/output/regicide-cosmic-amd64.iso"
if [[ -f "$ISO_PATH" ]]; then
    regicide_success "Build complete: $ISO_PATH"
    ls -lh "$ISO_PATH"
    if [[ -f "${ISO_PATH}.sha256" ]]; then
        regicide_log "Checksum: ${ISO_PATH}.sha256"
    fi
    echo
    regicide_log "Flash to USB with:"
    echo "  ./scripts/flash-usb.sh /dev/sdX"
else
    regicide_error "ISO not found after build: $ISO_PATH"
    exit 1
fi
