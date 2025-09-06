#!/bin/bash
# RegicideOS Docker Testing Script
# Tests the overlay and AI agents in a real Gentoo environment

set -euo pipefail

SCRIPT_DIR="$(dirname "$(realpath "$0")")"
IMAGE_NAME="regicide-overlay-test"
CONTAINER_NAME="regicide-test-$(date +%s)"

echo "=== RegicideOS Docker Test Suite ==="
echo "Building test container..."

# Build the test image
docker build -f overlays/regicide-rust/Dockerfile.test -t "$IMAGE_NAME" "$SCRIPT_DIR"

echo "✓ Test image built successfully"

# Run tests in container
echo "Running overlay tests in Gentoo container..."

docker run --rm --name "$CONTAINER_NAME" \
    -v "$SCRIPT_DIR/ai-agents:/regicide/ai-agents:ro" \
    -w /var/db/repos/regicide-overlay \
    "$IMAGE_NAME"

echo ""
echo "✓ Docker tests completed"
echo ""
echo "To debug interactively:"
echo "  docker run -it --rm \\"
echo "    -v '$SCRIPT_DIR/ai-agents:/regicide/ai-agents:ro' \\"
echo "    '$IMAGE_NAME' /bin/bash"
echo ""
echo "To test specific components:"
echo "  docker run -it --rm '$IMAGE_NAME' emerge --pretend regicide-tools/btrmind"
