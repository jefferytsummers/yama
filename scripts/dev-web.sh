#!/bin/bash
# Development script for Yama web UI with project persistence
#
# Starts:
# 1. Native host app (HTTP server on :8080, egui window)
# 2. Web dev server (Vite on :5173, proxies /api to :8080)
#
# Usage:
#   ./scripts/dev-web.sh              # Full stack (native + web)
#   ./scripts/dev-web.sh --web-only   # Web dev server only (assumes backend running)
#   ./scripts/dev-web.sh --headless   # No egui window, just HTTP server
#
# Press Ctrl+C to stop all services

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
WEB_DIR="$REPO_ROOT/web"

cd "$REPO_ROOT"

# Parse arguments
WEB_ONLY=false
HEADLESS=false
for arg in "$@"; do
    case $arg in
        --web-only)
            WEB_ONLY=true
            ;;
        --headless)
            HEADLESS=true
            ;;
    esac
done

echo "========================================="
echo "  Yama Web Development Server"
echo "========================================="
echo ""
echo "Database: ~/.yama/yama.db"
echo "Web UI:   http://localhost:5173"
echo "API:      http://localhost:8080/api"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "Shutting down..."
    jobs -p | xargs -r kill 2>/dev/null || true
    exit 0
}
trap cleanup EXIT INT TERM

if [ "$WEB_ONLY" = false ]; then
    # Build and run native app
    echo "Building native app..."
    cargo build -p yama-host-apple --release 2>&1 | grep -E "^(Compiling|Finished)" || true
    echo "Build complete!"
    echo ""

    # Start native app (binary is named 'yama')
    echo "Starting native host..."
    if [ "$HEADLESS" = true ]; then
        # Run in headless mode (no egui window)
        YAMA_HEADLESS=1 ./target/release/yama &
    else
        ./target/release/yama &
    fi
    HOST_PID=$!

    # Wait for HTTP server to be ready
    echo "Waiting for HTTP server..."
    for i in {1..30}; do
        if curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
            echo "HTTP server ready!"
            break
        fi
        sleep 0.5
    done

    if ! curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
        echo "ERROR: HTTP server failed to start"
        exit 1
    fi
    echo ""
fi

# Start web dev server
echo "Starting web dev server..."
cd "$WEB_DIR"

# Check if npm dependencies are installed
if [ ! -d "node_modules" ]; then
    echo "Installing npm dependencies..."
    npm install
fi

npm run dev &
WEB_PID=$!
echo ""

echo "========================================="
echo "  Development servers running!"
echo "========================================="
echo ""
echo "  Web UI:     http://localhost:5173"
echo "  API Server: http://localhost:8080"
echo "  Health:     http://localhost:8080/api/health"
echo ""
echo "  Routes:"
echo "    /                  - Home"
echo "    /onboarding        - Project wizard"
echo "    /dashboard         - Dashboard"
echo "    /style-guide       - Component library"
echo ""
echo "Press Ctrl+C to stop all services"
echo ""

# Wait for processes
wait
