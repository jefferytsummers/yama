# Yama Browser Tool
# Web browsing capabilities for agents

FROM rust:1.83-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    protobuf-compiler \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY containers/tools/browser/ containers/tools/browser/

RUN cargo build --release -p yama-tool-browser

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/yama-tool-browser /app/

ENV RUST_LOG=info

HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080

ENTRYPOINT ["/app/yama-tool-browser"]
