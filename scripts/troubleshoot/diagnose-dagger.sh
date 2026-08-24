#!/bin/bash
# Diagnose Dagger engine connection issues.
# Run on the host (outside Toolbx).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_FILE="${ROOT_DIR}/dagger-diagnose.log"

# shellcheck source=../lib/log.sh
source "${SCRIPT_DIR}/../lib/log.sh"
# shellcheck source=../lib/container-runtime.sh
source "${SCRIPT_DIR}/../lib/container-runtime.sh"

: > "$LOG_FILE"
exec > >(tee -a "$LOG_FILE") 2>&1

regicide_log "Dagger diagnostics"
regicide_log "Log file: $LOG_FILE"
echo

# Detect Dagger engine version from the CLI so we don't hardcode it.
DAGGER_ENGINE_TAG="v0.21.8"
if command -v dagger >/dev/null 2>&1; then
    DAGGER_ENGINE_TAG="$(dagger version | awk '{print $2}')"
fi

regicide_log "--- Dagger CLI and Python client ---"
dagger version 2>&1 || true
"${ROOT_DIR}/.venv/bin/python" -m pip show dagger-io 2>&1 | grep -E 'Name|Version|Location' || true
regicide_log "Dagger engine tag: $DAGGER_ENGINE_TAG"

echo
regicide_log "--- Docker/Podman setup ---"
command -v docker 2>&1 || true
readlink -f "$(command -v docker)" 2>&1 || true
docker version 2>&1 || true
docker info 2>&1 | grep -iE 'version|runtime|rootless|store|backend' || true
if regicide_is_rootless_podman; then
    regicide_warn "Current runtime is rootless Podman (Dagger engine is unsupported in this mode)."
fi

echo
regicide_log "--- Engine container ---"
docker ps -a --filter name=dagger-engine 2>&1 || true
docker inspect "dagger-engine-${DAGGER_ENGINE_TAG}" --format='{{json .State}}' 2>&1 || true
docker inspect "dagger-engine-${DAGGER_ENGINE_TAG}" --format='{{json .HostConfig.NetworkMode}}' 2>&1 || true

echo
regicide_log "--- Engine logs (last 100 lines) ---"
docker logs --tail 100 "dagger-engine-${DAGGER_ENGINE_TAG}" 2>&1 || true

echo
regicide_log "--- buildctl dial-stdio test ---"
timeout 30 docker exec -i "dagger-engine-${DAGGER_ENGINE_TAG}" buildctl dial-stdio </dev/null 2>&1 || true

echo
regicide_log "--- Host DNS ---"
cat /etc/resolv.conf 2>&1 || true
host registry-1.docker.io 2>&1 || true

echo
regicide_log "--- Podman services ---"
systemctl --user status podman.service --no-pager 2>&1 | head -20 || true
systemctl --user status podman.socket --no-pager 2>&1 | head -20 || true

echo
regicide_log "--- Rootful Podman socket ---"
sudo systemctl status podman.socket --no-pager 2>&1 | head -20 || true
ls -la /run/podman/podman.sock 2>&1 || true

echo
regicide_success "Diagnostics complete."
regicide_log "Paste the contents of: $LOG_FILE"
