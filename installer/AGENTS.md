# Agent Guidelines for RegicideOS Installer

**Scope**: `installer/`

## OVERVIEW
Safety-critical Rust OS installer. Builds the `installer` binary, validates user input and paths, partitions the target disk, and deploys the SquashFS image to a ROOTS partition.

## STRUCTURE
```
installer/
├── src/
│   ├── main.rs       # CLI, install orchestration, safe-command wrapper
│   ├── lib.rs        # Config/Partition types and validation helpers
│   ├── filesystem.rs # Partitioning and filesystem operations
│   ├── validation.rs # Input/path validation
│   └── logging.rs    # Sanitized logging
└── Cargo.toml
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add CLI argument | `src/main.rs` | Clap derive parser |
| Change partitioning | `src/filesystem.rs` | Wraps gdisk/parted/mkfs/cryptsetup |
| Validate config/path | `src/validation.rs` | `validate_safe_path()` blocks traversal |
| Add public helper | `src/lib.rs` | Add `#[cfg(test)]` unit tests |
| Sanitize output | `src/logging.rs` | Redacts paths, passwords, tokens |

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

## ANTI-PATTERNS
- **Do not use raw `std::process::Command`**: route all external commands through `execute()` or `execute_safe_command()`.
- **Do not allow path traversal**: every file path must be validated with `validate_safe_path()` before use.
- **Do not leak sensitive data in logs**: error messages must pass through `sanitize_error_message()`.
- **Do not write to block devices outside dry-run wrappers**: destructive operations are gated by safety checks and mocked in tests.
- **Do not bypass package-manager allowlist**: `dnf`/`apt`/`pacman` writes are blocked except for installing `gdisk`.
- **Do not disable or skip safety tests**: destructive-operation guards must remain under test.