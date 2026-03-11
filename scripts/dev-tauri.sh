#!/bin/bash
# Development script for Tauri app
# Run from project root: ./scripts/dev-tauri.sh

set -e

cd "$(dirname "$0")/.."

echo "Starting Yama Tauri development..."
echo "Working directory: $(pwd)"

# Ensure web dependencies are installed
if [ ! -d "web/node_modules" ]; then
    echo "Installing web dependencies..."
    npm install --prefix web
fi

# Run Tauri dev from src-tauri directory
cd src-tauri
cargo tauri dev
