#!/bin/bash
# Download LLaVA model for Yama VLM inference
#
# Downloads LLaVA-1.5-7B (or variant) from HuggingFace
#
# Usage:
#   ./download_llava.sh [model_variant]
#
# Model variants:
#   - llava-1.5-7b (default)
#   - llava-1.5-13b
#   - llava-v1.6-vicuna-7b
#   - llava-v1.6-mistral-7b

set -e

# Configuration
MODELS_PATH="${MODELS_PATH:-/home/jeff/data/models}"
MODEL_VARIANT="${1:-llava-1.5-7b}"
CONTAINER_TAG="${CONTAINER_TAG:-yama-vlm:r38}"

# Model mapping
declare -A MODEL_IDS=(
    ["llava-1.5-7b"]="liuhaotian/llava-v1.5-7b"
    ["llava-1.5-13b"]="liuhaotian/llava-v1.5-13b"
    ["llava-v1.6-vicuna-7b"]="liuhaotian/llava-v1.6-vicuna-7b"
    ["llava-v1.6-mistral-7b"]="liuhaotian/llava-v1.6-mistral-7b"
)

MODEL_ID="${MODEL_IDS[$MODEL_VARIANT]}"
if [ -z "$MODEL_ID" ]; then
    echo "Unknown model variant: $MODEL_VARIANT"
    echo "Available: ${!MODEL_IDS[@]}"
    exit 1
fi

echo "=== Yama LLaVA Model Download ==="
echo "Model: $MODEL_VARIANT ($MODEL_ID)"
echo "Path: $MODELS_PATH/llava/$MODEL_VARIANT"
echo ""

# Create directories
mkdir -p "$MODELS_PATH/llava/$MODEL_VARIANT"
mkdir -p "$MODELS_PATH/cache"

# Check for HuggingFace token
if [ -z "$HUGGING_FACE_HUB_TOKEN" ]; then
    echo "Note: HUGGING_FACE_HUB_TOKEN not set"
    echo "Some models may require authentication"
    echo ""
fi

# Download using container (has huggingface_hub installed)
echo "=== Downloading model ==="
docker run --rm --runtime nvidia \
    -v "$MODELS_PATH:/data/models" \
    -v "$MODELS_PATH/cache:/root/.cache/huggingface" \
    -e HUGGING_FACE_HUB_TOKEN="${HUGGING_FACE_HUB_TOKEN:-}" \
    -e HF_HOME="/root/.cache/huggingface" \
    "$CONTAINER_TAG" \
    python3 << EOF
import os
from huggingface_hub import snapshot_download

model_id = "$MODEL_ID"
local_dir = "/data/models/llava/$MODEL_VARIANT"

print(f"Downloading {model_id} to {local_dir}")

# Download all files
snapshot_download(
    repo_id=model_id,
    local_dir=local_dir,
    local_dir_use_symlinks=False,
    resume_download=True,
)

print(f"\\nDownload complete!")
print(f"Model saved to: {local_dir}")

# List downloaded files
import subprocess
result = subprocess.run(["du", "-sh", local_dir], capture_output=True, text=True)
print(f"Size: {result.stdout.strip()}")
EOF

echo ""
echo "=== Download Complete ==="
echo "Model: $MODELS_PATH/llava/$MODEL_VARIANT"
echo ""
echo "To test: ./test_llava.sh $MODEL_VARIANT"
