#!/bin/bash
set -e

# Kina Node Image Builder
# Builds custom Debian-based Kubernetes node image for Apple Container VMs
# Uses Apple Container CLI (NOT Docker)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_TAG="kina/node:v1.35.4"

echo "🏗️  Building Kina Kubernetes node image..."
echo "📁 Working directory: $SCRIPT_DIR"
echo "🏷️  Image tag: $IMAGE_TAG"

# Verify Apple Container CLI is available
if ! command -v container >/dev/null 2>&1; then
    echo "❌ Apple Container CLI 'container' not found"
    echo "Please ensure Apple Container is installed and available in PATH"
    exit 1
fi

echo "✅ Apple Container CLI found: $(container --version 2>/dev/null || echo 'version check failed')"

# Build the image using Apple Container (NOT Docker)
echo "🔨 Building image with Apple Container CLI..."
cd "$SCRIPT_DIR"

# Build the image
container build -t "$IMAGE_TAG" .

# Verify the image was built successfully
if container image list --format json | jq -e --arg tag "$IMAGE_TAG" '.[] | select(.reference == $tag)' > /dev/null 2>&1; then
    echo "✅ Successfully built image: $IMAGE_TAG"

    # Show image details
    echo "📋 Image details:"
    container image inspect "$IMAGE_TAG" | jq -r '.[] | "  Size: \(.variants[0].size // "unknown") | Created: \(.variants[0].config.created // "unknown")"'

    echo ""
    echo "🎉 Build complete! You can now use this image with kina:"
    echo "   mise run kina create my-cluster --image $IMAGE_TAG"

else
    echo "❌ Failed to build image"
    exit 1
fi

echo ""
echo "📝 Next steps:"
echo "1. Test the image: container run --rm -it $IMAGE_TAG /bin/bash"
echo "2. Create cluster: mise run kina create test-cluster --image $IMAGE_TAG"
echo "3. Install Cilium: cilium install"
