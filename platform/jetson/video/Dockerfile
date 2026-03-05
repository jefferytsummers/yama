# Yama Video Service - NVIDIA Jetson
# GStreamer + NVDEC for hardware-accelerated video decoding

# Use NVIDIA L4T base for JetPack compatibility
FROM nvcr.io/nvidia/l4t-base:r35.4.1 AS base

# Install build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-plugins-bad1.0-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-tools \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY platform/jetson/video/ platform/jetson/video/

# Build the service
RUN cargo build --release -p yama-video-jetson

# Runtime stage
FROM nvcr.io/nvidia/l4t-base:r35.4.1

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-tools \
    nvidia-l4t-gstreamer \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=base /app/target/release/yama-video /app/

# Health check
HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENV RUST_LOG=info
ENV GST_DEBUG=2

EXPOSE 8080

ENTRYPOINT ["/app/yama-video"]
