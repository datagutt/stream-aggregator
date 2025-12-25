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
# Stage 1: Build Dependencies (cached separately from source)
# ============================================================================
FROM rust:1.92-trixie AS planner

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

# Copy only dependency manifests for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/stream-aggregator/Cargo.toml ./crates/stream-aggregator/
COPY crates/stream-aggregator-api/Cargo.toml ./crates/stream-aggregator-api/
COPY crates/stream-aggregator-core/Cargo.toml ./crates/stream-aggregator-core/
COPY crates/stream-aggregator-scheduler/Cargo.toml ./crates/stream-aggregator-scheduler/
COPY crates/stream-aggregator-store/Cargo.toml ./crates/stream-aggregator-store/
COPY crates/stream-aggregator-migrator/Cargo.toml ./crates/stream-aggregator-migrator/
COPY crates/providers/stream-aggregator-provider-twitch/Cargo.toml ./crates/providers/stream-aggregator-provider-twitch/
COPY crates/providers/stream-aggregator-provider-youtube/Cargo.toml ./crates/providers/stream-aggregator-provider-youtube/
COPY crates/providers/stream-aggregator-provider-kick/Cargo.toml ./crates/providers/stream-aggregator-provider-kick/
COPY crates/providers/stream-aggregator-provider-dlive/Cargo.toml ./crates/providers/stream-aggregator-provider-dlive/
COPY crates/providers/stream-aggregator-provider-trovo/Cargo.toml ./crates/providers/stream-aggregator-provider-trovo/
COPY crates/providers/stream-aggregator-provider-tiktok/Cargo.toml ./crates/providers/stream-aggregator-provider-tiktok/
COPY crates/providers/stream-aggregator-provider-guac/Cargo.toml ./crates/providers/stream-aggregator-provider-guac/
COPY crates/providers/stream-aggregator-provider-angelthump/Cargo.toml ./crates/providers/stream-aggregator-provider-angelthump/
COPY crates/providers/stream-aggregator-provider-robotstreamer/Cargo.toml ./crates/providers/stream-aggregator-provider-robotstreamer/

# Create dummy lib.rs files for dependency compilation
RUN mkdir -p crates/stream-aggregator/src && echo "fn main() {}" > crates/stream-aggregator/src/main.rs
RUN mkdir -p crates/stream-aggregator-api/src && echo "" > crates/stream-aggregator-api/src/lib.rs
RUN mkdir -p crates/stream-aggregator-core/src && echo "" > crates/stream-aggregator-core/src/lib.rs
RUN mkdir -p crates/stream-aggregator-scheduler/src && echo "" > crates/stream-aggregator-scheduler/src/lib.rs
RUN mkdir -p crates/stream-aggregator-store/src && echo "" > crates/stream-aggregator-store/src/lib.rs
RUN mkdir -p crates/stream-aggregator-migrator/src && echo "fn main() {}" > crates/stream-aggregator-migrator/src/main.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-twitch/src && echo "" > crates/providers/stream-aggregator-provider-twitch/src/lib.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-youtube/src && echo "" > crates/providers/stream-aggregator-provider-youtube/src/lib.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-kick/src && echo "" > crates/providers/stream-aggregator-provider-kick/src/lib.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-dlive/src && echo "" > crates/providers/stream-aggregator-provider-dlive/src/lib.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-trovo/src && echo "" > crates/providers/stream-aggregator-provider-trovo/src/lib.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-tiktok/src && echo "" > crates/providers/stream-aggregator-provider-tiktok/src/lib.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-guac/src && echo "" > crates/providers/stream-aggregator-provider-guac/src/lib.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-angelthump/src && echo "" > crates/providers/stream-aggregator-provider-angelthump/src/lib.rs
RUN mkdir -p crates/providers/stream-aggregator-provider-robotstreamer/src && echo "" > crates/providers/stream-aggregator-provider-robotstreamer/src/lib.rs

# Pre-build dependencies only (this layer is heavily cached)
RUN cargo build --release --package stream-aggregator --features diesel-store

# ============================================================================
# Stage 2: Build Application
# ============================================================================
FROM rust:1.92-trixie AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    build-essential \
    libclang-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Copy pre-built dependencies from planner
COPY --from=planner /app/target ./target
COPY --from=planner /usr/local/cargo /usr/local/cargo

# Copy actual source code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build only the application code (dependencies already compiled)
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
