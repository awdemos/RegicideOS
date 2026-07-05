#!/bin/bash
# Stage 3: install the base system packages.
# This wrapper runs the cacheable sub-stages so manual builds match Dagger.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${SCRIPT_DIR}/stage3-base-a.sh"
"${SCRIPT_DIR}/stage3-base-b.sh"
"${SCRIPT_DIR}/stage3-base-c.sh"
"${SCRIPT_DIR}/stage3-base-d.sh"
"${SCRIPT_DIR}/stage3-base-e.sh"
"${SCRIPT_DIR}/stage3-base-f.sh"

echo "Stage 3 complete."
