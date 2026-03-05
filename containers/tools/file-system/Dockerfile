# Yama File System Tool
# File operations for agents

FROM rust:1.83-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY containers/tools/file-system/ containers/tools/file-system/

RUN cargo build --release -p yama-tool-file-system

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/yama-tool-file-system /app/

RUN mkdir -p /workspace && chmod 755 /workspace

ENV RUST_LOG=info
ENV WORKSPACE=/workspace

HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080

ENTRYPOINT ["/app/yama-tool-file-system"]
