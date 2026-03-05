# Yama Code Executor Tool
# Sandboxed code execution environment

FROM rust:1.83-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY containers/tools/code-executor/ containers/tools/code-executor/

RUN cargo build --release -p yama-tool-code-executor

# Runtime image with language runtimes
FROM debian:bookworm-slim

# Install language runtimes
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    # Python
    python3 \
    python3-pip \
    # Node.js (via nodesource)
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    # Cleanup
    && rm -rf /var/lib/apt/lists/*

# Install common Python packages
RUN pip3 install --break-system-packages \
    numpy \
    pandas \
    requests \
    && rm -rf /root/.cache/pip

WORKDIR /app

COPY --from=builder /app/target/release/yama-tool-code-executor /app/

# Create workspace directory
RUN mkdir -p /workspace && chmod 777 /workspace

# Create non-root user for code execution
RUN useradd -m -s /bin/bash executor
USER executor

ENV RUST_LOG=info
ENV WORKSPACE=/workspace

HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080

ENTRYPOINT ["/app/yama-tool-code-executor"]
