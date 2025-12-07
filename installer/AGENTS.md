# Agent Guidelines for RegicideOS Installer

## Build Commands
- **Build**: `cargo build --release`
- **Run**: `cargo run --bin installer`
- **Test**: `cargo test`
- **Single test**: `cargo test <test_name>`
- **Lint**: `cargo clippy -- -D warnings`
- **Format**: `cargo fmt`
- **Coverage**: `cargo tarpaulin --out Html`

## Code Style Guidelines

### Imports
- Group imports: std, external crates, internal modules
- Use `use anyhow::{bail, Result}` for error handling
- Prefer `std::process::Command as ProcessCommand` to avoid conflicts

### Naming Conventions
- Functions: `snake_case`
- Types/Structs: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- File paths: use absolute paths, validate with `validate_safe_path()`

### Error Handling
- Use `anyhow::Result<T>` for fallible functions
- Use `bail!()` for early returns with context
- Use `with_context()` for error enrichment
- Sanitize all error messages with `sanitize_error_message()`

### Security Requirements
- All external commands must use `execute()` or `execute_safe_command()`
- Validate all user inputs with regex patterns
- Use path traversal protection for file operations
- No hardcoded secrets or credentials

### Testing
- Write unit tests for all public functions in `#[cfg(test)]` modules
- Integration tests in separate `integration_tests` modules
- Test both success and failure cases
- Use `assert!` macros for assertions

### Code Organization
- Keep functions under 50 lines when possible
- Use descriptive variable names
- Add security comments for sensitive operations
- Follow Rust 2021 edition conventions