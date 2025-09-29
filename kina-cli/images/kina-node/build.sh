#!/bin/bash
# Build script for kina-node image

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_NAME="${1:-kina-node}"
IMAGE_TAG="${2:-latest}"
FULL_IMAGE_NAME="${IMAGE_NAME}:${IMAGE_TAG}"

echo "Building kina-node image: $FULL_IMAGE_NAME"
echo "Build context: $SCRIPT_DIR"

# Check if Docker is available
if ! command -v docker >/dev/null 2>&1; then
    echo "Error: Docker is required to build kina-node image"
    exit 1
fi

# Build the image
docker build \
    --tag "$FULL_IMAGE_NAME" \
    --file "$SCRIPT_DIR/Dockerfile" \
    "$SCRIPT_DIR"

echo "Successfully built $FULL_IMAGE_NAME"

# Test the image builds correctly
echo "Testing image startup..."
CONTAINER_ID=$(docker run --detach --rm "$FULL_IMAGE_NAME")

# Wait a few seconds for systemd to initialize
sleep 5

# Check if systemd is running
if docker exec "$CONTAINER_ID" systemctl is-system-running --wait >/dev/null 2>&1; then
    echo "✓ systemd initialized successfully"
else
    echo "⚠ systemd initialization may have issues"
fi

# Check if containerd is running
if docker exec "$CONTAINER_ID" systemctl is-active containerd >/dev/null 2>&1; then
    echo "✓ containerd service is active"
else
    echo "⚠ containerd service is not active"
fi

# Check if kubelet service exists (it won't start without a cluster)
if docker exec "$CONTAINER_ID" systemctl list-units --type=service | grep -q kubelet; then
    echo "✓ kubelet service is configured"
else
    echo "⚠ kubelet service is not found"
fi

# Check if kina initialization completed
if docker exec "$CONTAINER_ID" test -f /var/lib/kina/node-ready; then
    echo "✓ kina-node initialization completed"
else
    echo "⚠ kina-node initialization incomplete"
fi

# Clean up test container
docker stop "$CONTAINER_ID" >/dev/null

echo ""
echo "Image build and basic validation complete!"
echo "To use with kina:"
echo "  kina create cluster --image $FULL_IMAGE_NAME"
echo ""
echo "To push to registry:"
echo "  docker push $FULL_IMAGE_NAME"