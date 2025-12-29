# StreamAggregator Dockerfile
#
# Multi-stage build for minimal image size
# Includes: stream-aggregator server + stream-aggregator-migrator + TikTok bridge
# Supports: Docker, Kubernetes, Fly.io, Railway, Coolify, etc.
#
# Build:
#   docker build -t stream-aggregator .
#
# Run server:
#   docker run -p 8080:8080 -v ./data:/data -e DATABASE_URL=/data/streams.db stream-aggregator
#
# Run migrator:
#   docker run -v ./data:/data -v ./people.json:/data/people.json stream-aggregator \
#     stream-aggregator-migrator -i /data/people.json -d /data/streams.db

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM rust:1.92-trixie AS builder

WORKDIR /app

# Install build dependencies (including cmake for boring-sys/wreq, libclang for diesel)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    build-essential \
    libclang-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binaries with SQLite support
# Use sccache-like environment to leverage Docker layer caching
ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_RETRY=10
RUN cargo build --release --package stream-aggregator --features diesel-store
RUN cargo build --release --package stream-aggregator-migrator

# ============================================================================
# Stage 2: Node.js Bridge Builder
# ============================================================================
FROM node:24-slim AS node-builder

WORKDIR /bridge

# Copy TikTok bridge files
COPY crates/providers/stream-aggregator-provider-tiktok/nodejs-bridge/package*.json ./
COPY crates/providers/stream-aggregator-provider-tiktok/nodejs-bridge/index.js ./

# Install dependencies (production only)
RUN npm ci --omit=dev

# ============================================================================
# Stage 3: Runtime
# ============================================================================
FROM debian:trixie-slim AS runtime

WORKDIR /app

# Install runtime dependencies including Node.js
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3t64 \
    curl \
    nodejs \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 appuser
RUN mkdir -p /data && chown appuser:appuser /data

# Copy binaries from builder
COPY --from=builder /app/target/release/stream-aggregator /usr/local/bin/
COPY --from=builder /app/target/release/stream-aggregator-migrator /usr/local/bin/

# Copy TikTok bridge from node-builder
COPY --from=node-builder --chown=appuser:appuser /bridge /app/nodejs-bridge

# Switch to non-root user
USER appuser

# Environment variables (can be overridden)
ENV HOST=0.0.0.0
ENV PORT=8080
ENV RUST_LOG=info
ENV STORE_BACKEND=diesel
ENV DATABASE_URL=/data/streams.db

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["stream-aggregator"]
