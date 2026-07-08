#!/bin/bash
# Seed overlay upperdir directory trees from a read-only ROOTS mount.
# This is used after regicide-image installs a new ROOTS tarball so that
# overlayfs does not have to copy-up parent directories at runtime.
# Gentoo variant: only /etc and /var are mounted from the OVERLAY partition.
set -euo pipefail

SRC_ROOT="${1:-/roots}"
OVERLAY_ROOT="${2:-/overlay}"

for dir in etc var; do
    if [[ ! -d "${SRC_ROOT}/${dir}" ]]; then
        continue
    fi
    echo "Seeding directory tree for /${dir} ..."
    list="/tmp/${dir}_dirs.txt"
    cd "${SRC_ROOT}/${dir}"
    find . -type d -print0 > "${list}"
    cd "${OVERLAY_ROOT}"
    while IFS= read -r -d '' d; do
        mkdir -p "${OVERLAY_ROOT}/${dir}/${d}"
    done < "${list}"
    while IFS= read -r -d '' d; do
        src="${SRC_ROOT}/${dir}/${d}"
        dst="${OVERLAY_ROOT}/${dir}/${d}"
        if [[ -L "${src}" ]]; then
            rm -f "${dst}"
            ln -sfn "$(readlink "${src}")" "${dst}"
            continue
        fi
        chown "$(stat -c %u:%g "${src}")" "${dst}" 2>/dev/null || true
        chmod "$(stat -c %a "${src}")" "${dst}" 2>/dev/null || true
    done < "${list}"
done

mkdir -p "${OVERLAY_ROOT}/etcw" "${OVERLAY_ROOT}/varw"
chmod 755 "${OVERLAY_ROOT}/etcw" "${OVERLAY_ROOT}/varw"

echo "Done seeding overlay directory trees."
