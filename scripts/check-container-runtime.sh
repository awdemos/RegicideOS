#!/bin/bash
# Validate that the container runtime is suitable for Dagger.
# Exits 0 if rootful, exits 1 with instructions if rootless Podman is detected.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=lib/log.sh
source "${SCRIPT_DIR}/lib/log.sh"
# shellcheck source=lib/container-runtime.sh
source "${SCRIPT_DIR}/lib/container-runtime.sh"

regicide_require_rootful_runtime
