# AGENTS.md

## Build/Test Commands

- `cargo build` - Build the project
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test (e.g., `cargo test library_test`)
- `cargo clippy` - Lint code
- `cargo fmt` - Format code

## Code Style

- **Imports**: Group std, external crates, then local modules with blank lines between
- **Formatting**: Use `cargo fmt` - tabs for indentation, snake_case for variables/functions. DO NOT use emojis at all.
- **Types**: Explicit types preferred, use `Result<T, E>` for error handling with `thiserror`
- **Naming**: snake_case for functions/variables, PascalCase for types, SCREAMING_SNAKE_CASE for constants
- **Error Handling**: Use `Result` types, `thiserror` for custom errors, `anyhow` for application errors
- **Async**: Use `async/await`, prefer `tokio` primitives, avoid blocking operations
- **Database**: Use sqlx, async queries, proper error propagation
- **Comments**: Minimal inline comments, focus on why not what, no TODO comments in production code
