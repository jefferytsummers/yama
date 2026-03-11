#!/bin/bash
# Test LLaVA inference on Jetson Thor
#
# Usage:
#   ./test_llava.sh [model_variant] [image_path]
#
# Examples:
#   ./test_llava.sh                           # Use default model and test image
#   ./test_llava.sh llava-1.5-7b              # Specify model
#   ./test_llava.sh llava-1.5-7b /path/to.jpg # Specify model and image

set -e

# Configuration
MODELS_PATH="${MODELS_PATH:-/home/jeff/data/models}"
MODEL_VARIANT="${1:-llava-1.5-7b}"
IMAGE_PATH="${2:-}"
CONTAINER_TAG="${CONTAINER_TAG:-yama-vlm:r38}"
YAMA_PATH="${YAMA_PATH:-/home/jeff/source/repos/yama}"

MODEL_PATH="$MODELS_PATH/llava/$MODEL_VARIANT"

# Check model exists
if [ ! -d "$MODEL_PATH" ]; then
    echo "Error: Model not found at $MODEL_PATH"
    echo "Run: ./download_llava.sh $MODEL_VARIANT"
    exit 1
fi

echo "=== Yama LLaVA Inference Test ==="
echo "Model: $MODEL_VARIANT"
echo "Container: $CONTAINER_TAG"
echo ""

# Prepare test image
if [ -z "$IMAGE_PATH" ]; then
    # Create a simple test image if none provided
    TEST_IMAGE="/tmp/test_image.jpg"
    echo "Creating test image..."
    docker run --rm -v /tmp:/tmp python:3.12-slim python3 << 'EOF'
from PIL import Image
import random

# Create a simple test image (colored rectangle)
img = Image.new('RGB', (640, 480), color=(random.randint(0,255), random.randint(0,255), random.randint(0,255)))
img.save('/tmp/test_image.jpg')
print("Created test image: /tmp/test_image.jpg")
EOF
    IMAGE_PATH="$TEST_IMAGE"
fi

echo "Image: $IMAGE_PATH"
echo ""

# Run inference test
echo "=== Running LLaVA Inference ==="
docker run --rm --runtime nvidia \
    -v "$MODELS_PATH:/data/models" \
    -v "$(dirname "$IMAGE_PATH"):/data/images" \
    -e CUDA_VISIBLE_DEVICES=0 \
    "$CONTAINER_TAG" \
    python3 << EOF
import torch
import time
from PIL import Image
from transformers import AutoProcessor, LlavaForConditionalGeneration

# Configuration
model_path = "/data/models/llava/$MODEL_VARIANT"
image_path = "/data/images/$(basename "$IMAGE_PATH")"

print(f"Loading model from: {model_path}")
print(f"GPU Memory before load: {torch.cuda.memory_allocated()/1024**3:.2f} GB")

# Load model
start = time.time()
model = LlavaForConditionalGeneration.from_pretrained(
    model_path,
    torch_dtype=torch.float16,
    device_map="auto",
    low_cpu_mem_usage=True,
)
processor = AutoProcessor.from_pretrained(model_path)
load_time = time.time() - start

print(f"Model loaded in {load_time:.2f}s")
print(f"GPU Memory after load: {torch.cuda.memory_allocated()/1024**3:.2f} GB")

# Load image
print(f"\\nLoading image: {image_path}")
image = Image.open(image_path).convert("RGB")
print(f"Image size: {image.size}")

# Prepare prompt
prompt = "USER: <image>\\nDescribe what you see in this image in detail.\\nASSISTANT:"

# Process
inputs = processor(text=prompt, images=image, return_tensors="pt").to("cuda", torch.float16)

# Generate
print("\\n=== Generating Response ===")
start = time.time()
with torch.inference_mode():
    output = model.generate(
        **inputs,
        max_new_tokens=256,
        do_sample=True,
        temperature=0.7,
        top_p=0.9,
    )
gen_time = time.time() - start

# Decode
response = processor.decode(output[0], skip_special_tokens=True)
tokens_generated = output.shape[1] - inputs["input_ids"].shape[1]

print(f"\\n{response}")
print(f"\\n=== Performance ===")
print(f"Tokens generated: {tokens_generated}")
print(f"Generation time: {gen_time:.2f}s")
print(f"Tokens/second: {tokens_generated/gen_time:.2f}")
print(f"GPU Memory: {torch.cuda.memory_allocated()/1024**3:.2f} GB")
EOF

echo ""
echo "=== Test Complete ==="
