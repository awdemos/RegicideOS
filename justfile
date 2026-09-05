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
    cargo fmt

# Check formatting
check-fmt:
    cargo fmt -- --check

# Clean build artifacts
clean:
    cargo clean --workspace

# Build the OS installer binary
build-installer:
    cargo build --release -p installer

# Build btrmind AI agent
build-btrmind:
    cargo build --release -p btrmind

# Build everything including ISO (requires Gentoo/Catalyst)
build-iso:
    cd build-system/catalyst && sudo ./build.sh

# Verify the container runtime is rootful (rootless Podman breaks Dagger engine)
check-runtime:
	@./scripts/check-container-runtime.sh

# Run Dagger CI/CD pipeline (requires Dagger and a rootful runtime)
dagger-build: check-runtime
	DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain

# Build a fresh bootable ISO from source (requires Dagger and a rootful runtime)
rebuild-iso:
	./scripts/rebuild-iso.sh

# Flash the built ISO to a USB device (pass DEVICE=/dev/sdX)
flash-usb DEVICE:
	./scripts/flash-usb.sh {{DEVICE}}

# Fix Dagger engine runtime issues on Fedora Atomic / Podman hosts
fix-dagger:
	./scripts/troubleshoot/fix-dagger.sh

# Fix DNS resolution inside Dagger/Podman containers
fix-dagger-dns:
	./scripts/troubleshoot/fix-dagger-dns.sh

# Full CI check (build, test, lint)
ci: lint test build
	@echo "✓ All CI checks passed"
