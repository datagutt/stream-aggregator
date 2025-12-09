# StreamAggregator - Development Commands

## Building
```bash
# Build all crates
cargo build

# Build release
cargo build --release

# Build specific crate
cargo build -p stream-aggregator
cargo build -p stream-aggregator-core
```

## Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p stream-aggregator-core
cargo test -p stream-aggregator-store
cargo test -p stream-aggregator-api

# Run tests with output
cargo test -- --nocapture
```

## Running
```bash
# Run main binary (requires Twitch credentials)
export TWITCH_CLIENT_ID="your_id"
export TWITCH_CLIENT_SECRET="your_secret"
cargo run --release

# Run with custom port
cargo run --release -- --port 3000

# Show help
cargo run -- --help
```

## Code Quality
```bash
# Check compilation
cargo check

# Check with all features
cargo check --all-features

# Run clippy
cargo clippy

# Format code
cargo fmt

# Format check
cargo fmt -- --check
```

## Workspace Operations
```bash
# Check entire workspace
cargo check --workspace

# Test entire workspace
cargo test --workspace

# Build all with release optimizations
cargo build --release --workspace
```
