# StreamAggregator Dockerfile
#
# Multi-stage build for minimal image size
# Supports: Docker, Kubernetes, Fly.io, Railway, Coolify, etc.
#
# Build:
#   docker build -t stream-aggregator .
#
# Run with SQLite:
#   docker run -p 8080:8080 -v ./data:/data -e DATABASE_URL=/data/streams.db stream-aggregator

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM rust:1.92-trixie AS builder

WORKDIR /app

# Install build dependencies (including cmake for boring-sys/wreq)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary with SQLite support
RUN cargo build --release --package stream-aggregator --features diesel-store

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:trixie-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3t64 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 appuser
RUN mkdir -p /data && chown appuser:appuser /data

# Copy binary from builder
COPY --from=builder /app/target/release/stream-aggregator /usr/local/bin/

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

# ============================================================================
# Alternative: Build with libSQL support (for Turso)
# ============================================================================
FROM rust:1.92-trixie AS builder-libsql

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build with libSQL support instead of SQLite
RUN cargo build --release --package stream-aggregator --features diesel-store

# ============================================================================
# Runtime for libSQL version
# ============================================================================
FROM debian:trixie-slim AS runtime-libsql

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3t64 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 appuser
RUN mkdir -p /data && chown appuser:appuser /data
USER appuser

COPY --from=builder-libsql /app/target/release/stream-aggregator /usr/local/bin/

ENV HOST=0.0.0.0
ENV PORT=8080
ENV RUST_LOG=info
ENV STORE_BACKEND=diesel

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["stream-aggregator"]
