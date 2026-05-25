# RegicideOS Unified Build Commands

# Build all Rust crates in the workspace
build:
    cargo build --workspace --release

# Build debug versions
debug:
    cargo build --workspace

# Run all tests across the workspace
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Run clippy lints (treats warnings as errors in CI)
lint:
    cargo clippy --workspace -- -D warnings

# Auto-fix warnings and formatting
fix:
    cargo fix --workspace --allow-dirty
    cargo fmt --workspace

# Check formatting
check-fmt:
    cargo fmt --workspace -- --check

# Clean build artifacts
clean:
    cargo clean --workspace

# Build the OS installer binary
build-installer:
    cargo build --release -p installer

# Build btrmind AI agent
build-btrmind:
    cargo build --release -p btrmind

# Build portcl AI agent
build-portcl:
    cargo build --release -p portcl

# Build everything including ISO (requires Gentoo/Catalyst)
build-iso:
    cd build-system/catalyst && sudo ./build.sh

# Run Dagger CI/CD pipeline (requires Dagger)
dagger-build:
    dagger run python build-system/dagger_pipeline.py

# Full CI check (build, test, lint)
ci: lint test build
    @echo "✓ All CI checks passed"
