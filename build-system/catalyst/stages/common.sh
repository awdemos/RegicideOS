#!/bin/bash
# Common helpers used by all RegicideOS stage scripts.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STAGES_DIR="${SCRIPT_DIR}"
CATALYST_DIR="${SCRIPT_DIR}/.."

REGICIDE_BUILD_DIR="${REGICIDE_BUILD_DIR:-${BUILD_DIR:-${CATALYST_DIR}/tmp}}"
ROOTFS="${REGICIDE_BUILD_DIR}/rootfs"
STAGE3_DIR="${REGICIDE_BUILD_DIR}/stage3"
PORTAGE_SNAPSHOT="${REGICIDE_BUILD_DIR}/portage-latest.tar.xz"
OUTPUT_DIR="${REGICIDE_OUTPUT_DIR:-${OUTPUT_DIR:-${CATALYST_DIR}/output}}"

export ROOTFS
export REGICIDE_BUILD_DIR
export OUTPUT_DIR

run_in_chroot() {
    local -a bwrap_args=(
        --bind "${ROOTFS}" /
        --proc /proc
        --dev /dev
        --ro-bind /sys /sys
        --tmpfs /tmp
        --bind "${ROOTFS}/var/tmp/portage" /var/tmp/portage
        --bind "${ROOTFS}/var/cache/distfiles" /var/cache/distfiles
        --bind "${ROOTFS}/var/cache/binpkgs" /var/cache/binpkgs
        --bind "${ROOTFS}/var/db/repos/gentoo" /var/db/repos/gentoo
        --ro-bind /etc/resolv.conf /etc/resolv.conf
        --unshare-uts
        --unshare-ipc
        --unshare-pid
        --unshare-cgroup
        --share-net
    )
    if [[ -d "${ROOTFS}/var/db/repos/cosmic-overlay" ]]; then
        bwrap_args+=(--bind "${ROOTFS}/var/db/repos/cosmic-overlay" /var/db/repos/cosmic-overlay)
    fi
    if [[ -d "${ROOTFS}/var/db/repos/regicide-rust" ]]; then
        bwrap_args+=(--bind "${ROOTFS}/var/db/repos/regicide-rust" /var/db/repos/regicide-rust)
    fi
    bwrap "${bwrap_args[@]}" "$@"
}

ensure_dirs() {
    mkdir -p "${ROOTFS}"/var/db/repos/gentoo
    mkdir -p "${ROOTFS}"/var/lib/portage
    mkdir -p "${ROOTFS}"/var/tmp/portage
    mkdir -p "${ROOTFS}"/var/cache/distfiles
    mkdir -p "${ROOTFS}"/var/cache/binpkgs
    mkdir -p "${ROOTFS}"/overlay
    mkdir -p "${ROOTFS}"/roots
    mkdir -p "${ROOTFS}"/home
    mkdir -p "${ROOTFS}"/boot
    mkdir -p "${ROOTFS}"/recovery/etc
    mkdir -p "${ROOTFS}"/recovery/home/recovery
    mkdir -p "${OUTPUT_DIR}"
}

log_status() {
    local event="${1:-info}"
    local detail="${2:-}"
    local stage_name="${STAGE_NAME:-unknown}"
    local status_dir="${CATALYST_DIR}/output"
    local status_file="${status_dir}/build-status.jsonl"
    mkdir -p "${status_dir}"
    printf '{"time":"%s","stage":"%s","event":"%s","detail":"%s"}\n' \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        "${stage_name}" \
        "${event}" \
        "${detail}" >> "${status_file}"
}

clean_rootfs_transient() {
    echo "Cleaning transient build data to keep rootfs cache small..."
    rm -rf "${ROOTFS}/var/tmp/portage"/* "${ROOTFS}/var/tmp/portage"/.*[!.]* 2>/dev/null || true
    rm -rf "${ROOTFS}/tmp"/* "${ROOTFS}/tmp"/.*[!.]* 2>/dev/null || true
    rm -rf "${ROOTFS}/var/cache/edb" 2>/dev/null || true
    find "${ROOTFS}/var/log" -type f -delete 2>/dev/null || true
}
