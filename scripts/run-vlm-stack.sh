#!/bin/bash
# Run the Yama VLM Stack locally
#
# This script starts:
# 1. Event Bus Server (WebSocket + Unix socket)
# 2. VLM Container (Qwen2.5-VL with Metal acceleration)
#
# Requirements:
# - Rust toolchain
# - ~8GB RAM for 7B model (or use smaller 3B model)
# - GStreamer installed
#
# Usage:
#   ./scripts/run-vlm-stack.sh           # Use default 7B model
#   ./scripts/run-vlm-stack.sh --small   # Use smaller 3B model (faster download)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$REPO_ROOT"

# Parse arguments
MODEL_ID="Qwen/Qwen2.5-VL-7B-Instruct"
MODEL_NAME="7B"
if [[ "$1" == "--small" ]]; then
    MODEL_ID="Qwen/Qwen2.5-VL-3B-Instruct"
    MODEL_NAME="3B"
    echo "Using smaller 3B model for faster download"
fi

echo "========================================="
echo "  Yama VLM Stack - Local Development"
echo "========================================="
echo ""
echo "Model: $MODEL_ID"
echo "Quantization: Q4K (4-bit)"
echo "Device: Metal (GPU)"
echo ""

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "Shutting down..."
    jobs -p | xargs -r kill 2>/dev/null || true
    rm -f /tmp/yama-event.sock
    exit 0
}
trap cleanup EXIT INT TERM

# Build the binaries
echo "Building binaries..."
cargo build -p yama-host-apple --bin yama-event-bus --release 2>&1 | grep -v "^warning"
cargo build -p yama-vlm --features metal --release 2>&1 | grep -v "^warning"
echo "Build complete!"
echo ""

# Remove old socket if exists
rm -f /tmp/yama-event.sock

# Start Event Bus
echo "Starting Event Bus..."
./target/release/yama-event-bus &
EVENT_BUS_PID=$!
sleep 2

# Check if event bus started
if ! kill -0 $EVENT_BUS_PID 2>/dev/null; then
    echo "ERROR: Event bus failed to start"
    exit 1
fi
echo "Event Bus running (PID: $EVENT_BUS_PID)"
echo "  - WebSocket: ws://localhost:8765"
echo "  - Unix Socket: /tmp/yama-event.sock"
echo ""

# Start VLM Container
echo "Starting VLM Container..."
echo "  (First run will download model - this may take a while)"
echo ""

export VLM_MODEL_ID="$MODEL_ID"
export YAMA_EVENT_BUS_URL="ws://localhost:8765"

./target/release/yama-vlm &
VLM_PID=$!

echo "VLM Container starting (PID: $VLM_PID)"
echo ""
echo "========================================="
echo "  Stack is running!"
echo "========================================="
echo ""
echo "The VLM model will download on first run (~4GB for 7B, ~2GB for 3B)."
echo "Watch the logs above for 'VLM engine initialized' message."
echo ""
echo "To test, run in another terminal:"
echo "  cargo run -p yama-host-apple --bin yama-headless -- \\"
echo "    --video /path/to/video.mp4 \\"
echo "    --prompt 'Describe what you see' \\"
echo "    --synthesize"
echo ""
echo "Or run the headless test scenario:"
echo "  cargo run -p yama-host-apple --bin yama-headless -- --test-scenario single"
echo ""
echo "Press Ctrl+C to stop all services"
echo ""

# Wait for processes
wait
