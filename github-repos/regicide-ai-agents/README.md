# RegicideOS AI Agents

Collection of AI-powered agents and tools for RegicideOS system management and optimization.

## Overview

This repository contains the source code for RegicideOS AI agents:

- **BtrMind**: AI-powered BTRFS storage optimization
- **PortCL**: AI-driven package management (future)
- **SystemAI**: General system optimization (future)
- **NetworkAI**: Network performance optimization (future)

## Agents

### BtrMind
AI-powered storage optimization agent using reinforcement learning.

**Features:**
- BTRFS filesystem monitoring
- Predictive cleanup operations
- Learning from usage patterns
- Adaptive optimization strategies

**Installation:**
```bash
# Install from overlay
emerge app-misc/regicide-ai-tools

# Enable service
systemctl enable btrmind
systemctl start btrmind

# Monitor
btrmind analyze
btrmind stats
```

### PortCL (Future)
AI-driven package management optimization.

**Planned Features:**
- Build parallelism optimization
- Dependency analysis
- Cache management
- Performance prediction

## Building

### Prerequisites
- Rust 1.75+
- BTRFS tools (for BtrMind)
- systemd (for service integration)

### Quick Build
```bash
# Clone repository
git clone https://github.com/regicideos/regicide-ai-agents.git
cd regicide-ai-agents

# Build all agents
cargo build --release

# Build specific agent
cargo build --release --bin btrmind
```

### Testing
```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration

# Build and test with Docker
docker build -t regicide-ai-agents .
docker run --rm regicide-ai-agents cargo test
```

## Architecture

### Core Components
- **Learning Engine**: Reinforcement learning algorithms
- **Action Executor**: System operation handlers
- **Monitor**: System metrics collection
- **Configuration**: Agent and system settings

### Machine Learning
- Q-learning for decision making
- Experience replay for training
- Adaptive exploration strategies
- Performance reward functions

## Development

### Project Structure
```
├── btrmind/         # BTRFS optimization agent
├── portcl/          # Package management agent (future)
├── shared/          # Common utilities and libraries
├── ml/              # Machine learning components
├── config/          # Configuration management
├── tests/           # Test suites
└── docs/            # Documentation
```

### Adding New Agents
1. Create agent directory
2. Implement core traits
3. Add to workspace
4. Write tests
5. Update documentation

## Configuration

### BtrMind Configuration
```toml
[monitoring]
target_path = "/"
poll_interval = 60

[thresholds]
warning_level = 75.0
critical_level = 85.0

[learning]
exploration_rate = 0.1
learning_rate = 0.01
```

### Environment Variables
- `BTRMIND_CONFIG` - Configuration file path
- `RUST_LOG` - Logging level
- `BTRMIND_DATA_PATH` - Data storage directory

## Monitoring

### Service Management
```bash
# Check service status
systemctl status btrmind

# View logs
journalctl -u btrmind -f

# Monitor metrics
btrmind stats
```

### Performance Metrics
- Storage optimization efficiency
- Learning convergence rate
- Resource utilization
- Response times

## Contributing

### Development Setup
```bash
# Install development dependencies
cargo install cargo-watch cargo-audit

# Run development server
cargo watch -x run

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Testing Requirements
- 90%+ test coverage
- Integration tests for all features
- Performance benchmarks
- Security scanning

### Pull Request Process
1. Fork and create feature branch
2. Implement changes with tests
3. Update documentation
4. Pass all CI checks
5. Submit pull request

## License

GPL-3.0 - See [LICENSE](LICENSE) for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/regicideos/RegicideOS/issues)
- **Documentation**: [RegicideOS Handbook](https://docs.regicideos.com)
- **Discussions**: [GitHub Discussions](https://github.com/regicideos/RegicideOS/discussions)

---

**Building the future of intelligent system management** 🤖