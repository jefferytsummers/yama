# Yama Inference Service - NVIDIA Jetson
# TensorRT-LLM / llama.cpp with CUDA acceleration

# Use NVIDIA L4T ML base image for TensorRT support
FROM nvcr.io/nvidia/l4t-ml:r35.4.1-py3 AS base

# Install build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    cmake \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY platform/jetson/inference/ platform/jetson/inference/

# Build the service
RUN cargo build --release -p yama-inference-jetson

# Runtime stage
FROM nvcr.io/nvidia/l4t-ml:r35.4.1-py3

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=base /app/target/release/yama-inference /app/

# Create models directory
RUN mkdir -p /app/models

# Health check
HEALTHCHECK --interval=5s --timeout=3s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENV RUST_LOG=info
ENV MODEL_PATH=/app/models
ENV CUDA_VISIBLE_DEVICES=0

EXPOSE 8080

ENTRYPOINT ["/app/yama-inference"]
