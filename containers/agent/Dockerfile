# Yama Agent Runtime Container
# Platform-agnostic LLM agent execution environment

FROM rust:1.83-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY containers/agent/ containers/agent/

# Build the agent
RUN cargo build --release -p yama-agent

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/yama-agent /app/

# Create directories
RUN mkdir -p /app/skills /app/config

# Health check endpoint
HEALTHCHECK --interval=5s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Environment
ENV RUST_LOG=info
ENV SKILLS_PATH=/app/skills
ENV CONFIG_PATH=/app/config

EXPOSE 8080

ENTRYPOINT ["/app/yama-agent"]
