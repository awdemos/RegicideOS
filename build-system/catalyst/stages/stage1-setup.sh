#!/bin/bash
# Stage 1: prepare the Gentoo stage3 seed and Portage snapshot.
set -euo pipefail

# shellcheck source=common.sh
source "$(dirname "$0")/common.sh"

for cmd in bwrap curl tar; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "ERROR: required command '$cmd' not found"
        exit 1
    fi
done

mkdir -p "${REGICIDE_BUILD_DIR}" "${ROOTFS}" "${STAGE3_DIR}"

STAGE3_FILE="${STAGE3_DIR}/stage3-amd64-systemd.tar.xz"
if [[ ! -f "${STAGE3_FILE}" ]]; then
    echo "Downloading Gentoo stage3..."
    LATEST_URL="https://distfiles.gentoo.org/releases/amd64/autobuilds/current-stage3-amd64-systemd/latest-stage3-amd64-systemd.txt"
    STAGE3_PATH=$(curl -s "$LATEST_URL" | awk '/\.tar\.xz/{print $1; exit}')
    if [[ -z "${STAGE3_PATH}" ]]; then
        echo "ERROR: Could not determine stage3 download path"
        exit 1
    fi
    curl -o "${STAGE3_FILE}" "https://distfiles.gentoo.org/releases/amd64/autobuilds/current-stage3-amd64-systemd/${STAGE3_PATH}"
    echo "Stage3 downloaded: ${STAGE3_FILE}"
else
    echo "Using existing stage3: ${STAGE3_FILE}"
fi

if [[ -z "$(ls -A "${ROOTFS}" 2>/dev/null)" ]]; then
    echo "Extracting stage3 to rootfs..."
    mkdir -p "${ROOTFS}/dev"
    tar -C "${ROOTFS}" -xJf "${STAGE3_FILE}" \
        --xattrs-include='*.*' --numeric-owner --no-same-permissions \
        --exclude='./dev/*' 2>/dev/null || \
    tar -C "${ROOTFS}" -xJf "${STAGE3_FILE}" \
        --no-same-permissions --exclude='./dev/*'
    echo "Stage3 extracted"
else
    echo "Rootfs already populated, skipping extraction"
fi

ensure_dirs

chown -R "$(id -u):$(id -g)" "${ROOTFS}"
chmod u+w "${ROOTFS}/etc/passwd" "${ROOTFS}/etc/shadow" \
    "${ROOTFS}/etc/group" "${ROOTFS}/etc/gshadow"

sed -i 's/^portage:x:[0-9]\+:/portage:x:0:/' "${ROOTFS}/etc/group"
sed -i 's/^portage:x:[0-9]\+:[0-9]\+:/portage:x:0:0:/' "${ROOTFS}/etc/passwd"

echo "Downloading portage snapshot..."
if [[ ! -f "${PORTAGE_SNAPSHOT}" ]]; then
    curl -o "${PORTAGE_SNAPSHOT}" "https://distfiles.gentoo.org/snapshots/portage-latest.tar.xz"
fi
if [[ -z "$(ls -A "${ROOTFS}/var/db/repos/gentoo" 2>/dev/null)" ]]; then
    tar -C "${ROOTFS}/var/db/repos/gentoo" --strip-components=1 -xJf "${PORTAGE_SNAPSHOT}" --no-same-permissions 2>/dev/null || \
    tar -C "${ROOTFS}/var/db/repos/gentoo" --strip-components=1 -xJf "${PORTAGE_SNAPSHOT}" --no-same-permissions
    echo "Portage snapshot extracted"
else
    echo "Portage tree already present, skipping snapshot extraction"
fi

PROFILE="default/linux/amd64/23.0/desktop/systemd"
echo "Setting profile to ${PROFILE}..."
run_in_chroot eselect profile set "$PROFILE"

echo "Stage 1 complete."
