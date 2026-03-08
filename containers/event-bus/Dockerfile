# Yama Event Bus Container
# WebSocket pub/sub server for inter-service communication

# Build stage
FROM rust:1.85 as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files first
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members (needed for workspace resolution)
COPY shared/ shared/
COPY containers/ containers/

# Create stub directories for platform crates (not needed for event-bus)
RUN mkdir -p platform/apple/host/src platform/apple/video/src platform/apple/inference/src \
    && echo "fn main() {}" > platform/apple/host/src/main.rs \
    && echo "" > platform/apple/video/src/lib.rs \
    && echo "" > platform/apple/inference/src/lib.rs
COPY platform/apple/host/Cargo.toml platform/apple/host/
COPY platform/apple/video/Cargo.toml platform/apple/video/
COPY platform/apple/inference/Cargo.toml platform/apple/inference/

# Build the event bus
RUN cargo build --release -p yama-event-bus

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false eventbus

# Copy binary
COPY --from=builder /app/target/release/yama-event-bus /usr/local/bin/

# Set environment
ENV EVENT_BUS_BIND=0.0.0.0:8765
ENV RUST_LOG=info

# Expose port
EXPOSE 8765

# Run as non-root
USER eventbus

# Health check
HEALTHCHECK --interval=10s --timeout=5s --start-period=5s --retries=3 \
    CMD nc -z localhost 8765 || exit 1

CMD ["yama-event-bus"]
