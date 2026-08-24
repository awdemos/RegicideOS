#!/bin/bash
# Fix DNS resolution inside Dagger/Podman containers.
# Run on the host (outside Toolbx).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_FILE="${ROOT_DIR}/dagger-dns-fix.log"

# shellcheck source=../lib/log.sh
source "${SCRIPT_DIR}/../lib/log.sh"
# shellcheck source=../lib/container-runtime.sh
source "${SCRIPT_DIR}/../lib/container-runtime.sh"

usage() {
    cat <<EOF
Usage: $0

Fix DNS resolution inside Dagger / Podman containers.
Configures public DNS resolvers, restarts Podman, removes the old engine,
and pre-pulls the base images needed by the RegicideOS Dagger pipeline.
This script requires a rootful container runtime.
EOF
}

if [[ "${1:-}" == "-h" ]] || [[ "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

: > "$LOG_FILE"
exec > >(tee -a "$LOG_FILE") 2>&1

regicide_log "Dagger DNS fix script"
regicide_log "Log file: $LOG_FILE"
echo

regicide_log "--- Host DNS test ---"
host registry-1.docker.io 2>&1 || true
nslookup registry-1.docker.io 2>&1 || true

echo
regicide_log "--- Host curl test ---"
curl -sI --max-time 10 https://registry-1.docker.io/v2/ 2>&1 | head -5 || true

echo
regicide_log "--- Docker info (runtime) ---"
docker info 2>&1 | grep -iE 'server version|default runtime|runc|crun|storage driver|root dir|rootless' || true

# Detect Dagger engine version from the CLI so we don't hardcode it.
DAGGER_ENGINE_TAG="v0.21.8"
if command -v dagger >/dev/null 2>&1; then
    DAGGER_ENGINE_TAG="$(dagger version | awk '{print $2}')"
fi
regicide_log "Dagger engine tag: $DAGGER_ENGINE_TAG"

# Rootless Podman cannot host the Dagger engine; bail out early with instructions.
if regicide_is_rootless_podman; then
    echo
    regicide_error "Rootless Podman detected. This script cannot fix DNS because"
    regicide_error "the Dagger engine cannot start under rootless Podman."
    regicide_log "Use a rootful runtime instead:"
    echo
    echo "  sudo systemctl enable --now podman.socket"
    echo "  DOCKER_HOST=unix:///run/podman/podman.sock sudo -E ${0}"
    echo
    exit 1
fi

echo
regicide_log "--- Configuring Podman DNS ---"
mkdir -p ~/.config/containers
if [[ -f ~/.config/containers/containers.conf ]]; then
    cp ~/.config/containers/containers.conf ~/.config/containers/containers.conf.bak."$(date +%s)"
fi

cat > ~/.config/containers/containers.conf <<'EOF'
[containers]
dns = ["1.1.1.1", "8.8.8.8"]
dns_backend = "none"
EOF
regicide_success "Podman DNS configured to use 1.1.1.1 and 8.8.8.8"

echo
regicide_log "--- Restarting Podman services ---"
systemctl --user restart podman.service 2>&1 || true
systemctl --user restart podman.socket 2>&1 || true
sleep 3

echo
regicide_log "--- Removing Dagger engine so it picks up DNS changes ---"
docker rm -f "dagger-engine-${DAGGER_ENGINE_TAG}" 2>&1 || true

echo
regicide_log "--- Creating Dagger engine with host networking ---"
docker run -d --name "dagger-engine-${DAGGER_ENGINE_TAG}" \
  --privileged \
  --net=host \
  --restart always \
  -v dagger-engine-state:/var/lib/dagger \
  "registry.dagger.io/engine:${DAGGER_ENGINE_TAG}" 2>&1 || regicide_warn "manual engine start failed"
sleep 5
docker ps -a --filter name=dagger-engine --format '{{.Names}} {{.Status}}' 2>&1 || true

echo
regicide_log "--- Testing DNS from inside the Dagger engine ---"
docker exec -i "dagger-engine-${DAGGER_ENGINE_TAG}" nslookup registry-1.docker.io 2>&1 || regicide_warn "engine DNS test failed"

echo
regicide_log "--- Pre-pulling base images for Dagger pipeline ---"
docker pull gentoo/stage3:amd64-systemd 2>&1 || regicide_warn "failed to pull gentoo/stage3:amd64-systemd"
docker pull alpine:latest 2>&1 || regicide_warn "failed to pull alpine:latest"

echo
regicide_success "DNS fix script complete."
regicide_log "Now run: ./scripts/rebuild-iso.sh"
regicide_log "If it still fails, paste: $LOG_FILE"
