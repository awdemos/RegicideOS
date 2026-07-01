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
    bwrap \
        --bind "${ROOTFS}" / \
        --proc /proc \
        --dev /dev \
        --ro-bind /sys /sys \
        --tmpfs /tmp \
        --bind "${ROOTFS}/var/tmp/portage" /var/tmp/portage \
        --bind "${ROOTFS}/var/cache/distfiles" /var/cache/distfiles \
        --bind "${ROOTFS}/var/cache/binpkgs" /var/cache/binpkgs \
        --bind "${ROOTFS}/var/db/repos/gentoo" /var/db/repos/gentoo \
        --ro-bind /etc/resolv.conf /etc/resolv.conf \
        --unshare-uts \
        --unshare-ipc \
        --unshare-pid \
        --unshare-cgroup \
        --share-net \
        "$@"
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
