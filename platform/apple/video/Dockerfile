# Yama Video Service - Apple Silicon
# GStreamer + VideoToolbox for hardware-accelerated video decoding

FROM rust:1.83-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-plugins-bad1.0-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY platform/apple/video/ platform/apple/video/

# Build the service
RUN cargo build --release -p yama-video-apple

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-rtsp \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/yama-video /app/

# Health check endpoint
HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Environment
ENV RUST_LOG=info
ENV GST_DEBUG=2

EXPOSE 8080

ENTRYPOINT ["/app/yama-video"]
