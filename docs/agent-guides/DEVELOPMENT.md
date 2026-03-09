# Development Quick Reference

## Build Commands

```bash
# Build all packages
cargo build --workspace

# Build specific packages
cargo build -p yama-host-apple             # Apple host
cargo build -p yama-host-jetson            # Jetson host
cargo build -p yama-platform-traits        # Shared traits
cargo build -p yama-protocol               # Protocol definitions

# Release build
cargo build --workspace --release
```

## Run Commands

```bash
# Run Apple host (opens egui window)
cargo run -p yama-host-apple

# Run in headless mode (no UI)
cargo run -p yama-host-apple -- --headless

# Run with specific config
cargo run -p yama-host-apple -- --config ./yama.toml

# Run web frontend dev server
cd web && npm run dev
```

## Test Commands

```bash
# Run all tests
cargo test --workspace

# Run tests for specific package
cargo test -p yama-host-apple
cargo test -p yama-platform-traits

# Run specific test
cargo test -p yama-host-apple test_index_provider

# Run tests with output
cargo test --workspace -- --nocapture

# Run integration tests only
cargo test --workspace --test '*'
```

## Check Commands

```bash
# Lint all packages
cargo clippy --workspace

# Lint with pedantic (fail on warnings)
cargo clippy --workspace -- -D warnings

# Format check (don't modify)
cargo fmt --all -- --check

# Format (modify files)
cargo fmt --all

# Check compilation without building
cargo check --workspace
```

## Web Frontend

```bash
# Install dependencies
cd web && npm install

# Development server
cd web && npm run dev

# Build for production
cd web && npm run build

# Type checking
cd web && npm run check

# Lint
cd web && npm run lint
```

## Model Management

```bash
# Create models directory
mkdir -p ~/.cache/yama/models

# Download Whisper base.en
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \
  -o ~/.cache/yama/models/ggml-base.en.bin

# Download CLIP (use huggingface-cli)
huggingface-cli download openai/clip-vit-base-patch32 \
  --local-dir ~/.cache/yama/models/clip-vit-b-32

# List downloaded models
ls -lh ~/.cache/yama/models/
```

## Database

```bash
# Open SQLite database
sqlite3 ~/.local/share/yama/yama.db

# Common queries
.tables                        # List tables
.schema videos                 # Show schema
SELECT COUNT(*) FROM videos;   # Count videos
```

## Debugging

```bash
# Run with debug logging
RUST_LOG=debug cargo run -p yama-host-apple

# Run with trace logging (very verbose)
RUST_LOG=trace cargo run -p yama-host-apple

# Profile with Instruments (macOS)
cargo instruments -t time --release -p yama-host-apple

# Memory profiling
cargo instruments -t allocations --release -p yama-host-apple
```

## Git Workflow

```bash
# Confirm worktree location
pwd
git branch --show-current
git status

# Standard commit
git add <files>
git commit -m "type(scope): message"

# Types: feat, fix, docs, refactor, test, chore
```

## Common Patterns

### Running Examples

```bash
# Run an example from a package
cargo run -p yama-host-apple --example extract_frames -- ~/Videos/test.mp4
cargo run -p yama-host-apple --example clip_similarity -- "person presenting"
```

### Environment Variables

```bash
# Set model cache location
export YAMA_MODELS_DIR=~/.cache/yama/models

# Set database location
export YAMA_DATA_DIR=~/.local/share/yama

# Enable Metal debugging (macOS)
export MTL_DEBUG_LAYER=1
```

### Useful Aliases

```bash
# Add to ~/.zshrc or ~/.bashrc
alias yama-build='cargo build -p yama-host-apple'
alias yama-run='cargo run -p yama-host-apple'
alias yama-test='cargo test --workspace'
alias yama-check='cargo clippy --workspace && cargo fmt --all -- --check'
```
