# Yama Inference Service - Apple Silicon
# TTRA/llama.cpp with Metal acceleration

FROM rust:1.83-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    protobuf-compiler \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY platform/apple/inference/ platform/apple/inference/

# Build the service
RUN cargo build --release -p yama-inference-apple

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/yama-inference /app/

# Create models directory
RUN mkdir -p /app/models

# Health check endpoint
HEALTHCHECK --interval=5s --timeout=3s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Environment
ENV RUST_LOG=info
ENV MODEL_PATH=/app/models

EXPOSE 8080

ENTRYPOINT ["/app/yama-inference"]
