# RegicideOS Dagger CI

This directory contains the Dagger CI pipeline for RegicideOS using the Go SDK.

## Pipeline Stages

1. **Rust Components Build** - Builds installer and AI agents with caching
2. **Security Scanning** - Runs Trivy, cargo audit, and hadolint
3. **Overlay Testing** - Tests overlay in real Gentoo environment
4. **AI Agents Testing** - Tests AI agents with simulated environments

## Usage

### Prerequisites

Install Dagger CLI:
```bash
# On macOS
brew install dagger/tap/dagger

# On Linux
curl -L https://dl.dagger.io/dagger/install.sh | sh
```

### Run CI Pipeline

```bash
# From project root
cd ci
go mod tidy
dagger run go run main.go
```

### Run Individual Stages

The pipeline automatically runs all stages, but you can customize by modifying `main.go`.

## Features

- **Caching**: Cargo registry and build artifacts are cached for speed
- **Security-First**: Follows AGENTS.md requirements with security scanning
- **Multi-Platform**: Tests across Gentoo environment and Rust toolchain
- **AI-Aware**: Special handling for AI agents and BTRFS requirements

## Architecture

The pipeline leverages Dagger's container orchestration to:
- Build Rust components in parallel
- Run security scans with proper vulnerability detection
- Test overlay installation in actual Gentoo containers
- Validate AI agents without requiring root privileges

This replaces the existing shell-based Docker tests with a more robust, cacheable pipeline.
