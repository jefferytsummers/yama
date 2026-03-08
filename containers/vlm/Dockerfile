# Yama VLM Container
# Vision Language Model inference for video analysis

# Build stage
FROM rustlang/rust:nightly as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members (needed for workspace resolution)
COPY shared/ shared/
COPY containers/ containers/

# Create stub directories for platform crates (not needed for VLM container)
RUN mkdir -p platform/apple/host/src platform/apple/video/src platform/apple/inference/src \
    && echo "fn main() {}" > platform/apple/host/src/main.rs \
    && echo "" > platform/apple/video/src/lib.rs \
    && echo "" > platform/apple/inference/src/lib.rs
COPY platform/apple/host/Cargo.toml platform/apple/host/
COPY platform/apple/video/Cargo.toml platform/apple/video/
COPY platform/apple/inference/Cargo.toml platform/apple/inference/

# Build the VLM container
RUN cargo build --release -p yama-vlm

# Runtime stage - use trixie (testing) for newer glibc
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    procps \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-libav \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user with home directory
RUN useradd -r -s /bin/false -m vlm

# Create directories
RUN mkdir -p /app/config /app/models /home/vlm/.cache/huggingface && \
    chown -R vlm:vlm /app /home/vlm

# Copy binary
COPY --from=builder /app/target/release/yama-vlm /usr/local/bin/

# Copy default config if it exists
COPY --chown=vlm:vlm containers/vlm/config/ /app/config/

# Set environment
ENV CONFIG_PATH=/app/config
ENV YAMA_EVENT_BUS=/tmp/yama-event.sock
# WebSocket URL takes precedence if set
# ENV YAMA_EVENT_BUS_URL=ws://event-bus:8765
ENV RUST_LOG=info

# Run as non-root
USER vlm
WORKDIR /app

# Health check - just verify process is running
HEALTHCHECK --interval=30s --timeout=10s --start-period=120s --retries=3 \
    CMD pgrep yama-vlm || exit 1

CMD ["yama-vlm"]
