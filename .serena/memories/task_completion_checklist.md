# Task Completion Checklist

When completing a coding task:

1. **Build Check**
   ```bash
   cargo check --workspace
   ```
   Ensure all code compiles without errors

2. **Run Tests**
   ```bash
   cargo test --workspace
   ```
   Ensure all tests pass

3. **Format Code**
   ```bash
   cargo fmt
   ```
   Apply standard Rust formatting

4. **Lint Check** (Optional but recommended)
   ```bash
   cargo clippy -- -D warnings
   ```
   Check for common mistakes and improvements

5. **Verify Changes**
   - Test the specific functionality you added/modified
   - Check that error cases are handled properly
   - Verify logging/tracing is appropriate

6. **Documentation**
   - Update relevant docs if API changed
   - Add doc comments to new public items
   - Update QUICKSTART.md or STATUS.md if needed

## Notes
- It's OK to have dead code warnings during development
- Focus on getting tests passing first
- Release builds take longer but produce optimized binaries
