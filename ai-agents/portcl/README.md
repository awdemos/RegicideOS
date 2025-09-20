# PortCL - Portage Continual Learning Agent

**PortCL** is a reinforcement learning agent for Gentoo Portage that optimizes package management operations using continual learning techniques. It's the second AI agent in the RegicideOS ecosystem, working alongside BtrMind to provide comprehensive system optimization.

## Overview

PortCL monitors Portage operations, system metrics, and package dependencies to learn optimal strategies for:
- Compilation parallelism adjustment
- Package build order optimization
- Operation scheduling for off-peak performance
- Dependency pre-fetching
- Package cleanup and maintenance

## Features

- **Continual Learning**: Maintains knowledge from previous operations while adapting to new system states
- **Real-time Monitoring**: Tracks Portage metrics and system performance
- **Safe Actions**: Includes rollback mechanisms and safety checks
- **Service Integration**: Runs as systemd/OpenRC service
- **Configurable**: Extensive configuration options for different environments

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Portage         │───▶│ Continual RL     │───▶│ Action          │
│ Monitoring      │    │ Engine           │    │ Executor        │
└─────────────────┘    └──────────────────┘    └─────────────────┘
        ▲                                              │
        │                                              ▼
┌─────────────────┐                          ┌─────────────────┐
│ Reward          │                          │ Post-Action     │
│ Calculator      │◀─────────────────────────│ Feedback Loop   │
└─────────────────┘                          └─────────────────┘
```

## Installation

### Prerequisites

- Gentoo Linux system
- Rust 1.75+
- libtorch (for PyTorch bindings)
- Portage package manager

### System Dependencies

```bash
# Install libtorch (required for tch crate)
# See: https://github.com/LaurentMazare/tch-rs/blob/main/README.md

# Create user and directories
sudo useradd -r -s /bin/false portcl
sudo mkdir -p /etc/portcl /var/lib/portcl /var/log/portcl
sudo chown portcl:portcl /etc/portcl /var/lib/portcl /var/log/portcl
```

### Build and Install

```bash
# Clone repository
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS/ai-agents/portcl

# Build
cargo build --release

# Install binary and configuration
sudo cp target/release/portcl /usr/bin/
sudo cp config/default.toml /etc/portcl/config.toml
sudo cp systemd/portcl.service /etc/systemd/system/
sudo cp systemd/portcl /etc/init.d/

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable portcl
sudo systemctl start portcl
```

## Configuration

PortCL uses TOML configuration files. The main configuration is located at `/etc/portcl/config.toml`.

### Key Configuration Sections

- **monitoring**: Portage monitoring settings
- **rl**: Reinforcement learning parameters
- **actions**: Action execution and safety settings
- **safety**: System safety limits and checks
- **general**: General service configuration

See `config/default.toml` for all available options.

## Usage

### Command Line Interface

```bash
# Run the agent
sudo portcl run

# Validate configuration
sudo portcl validate

# Show status
sudo portcl status

# Test Portage integration
sudo portcl test-portage

# Enable verbose logging
sudo portcl --verbose run
```

### Service Management

```bash
# Systemd
sudo systemctl start portcl
sudo systemctl stop portcl
sudo systemctl status portcl
sudo journalctl -u portcl -f

# OpenRC
sudo rc-service portcl start
sudo rc-service portcl stop
sudo rc-service portcl status
```

## Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with specific features
cargo run --features test-utils
```

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration

# Run specific test
cargo test test_portage_integration
```

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── error.rs             # Error handling
├── config/              # Configuration management
│   ├── mod.rs
│   ├── settings.rs      # Configuration structs
│   └── validation.rs    # Configuration validation
├── monitor/             # Portage monitoring
├── rl_engine/           # Reinforcement learning
├── actions/             # Action execution
└── utils/               # Utility functions

config/                  # Configuration files
systemd/                 # Service files
tests/                   # Test suite
```

## Monitoring and Logging

### Logs

- **Service logs**: `/var/log/portcl.log`
- **Systemd journal**: `journalctl -u portcl`
- **Structured logging**: JSON format with tracing

### Metrics

PortCL collects and maintains metrics on:
- Package installation success rates
- Compilation times and resource usage
- Disk usage during operations
- System load and performance
- Action execution results

## Safety Features

- **Dry-run mode**: Test actions without execution
- **Rollback capability**: Revert failed actions
- **System checks**: Validate system state before actions
- **Critical package protection**: Protect essential system packages
- **Resource limits**: Prevent system overload

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement changes with tests
4. Ensure all tests pass
5. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Support

- **Issues**: GitHub Issues
- **Documentation**: RegicideOS Handbook
- **Community**: RegicideOS Forums

---

**Part of the RegicideOS AI Agent Ecosystem**