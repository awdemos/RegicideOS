#!/usr/bin/env bash
# RegicideOS full local build: stage4 (Gentoo/Catalyst, via Dagger) -> SquashFS.
#
# Site poka-yoke for hosts with rootful Podman (no rootful Docker) and
# NFS-backed home directories. What it wires up, and why:
#
#   - DOCKER_HOST -> rootful Podman socket (regicide-engine runs there):
#       sudo systemctl enable --now podman.socket
#
#   - NFS ($HOME) cannot lchown Gentoo's 'games' (GID 42) files during image
#     unpack (EPERM), so the engine's containerd store is pinned to a local
#     EXT4 path via CONTAINERS_STORAGE_CONF. The storage dir comes from
#     REGICIDE_EXT4_STORAGE_DIR (required; see dagger-ext4-storage.conf).
#
#   - Podman's docker-compat path defaults to PidsLimit=2048; a many-core
#     Gentoo stage build exhausts the cap and fork() fails with EAGAIN
#     ("resource temporarily unavailable"), killing e.g. dev-lang/go in
#     stage3-base-c. dagger-podman-containers.conf removes the cap.
#
#   - REGICIDE_SKIP_ROOTLESS_CHECK=1: the pipeline's rootless-Podman guard
#     misfires when pointed at a rootful Podman socket via DOCKER_HOST
#     ("rootless" is reported for the unprivileged *client* user even though
#     the engine side is rootful).
#
#   - --skip-sign: no Sigstore credentials on local workstations; artifacts
#     are left unsigned. Pass pipeline flags through as-is, e.g.:
#       ./build-system/catalyst/run-dagger-build-local.sh --from-squashfs FILE
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${REPO_ROOT}"

CATALYST_DIR="${REPO_ROOT}/build-system/catalyst"
GENERATED_DIR="${CATALYST_DIR}/generated"

fail() {
    echo "ERROR: $*" >&2
    exit 1
}

# 1. Required: local-FS storage for the Dagger engine containerd store.
#    A wrong (NFS) location corrupts the engine image store on unpack.
STORAGE_DIR="${REGICIDE_EXT4_STORAGE_DIR:-}"
if [[ -z "${STORAGE_DIR}" ]]; then
    fail "REGICIDE_EXT4_STORAGE_DIR is not set.
Set it to a local (non-NFS) filesystem path for the Dagger engine storage, e.g.:
  export REGICIDE_EXT4_STORAGE_DIR=/raid/\$USER/dagger-regicide-storage"
fi

# 2. Rootful Podman socket must exist and be active.
DOCKER_HOST="${DOCKER_HOST:-unix:///run/podman/podman.sock}"
export DOCKER_HOST
SOCKET_PATH="${DOCKER_HOST#unix://}"
if [[ ! -S "${SOCKET_PATH}" ]]; then
    fail "docker endpoint ${DOCKER_HOST} is not a socket.
Start the rootful Podman socket and re-run:
  sudo systemctl enable --now podman.socket"
fi

# 3. Render the storage conf with the real path (placeholders -> values).
mkdir -p "${GENERATED_DIR}"
sed -e "s|@REGICIDE_EXT4_STORAGE_DIR@|${STORAGE_DIR}|g" \
    -e "s|@REGICIDE_RUNUSER_UID@|$(id -u)|g" \
    "${CATALYST_DIR}/dagger-ext4-storage.conf" \
    > "${GENERATED_DIR}/dagger-ext4-storage.conf"

export CONTAINERS_STORAGE_CONF="${GENERATED_DIR}/dagger-ext4-storage.conf"
export CONTAINERS_CONF="${REGICIDE_CONTAINERS_CONF:-${CATALYST_DIR}/dagger-podman-containers.conf}"
export REGICIDE_SKIP_ROOTLESS_CHECK="${REGICIDE_SKIP_ROOTLESS_CHECK:-1}"
export DAGGER_PROGRESS="${DAGGER_PROGRESS:-plain}"
export DOCKER_CONTENT_TRUST="${DOCKER_CONTENT_TRUST:-0}"

# 4. Preflight: engine runtime sanity before a many-hour build.
"${REPO_ROOT}/build-system/.venv/bin/python" \
    "${REPO_ROOT}/build-system/dagger_pipeline.py" --check-runtime

exec "${REPO_ROOT}/build-system/.venv/bin/python" \
    "${REPO_ROOT}/build-system/dagger_pipeline.py" \
    --plain --skip-sign "$@"
