#!/bin/bash
# Web server for Tauri beforeDevCommand/beforeBuildCommand
# Tauri handles the Rust backend separately
#
# Usage:
#   ./tauri-web-dev.sh        # Start dev server
#   ./tauri-web-dev.sh build  # Production build

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WEB_DIR="$(dirname "$SCRIPT_DIR")/web"

cd "$WEB_DIR"

# Install deps if needed
if [ ! -d "node_modules" ]; then
    npm install
fi

if [ "$1" = "build" ]; then
    exec npm run build
else
    exec npm run dev
fi
