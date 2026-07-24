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

# Target architecture for the rootfs. amd64 is the default; arm64 builds run
# the same stages under qemu-user binfmt on an x86_64 host.
REGICIDE_ARCH="${REGICIDE_ARCH:-amd64}"
case "${REGICIDE_ARCH}" in
    amd64)
        GENTOO_ARCH="amd64"
        GENTOO_STAGE3_BASENAME="stage3-amd64-systemd"
        GENTOO_PROFILE="default/linux/amd64/23.0/desktop/systemd"
        GENTOO_KEYWORDS="~amd64"
        GENTOO_VIDEO_CARDS="video_cards_amdgpu video_cards_intel video_cards_nouveau video_cards_radeon video_cards_radeonsi video_cards_virgl video_cards_vmware"
        ;;
    arm64)
        GENTOO_ARCH="arm64"
        GENTOO_STAGE3_BASENAME="stage3-arm64-systemd"
        GENTOO_PROFILE="default/linux/arm64/23.0/desktop/systemd"
        GENTOO_KEYWORDS="~arm64"
        GENTOO_VIDEO_CARDS="video_cards_virgl video_cards_vmware"
        ;;
    *)
        echo "ERROR: unsupported REGICIDE_ARCH: ${REGICIDE_ARCH}" >&2
        exit 1
        ;;
esac
export REGICIDE_ARCH GENTOO_ARCH GENTOO_STAGE3_BASENAME GENTOO_PROFILE GENTOO_KEYWORDS GENTOO_VIDEO_CARDS

export ROOTFS
export REGICIDE_BUILD_DIR
export OUTPUT_DIR

# PKGDIR for the chroot. Defaults to a directory inside the rootfs so manual
# host builds keep binpkgs next to the rootfs. The Dagger pipeline sets
# REGICIDE_BINPKGS_DIR to its named binpkgs cache volume so binary packages
# survive Dagger layer-cache invalidation (e.g. overlay bumps or stage script
# changes) and can be reused via --usepkg on the next run.
BINPKGS_DIR="${REGICIDE_BINPKGS_DIR:-${ROOTFS}/var/cache/binpkgs}"
export BINPKGS_DIR

# bwrap cannot create namespaces under qemu-user emulation (cross-arch
# builds): its unshare/clone3 calls fail with EINVAL. Probe once per stage
# run and fall back to a plain chroot (with explicit bind mounts) in that
# case. REGICIDE_CHROOT_METHOD=bwrap|chroot overrides the probe.
_CHROOT_METHOD=""

_detect_chroot_method() {
    if [[ -n "${_CHROOT_METHOD}" ]]; then
        return
    fi
    if [[ -n "${REGICIDE_CHROOT_METHOD:-}" ]]; then
        _CHROOT_METHOD="${REGICIDE_CHROOT_METHOD}"
    elif bwrap --bind "${ROOTFS}" / true 2>/dev/null; then
        _CHROOT_METHOD="bwrap"
    elif [[ "$(id -u)" -eq 0 ]]; then
        echo "run_in_chroot: bwrap namespace probe failed; falling back to chroot" >&2
        _CHROOT_METHOD="chroot"
    else
        echo "ERROR: bwrap cannot create namespaces and chroot needs root." >&2
        exit 1
    fi
}

run_in_chroot_bwrap() {
    local -a bwrap_args=(
        --bind "${ROOTFS}" /
        --proc /proc
        --dev /dev
        --ro-bind /sys /sys
        --tmpfs /tmp
        --bind "${ROOTFS}/var/tmp/portage" /var/tmp/portage
        --bind "${ROOTFS}/var/cache/distfiles" /var/cache/distfiles
        --bind "${BINPKGS_DIR}" /var/cache/binpkgs
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
    # When running unprivileged (e.g. manual builds inside a toolbx
    # container), enter a nested user namespace as root so portage can
    # chown files to portage:portage; without it that uid is unmapped and
    # chown fails with EINVAL. Real-root environments (Dagger) are
    # unaffected.
    if [[ "$(id -u)" -ne 0 ]]; then
        bwrap_args+=(--unshare-user --uid 0 --gid 0)
    fi
    bwrap "${bwrap_args[@]}" "$@"
}

run_in_chroot_chroot() {
    local -a mounted=()
    local ret=0

    mkdir -p "${ROOTFS}"/proc "${ROOTFS}"/dev "${ROOTFS}"/sys "${ROOTFS}"/tmp
    mkdir -p "${ROOTFS}"/etc
    touch "${ROOTFS}"/etc/resolv.conf

    mount -t proc proc "${ROOTFS}/proc" && mounted+=("${ROOTFS}/proc")
    # Recursive bind of the container /dev. A plain --bind does not carry the
    # per-file submounts (/dev/null & co. are individual bind mounts in runc
    # containers), and mknod is blocked even in privileged Dagger execs, so
    # rbind is the only way to give the chroot a sane /dev.
    mount --rbind /dev "${ROOTFS}/dev" && mounted+=("${ROOTFS}/dev")
    mount --rbind /sys "${ROOTFS}/sys" && mount -o remount,ro,bind "${ROOTFS}/sys" && mounted+=("${ROOTFS}/sys")
    mount -t tmpfs tmpfs "${ROOTFS}/tmp" && mounted+=("${ROOTFS}/tmp")
    if [[ "${BINPKGS_DIR}" != "${ROOTFS}/var/cache/binpkgs" ]]; then
        mkdir -p "${ROOTFS}/var/cache/binpkgs"
        mount --bind "${BINPKGS_DIR}" "${ROOTFS}/var/cache/binpkgs" && mounted+=("${ROOTFS}/var/cache/binpkgs")
    fi
    mount --bind /etc/resolv.conf "${ROOTFS}/etc/resolv.conf" && mounted+=("${ROOTFS}/etc/resolv.conf")

    chroot "${ROOTFS}" /usr/bin/env -i \
        HOME=/root \
        TERM="${TERM:-xterm}" \
        PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
        "$@" || ret=$?

    local i
    for (( i=${#mounted[@]}-1 ; i>=0 ; i-- )); do
        umount -l "${mounted[i]}" 2>/dev/null || true
    done
    return "${ret}"
}

run_in_chroot() {
    _detect_chroot_method
    if [[ "${_CHROOT_METHOD}" == "chroot" ]]; then
        run_in_chroot_chroot "$@"
    else
        run_in_chroot_bwrap "$@"
    fi
}

ensure_dirs() {
    mkdir -p "${ROOTFS}"/var/db/repos/gentoo
    mkdir -p "${ROOTFS}"/var/lib/portage
    mkdir -p "${ROOTFS}"/var/tmp/portage
    mkdir -p "${ROOTFS}"/var/cache/distfiles
    mkdir -p "${ROOTFS}"/var/cache/binpkgs
    mkdir -p "${BINPKGS_DIR}"
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
