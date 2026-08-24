#!/bin/bash
# Fix Dagger engine connection issues on Fedora Atomic / crun hosts.
# Run on the host (outside Toolbx).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_FILE="${ROOT_DIR}/dagger-fix.log"

# shellcheck source=../lib/log.sh
source "${SCRIPT_DIR}/../lib/log.sh"
# shellcheck source=../lib/container-runtime.sh
source "${SCRIPT_DIR}/../lib/container-runtime.sh"

usage() {
    cat <<EOF
Usage: $0

Fix Dagger engine connection issues on Fedora Atomic / Podman hosts.
Removes the broken engine, switches the runtime from crun to runc, and
restarts the engine container. This script requires a rootful container
runtime; rootless Podman cannot create the Dagger engine's dagger0 bridge.
EOF
}

if [[ "${1:-}" == "-h" ]] || [[ "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

: > "$LOG_FILE"
exec > >(tee -a "$LOG_FILE") 2>&1

regicide_log "Dagger engine fix script"
regicide_log "Log file: $LOG_FILE"
echo

regicide_log "--- Docker version ---"
docker version 2>&1 || true

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
    regicide_error "Rootless Podman detected. This script cannot fix the engine because"
    regicide_error "rootless Podman cannot create the 'dagger0' bridge required by Dagger."
    regicide_log "Use a rootful runtime instead:"
    echo
    echo "  sudo systemctl enable --now podman.socket"
    echo "  DOCKER_HOST=unix:///run/podman/podman.sock sudo -E ${0}"
    echo
    exit 1
fi

echo
regicide_log "--- Current Dagger engine container ---"
docker ps -a --filter name=dagger-engine --format '{{.Names}} {{.Status}} {{.State}}' 2>&1 || true

echo
regicide_log "--- Removing old Dagger engine container and image ---"
docker rm -f "dagger-engine-${DAGGER_ENGINE_TAG}" 2>&1 || true
docker rmi -f "registry.dagger.io/engine:${DAGGER_ENGINE_TAG}" 2>&1 || true
regicide_success "Old engine removed"

echo
regicide_log "--- Detecting Docker/Podman setup ---"
DOCKER_PATH=$(command -v docker || true)
regicide_log "docker binary: $DOCKER_PATH"

# Check if docker is actually podman (common on Fedora Atomic)
IS_PODMAN_WRAPPER=false
if [[ -L "$DOCKER_PATH" ]]; then
    LINK_TARGET=$(readlink "$DOCKER_PATH" || true)
    regicide_log "docker symlink target: $LINK_TARGET"
    if [[ "$LINK_TARGET" == *podman* ]]; then
        IS_PODMAN_WRAPPER=true
    fi
fi
if docker version 2>&1 | grep -qi podman; then
    IS_PODMAN_WRAPPER=true
fi

if [[ "$IS_PODMAN_WRAPPER" == true ]]; then
    regicide_warn "docker is a Podman wrapper. Configuring Podman to use runc..."
    mkdir -p ~/.config/containers
    if [[ -f ~/.config/containers/containers.conf ]]; then
        cp ~/.config/containers/containers.conf ~/.config/containers/containers.conf.bak."$(date +%s)"
    fi
    cat > ~/.config/containers/containers.conf <<'EOF'
[engine]
runtime = "runc"
EOF
    regicide_log "Podman runtime set to runc in ~/.config/containers/containers.conf"
    regicide_log "Restarting rootless Podman services..."
    systemctl --user restart podman.service 2>&1 || true
    systemctl --user restart podman.socket 2>&1 || true
    sleep 3
else
    RUNTIME=$(docker info 2>&1 | grep -i 'default runtime' | awk '{print $NF}' || echo "unknown")
    regicide_log "Current Docker runtime: $RUNTIME"

    if [[ "$RUNTIME" == "crun" ]]; then
        regicide_warn "Docker is using crun, which causes Dagger engine crashes."
        if command -v sudo >/dev/null 2>&1; then
            regicide_log "Switching Docker default runtime to runc..."
            sudo mkdir -p /etc/docker
            sudo tee /etc/docker/daemon.json >/dev/null <<'EOF'
{
  "default-runtime": "runc"
}
EOF
            regicide_log "Attempting to restart Docker..."
            if sudo systemctl restart docker 2>&1; then
                sleep 5
            elif sudo systemctl restart docker.socket docker.service 2>&1; then
                sleep 5
            else
                regicide_warn "Could not restart docker.service (it may not be a systemd unit on this host)."
                regicide_warn "You may need to restart the Docker daemon manually."
            fi
            NEW_RUNTIME=$(docker info 2>&1 | grep -i 'default runtime' | awk '{print $NF}' || echo "unknown")
            regicide_log "New Docker runtime: $NEW_RUNTIME"
        else
            regicide_error "sudo not available; cannot switch runtime automatically."
            regicide_error "Switch manually: set default-runtime to runc in /etc/docker/daemon.json and restart docker."
        fi
    fi
fi

echo
regicide_log "--- Testing Docker with hello-world ---"
docker run --rm hello-world 2>&1 || regicide_warn "hello-world test failed"

echo
regicide_log "--- Pulling fresh Dagger engine image ---"
docker pull "registry.dagger.io/engine:${DAGGER_ENGINE_TAG}" 2>&1 || regicide_warn "engine pull failed"

echo
regicide_log "--- Starting Dagger engine manually with host networking ---"
docker run -d --name "dagger-engine-${DAGGER_ENGINE_TAG}" \
  --privileged \
  --net=host \
  --restart always \
  -v dagger-engine-state:/var/lib/dagger \
  "registry.dagger.io/engine:${DAGGER_ENGINE_TAG}" 2>&1 || regicide_warn "manual engine start failed"

sleep 5

echo
regicide_log "--- Engine container status ---"
docker ps -a --filter name=dagger-engine --format '{{.Names}} {{.Status}} {{.State}}' 2>&1 || true
docker inspect "dagger-engine-${DAGGER_ENGINE_TAG}" --format='{{json .State}}' 2>&1 || true

echo
regicide_log "--- Engine logs (last 50 lines) ---"
docker logs --tail 50 "dagger-engine-${DAGGER_ENGINE_TAG}" 2>&1 || true

echo
regicide_log "--- Testing buildctl dial-stdio ---"
docker exec -i "dagger-engine-${DAGGER_ENGINE_TAG}" buildctl dial-stdio </dev/null 2>&1 || regicide_warn "buildctl dial-stdio failed"

echo
regicide_success "Fix script complete."
regicide_log "If the engine is now running, run: ./scripts/rebuild-iso.sh"
regicide_log "If it's still broken, paste the contents of: $LOG_FILE"
