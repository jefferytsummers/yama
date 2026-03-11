#!/bin/bash
# Build VLM container for Jetson Thor
#
# This script builds a container optimized for LLaVA inference
# on NVIDIA Jetson Thor (L4T R38, CUDA 13.0, SM 110)
#
# Prerequisites:
#   - Docker with nvidia runtime
#   - Base image: vila:r38.4.arm64-sbsa-cu130-24.04-pytorch_2.10
#
# Usage:
#   ./build_vlm_container.sh [--clean] [--skip-tests]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_DIR="$(dirname "$SCRIPT_DIR")"
MODELS_PATH="${MODELS_PATH:-/home/jeff/data/models}"

# Parse arguments
CLEAN_BUILD=""
SKIP_TESTS=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --clean) CLEAN_BUILD="--no-cache"; shift ;;
        --skip-tests) SKIP_TESTS=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo "=== Yama VLM Container Build Script ==="
echo "Platform dir: $PLATFORM_DIR"
echo "Models path: $MODELS_PATH"
echo ""

# Detect L4T version
L4T_VERSION=$(head -n 1 /etc/nv_tegra_release | grep -oP '(?<=R)\d+' || echo "38")
echo "Detected L4T R${L4T_VERSION}"

# Check base image exists
BASE_IMAGE="vila:r38.4.arm64-sbsa-cu130-24.04-pytorch_2.10"
if ! docker image inspect "$BASE_IMAGE" &> /dev/null; then
    echo "Error: Base image $BASE_IMAGE not found"
    echo "This image should be pre-built for Thor's SM 110 architecture."
    echo ""
    echo "If the base image is missing, you may need to:"
    echo "  1. Build PyTorch from source for SM 110"
    echo "  2. Or contact NVIDIA for official JetPack 7 containers"
    exit 1
fi

# Create models directory
mkdir -p "$MODELS_PATH/llava"
mkdir -p "$MODELS_PATH/cache"

# Build container
CONTAINER_TAG="yama-vlm:r${L4T_VERSION}"
echo "=== Building $CONTAINER_TAG ==="
cd "$PLATFORM_DIR"

docker build \
    $CLEAN_BUILD \
    -t "$CONTAINER_TAG" \
    -f Dockerfile.vlm \
    .

# Run tests if not skipped
if [ "$SKIP_TESTS" = false ]; then
    echo ""
    echo "=== Running Container Tests ==="
    docker run --rm --runtime nvidia \
        -e NVIDIA_VISIBLE_DEVICES=all \
        -e NVIDIA_DRIVER_CAPABILITIES=compute,utility \
        -v "$MODELS_PATH:/data/models" \
        "$CONTAINER_TAG" \
        /bin/bash -c '
python3 << EOF
import torch
print(f"PyTorch version: {torch.__version__}")
print(f"CUDA available: {torch.cuda.is_available()}")

if torch.cuda.is_available():
    print(f"GPU: {torch.cuda.get_device_name(0)}")

    # Test compute
    x = torch.tensor([1.0, 2.0, 3.0], device="cuda")
    y = torch.tensor([4.0, 5.0, 6.0], device="cuda")
    z = x + y
    print(f"CUDA compute test: {z.cpu().tolist()}")

# Test transformers
from transformers import AutoConfig
print("Transformers loaded successfully")

# Test PIL
from PIL import Image
print("Pillow loaded successfully")

print("\nContainer ready for VLM inference!")
EOF
'
fi

echo ""
echo "=== Build Complete ==="
echo "Container: $CONTAINER_TAG"
echo ""
echo "Next steps:"
echo "  1. Download LLaVA model: ./download_llava.sh"
echo "  2. Test inference: ./test_llava.sh"
echo "  3. Start server: docker-compose -f docker-compose.triton.yml up vlm -d"
