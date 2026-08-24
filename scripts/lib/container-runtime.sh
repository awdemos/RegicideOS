#!/bin/bash
# Container runtime detection helpers for RegicideOS scripts.
# Source this file after sourcing log.sh.

# Return 0 if the current Docker-compatible endpoint is Podman running in
# rootless mode. Dagger's engine image needs to create the dagger0 bridge,
# which rootless Podman cannot do.
regicide_is_rootless_podman() {
    if ! command -v docker >/dev/null 2>&1; then
        return 1
    fi
    # "docker info" on a Podman socket reports "rootless" under Security Options.
    if docker info 2>/dev/null | grep -qiE '^  rootless$|rootless:\s*true'; then
        return 0
    fi
    return 1
}

# Print the name of the container runtime backing the current docker endpoint
# (e.g. "Podman", "Docker", or "unknown").
regicide_container_runtime_name() {
    if ! command -v docker >/dev/null 2>&1; then
        echo "none"
        return
    fi
    local server_name
    server_name=$(docker version 2>/dev/null | awk '/Server:/,/Podman Engine:|Docker Engine:/' | grep -iE 'Podman Engine|Docker Engine' | head -n1 | sed -E 's/.*(Podman|Docker).*/\1/i')
    if [[ -z "$server_name" ]]; then
        server_name=$(docker version 2>/dev/null | grep -iE '^ Server:' | head -n1)
        if [[ "$server_name" == *Podman* ]]; then
            echo "Podman"
            return
        elif [[ "$server_name" == *Docker* ]]; then
            echo "Docker"
            return
        fi
        echo "unknown"
        return
    fi
    # Capitalize first letter.
    echo "${server_name^}"
}

# Print a known rootful Podman socket path if the socket node exists, else nothing.
regicide_rootful_podman_socket() {
    local sock
    for sock in /run/podman/podman.sock /var/run/podman/podman.sock; do
        if [[ -S "$sock" ]]; then
            echo "$sock"
            return 0
        fi
    done
    return 1
}

# Print human-readable information about the current container runtime.
regicide_container_runtime_info() {
    local runtime_name socket
    runtime_name=$(regicide_container_runtime_name)
    socket=${DOCKER_HOST:-"default docker context"}
    regicide_log "INFO" "Runtime: $runtime_name"
    regicide_log "INFO" "DOCKER_HOST: $socket"
    if regicide_is_rootless_podman; then
        regicide_log "INFO" "Rootless: yes"
    else
        regicide_log "INFO" "Rootless: no"
    fi
}

# Ensure the container runtime is suitable for Dagger. If the current endpoint
# is rootless Podman, print a clear error and instructions, then exit 1.
regicide_require_rootful_runtime() {
    if ! command -v docker >/dev/null 2>&1; then
        regicide_error "docker CLI not found in PATH"
        exit 1
    fi

    local runtime_name
    runtime_name=$(regicide_container_runtime_name)
    regicide_log "INFO" "Container runtime: $runtime_name (${DOCKER_HOST:-default context})"

    if ! regicide_is_rootless_podman; then
        return 0
    fi

    regicide_error "Rootless Podman is not supported by Dagger engine v0.21.8."
    regicide_error "The engine needs to create the 'dagger0' bridge, which requires root."
    echo

    local rootful_sock
    rootful_sock=$(regicide_rootful_podman_socket || true)
    if [[ -n "$rootful_sock" ]]; then
        regicide_log "INFO" "A rootful Podman socket was detected at: $rootful_sock"
        regicide_log "INFO" "Run the build with the rootful socket, for example:"
        echo
        echo "  sudo systemctl start podman.socket"
        echo "  DOCKER_HOST=unix://$rootful_sock sudo -E ${0:-./scripts/rebuild-iso.sh}"
        echo
        regicide_log "Or, if you prefer Docker, install rootful Docker CE and unset DOCKER_HOST."
    else
        regicide_log "INFO" "No rootful Podman socket found at /run/podman/podman.sock."
        regicide_log "INFO" "Start one with:"
        echo
        echo "  sudo systemctl enable --now podman.socket"
        echo
        regicide_log "INFO" "Then re-run with:"
        echo
        echo "  DOCKER_HOST=unix:///run/podman/podman.sock sudo -E ${0:-./scripts/rebuild-iso.sh}"
        echo
    fi
    exit 1
}
