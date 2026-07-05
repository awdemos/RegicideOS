#!/bin/bash
# Stage 4: install the COSMIC desktop packages.
# This wrapper runs the cacheable sub-stages so manual builds match Dagger.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${SCRIPT_DIR}/stage4-cosmic-a.sh"
"${SCRIPT_DIR}/stage4-cosmic-b.sh"

echo "Stage 4 complete."
