#!/bin/bash
# Local VLM development stack
#
# Usage:
#   ./scripts/dev-vlm.sh start    # Start event bus + VLM in background
#   ./scripts/dev-vlm.sh stop     # Stop all services
#   ./scripts/dev-vlm.sh status   # Check service status
#   ./scripts/dev-vlm.sh logs     # Tail VLM logs
#
# After starting, run inference:
#   cargo run -p yama-host-apple --bin yama-headless -- \
#     --video ~/Documents/video.mp4 --prompt "What happens?" --synthesize

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
PID_DIR="${REPO_ROOT}/.dev-pids"
LOG_DIR="${REPO_ROOT}/.dev-logs"

mkdir -p "$PID_DIR" "$LOG_DIR"

start_services() {
    echo "Starting Yama VLM development stack..."

    # Build first
    echo "Building services..."
    cargo build -p yama-event-bus -p yama-vlm --release

    # Start event bus
    echo "Starting event bus..."
    EVENT_BUS_BIND="0.0.0.0:8765" \
    RUST_LOG=info \
    "${REPO_ROOT}/target/release/yama-event-bus" \
        > "${LOG_DIR}/event-bus.log" 2>&1 &
    echo $! > "${PID_DIR}/event-bus.pid"

    sleep 1

    # Start VLM
    echo "Starting VLM service (this may take a while to load the model)..."
    YAMA_EVENT_BUS="/tmp/yama-event.sock" \
    RUST_LOG=info \
    "${REPO_ROOT}/target/release/yama-vlm" \
        > "${LOG_DIR}/vlm.log" 2>&1 &
    echo $! > "${PID_DIR}/vlm.pid"

    echo ""
    echo "Services started!"
    echo "  Event bus: ws://localhost:8765"
    echo "  VLM logs:  tail -f ${LOG_DIR}/vlm.log"
    echo ""
    echo "Run inference with:"
    echo "  cargo run -p yama-host-apple --bin yama-headless -- \\"
    echo "    --video ~/Documents/video.mp4 --prompt \"What happens?\" --synthesize"
}

stop_services() {
    echo "Stopping services..."

    for pid_file in "${PID_DIR}"/*.pid; do
        if [[ -f "$pid_file" ]]; then
            pid=$(cat "$pid_file")
            name=$(basename "$pid_file" .pid)
            if kill -0 "$pid" 2>/dev/null; then
                echo "Stopping $name (PID $pid)..."
                kill "$pid" 2>/dev/null || true
            fi
            rm -f "$pid_file"
        fi
    done

    echo "Services stopped."
}

status_services() {
    echo "Service status:"
    for pid_file in "${PID_DIR}"/*.pid; do
        if [[ -f "$pid_file" ]]; then
            pid=$(cat "$pid_file")
            name=$(basename "$pid_file" .pid)
            if kill -0 "$pid" 2>/dev/null; then
                echo "  $name: running (PID $pid)"
            else
                echo "  $name: stopped"
                rm -f "$pid_file"
            fi
        fi
    done
}

show_logs() {
    if [[ -f "${LOG_DIR}/vlm.log" ]]; then
        tail -f "${LOG_DIR}/vlm.log"
    else
        echo "No VLM logs found. Start services first."
    fi
}

case "${1:-}" in
    start)
        start_services
        ;;
    stop)
        stop_services
        ;;
    status)
        status_services
        ;;
    logs)
        show_logs
        ;;
    restart)
        stop_services
        sleep 1
        start_services
        ;;
    *)
        echo "Usage: $0 {start|stop|status|logs|restart}"
        exit 1
        ;;
esac
