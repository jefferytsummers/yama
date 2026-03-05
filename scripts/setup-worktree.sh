#!/bin/bash
# Yama Worktree Setup Script
# Creates a platform-specific worktree with sparse checkout

set -e

PLATFORM=$1
WORKTREE_PATH=$2
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <platform> <worktree-path>"
    echo ""
    echo "Platforms:"
    echo "  apple   - Apple Silicon (M1/M2/M3)"
    echo "  jetson  - NVIDIA Jetson"
    echo ""
    echo "Examples:"
    echo "  $0 apple ../yama-apple"
    echo "  $0 jetson ../yama-jetson"
    exit 1
}

# Validate arguments
if [[ -z "$PLATFORM" || -z "$WORKTREE_PATH" ]]; then
    usage
fi

if [[ "$PLATFORM" != "apple" && "$PLATFORM" != "jetson" ]]; then
    echo -e "${RED}Error: Invalid platform '$PLATFORM'${NC}"
    echo "Valid platforms: apple, jetson"
    exit 1
fi

# Check if worktree path already exists
if [[ -d "$WORKTREE_PATH" ]]; then
    echo -e "${RED}Error: Worktree path already exists: $WORKTREE_PATH${NC}"
    exit 1
fi

# Verify we're in a git repository
if ! git -C "$REPO_ROOT" rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}Error: Not a git repository${NC}"
    exit 1
fi

echo -e "${GREEN}Setting up Yama worktree for $PLATFORM${NC}"
echo "Repository: $REPO_ROOT"
echo "Worktree: $WORKTREE_PATH"
echo ""

# Create the worktree with a new branch
BRANCH_NAME="dev-$PLATFORM"
echo -e "${YELLOW}Creating worktree with branch: $BRANCH_NAME${NC}"

# Check if branch exists
if git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
    echo "Branch $BRANCH_NAME already exists, using it"
    git -C "$REPO_ROOT" worktree add "$WORKTREE_PATH" "$BRANCH_NAME"
else
    echo "Creating new branch $BRANCH_NAME"
    git -C "$REPO_ROOT" worktree add -b "$BRANCH_NAME" "$WORKTREE_PATH"
fi

# Navigate to worktree
cd "$WORKTREE_PATH"

# Enable sparse checkout
echo -e "${YELLOW}Enabling sparse checkout${NC}"
git sparse-checkout init --cone

# Apply platform-specific patterns
SPARSE_FILE="$SCRIPT_DIR/sparse-checkout/$PLATFORM.txt"
if [[ -f "$SPARSE_FILE" ]]; then
    echo "Applying patterns from $SPARSE_FILE"
    git sparse-checkout set --stdin < "$SPARSE_FILE"
else
    echo -e "${YELLOW}Warning: Sparse checkout file not found, using defaults${NC}"
    # Default patterns
    git sparse-checkout set \
        Cargo.toml \
        Cargo.lock \
        yama.toml \
        shared \
        containers \
        "platform/$PLATFORM"
fi

# Count files
RUST_FILES=$(find . -name '*.rs' 2>/dev/null | wc -l | tr -d ' ')
TOTAL_FILES=$(find . -type f 2>/dev/null | wc -l | tr -d ' ')

echo ""
echo -e "${GREEN}Worktree created successfully!${NC}"
echo ""
echo "Statistics:"
echo "  Total files: $TOTAL_FILES"
echo "  Rust files: $RUST_FILES"
echo ""
echo "Next steps:"
echo "  cd $WORKTREE_PATH"
echo "  cargo build --release"
echo ""

# Platform-specific instructions
if [[ "$PLATFORM" == "apple" ]]; then
    echo "Apple Silicon Setup:"
    echo "  # Start services"
    echo "  docker compose -f platform/apple/docker-compose.yml up"
    echo ""
    echo "  # Run the host compositor"
    echo "  cargo run --release -p yama-host-apple"
elif [[ "$PLATFORM" == "jetson" ]]; then
    echo "NVIDIA Jetson Setup:"
    echo "  # Ensure NVIDIA Container Runtime is installed"
    echo "  # Start services"
    echo "  docker compose -f platform/jetson/docker-compose.yml up"
    echo ""
    echo "  # Run the host compositor"
    echo "  cargo run --release -p yama-host-jetson"
fi

echo ""
echo -e "${GREEN}Done!${NC}"
