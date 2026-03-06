# Yama VLM Container
# Vision Language Model inference for video analysis

# Build stage
FROM rust:1.82 as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY containers/vlm/ containers/vlm/

# Build the VLM container
RUN cargo build --release -p yama-vlm

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-libav \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false vlm

# Create directories
RUN mkdir -p /app/config /app/models && \
    chown -R vlm:vlm /app

# Copy binary
COPY --from=builder /app/target/release/yama-vlm /usr/local/bin/

# Copy default config
COPY containers/vlm/config/vlm.toml /app/config/

# Set environment
ENV CONFIG_PATH=/app/config
ENV YAMA_EVENT_BUS=/tmp/yama-event.sock
ENV RUST_LOG=info

# Run as non-root
USER vlm
WORKDIR /app

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD test -S $YAMA_EVENT_BUS || exit 1

CMD ["yama-vlm"]
