# Code Style and Conventions

## Rust Style
- Follow standard Rust conventions (rustfmt)
- Edition 2021
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting

## Naming Conventions
- **Crates**: kebab-case (`stream-aggregator-core`)
- **Types**: PascalCase (`StreamInfo`, `PlatformProvider`)
- **Functions/Methods**: snake_case (`fetch_stream`, `get_streams`)
- **Constants**: SCREAMING_SNAKE_CASE
- **Modules**: snake_case

## Code Organization
- Use modules to organize code (`mod config`, `mod providers`)
- Export public API through `lib.rs`
- Keep `main.rs` minimal and clean
- Separate concerns (config, providers, handlers, etc.)

## Async Patterns
- Use `async-trait` for async traits
- All I/O operations are async
- Use `Arc` for shared ownership of providers and stores
- Use `tokio::spawn` for concurrent operations

## Error Handling
- Use `thiserror` for error types
- Use `anyhow` for application-level errors
- Provide descriptive error messages
- Use `Result<T, E>` return types

## Documentation
- Use `///` doc comments for public APIs
- Include examples in doc comments where helpful
- Document module purposes with `//!`
- Keep comments concise and meaningful

## Traits and Abstractions
- Use trait-based abstractions (`PlatformProvider`, `StreamStore`)
- Implement `Default` where sensible
- Use `#[async_trait]` for async traits
- Keep trait methods focused and composable
