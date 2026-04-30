# BtrMind - AI-Powered BTRFS Storage Monitoring

**BtrMind** is an AI-powered storage monitoring agent that uses reinforcement learning to optimize BTRFS filesystem health and disk space usage. It's part of the RegicideOS AI system management suite.

## Features

- **Real-time BTRFS monitoring** with disk usage, metadata, and fragmentation tracking
- **Reinforcement Learning optimization** using Deep Q-Networks (DQN)
- **Autonomous cleanup actions** including temp file removal, compression, and snapshot management
- **Configurable thresholds** for warning, critical, and emergency states
- **Systemd integration** for reliable service management
- **Comprehensive logging** with structured output

## Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- Linux system with systemd
- BTRFS filesystem (recommended, but works with any filesystem)
- Root access for installation

### Installation

```bash
# Clone and build
git clone https://github.com/awdemos/RegicideOS.git
cd RegicideOS/ai-agents/btrmind

# Build and install
cargo build --release
sudo ./install.sh
```

### Usage

```bash
# Start the service
sudo systemctl enable btrmind
sudo systemctl start btrmind

# Check status
sudo systemctl status btrmind

# View logs
sudo journalctl -u btrmind -f

# Manual commands
btrmind analyze              # Analyze current storage state
btrmind cleanup --aggressive # Manual cleanup
btrmind stats               # Show AI performance stats
btrmind config              # Validate configuration
```

## How It Works

### 1. Monitoring
BtrMind continuously monitors:
- **Disk Usage**: Overall filesystem utilization percentage
- **Free Space Trends**: Rate of space consumption over time  
- **Metadata Usage**: BTRFS-specific metadata overhead
- **Fragmentation**: Filesystem fragmentation levels

### 2. AI Decision Making
The reinforcement learning agent:
- **Observes** current system state (4-dimensional state space)
- **Selects** actions based on learned Q-values with ε-greedy exploration
- **Executes** storage optimization actions
- **Learns** from the results using reward feedback

### 3. Actions
Available optimization actions:
- **Delete Temp Files**: Clean `/tmp`, `/var/tmp`, cache directories
- **Compress Files**: BTRFS compression and defragmentation
- **Balance Metadata**: BTRFS metadata reorganization
- **Cleanup Snapshots**: Remove old BTRFS snapshots
- **No Operation**: Monitoring only

### 4. Reward Function
The AI learns through this reward system:
```rust
reward = space_freed * 10.0 - usage_penalties + efficiency_bonuses
```

- **Positive rewards** for freeing disk space
- **Penalties** for high usage (>85%, >95%, >98%)
- **Bonuses** for sustained improvements

## Configuration

Edit `/etc/btrmind/config.toml`:

```toml
[monitoring]
target_path = "/"           # Path to monitor
poll_interval = 60          # Seconds between checks

[thresholds]  
warning_level = 85.0        # Warning threshold (%)
critical_level = 95.0       # Critical threshold (%)
emergency_level = 98.0      # Emergency threshold (%)

[actions]
enable_compression = true   # Enable BTRFS compression
enable_temp_cleanup = true  # Enable temp file cleanup
temp_paths = ["/tmp", "/var/cache"]

[learning]
exploration_rate = 0.1      # AI exploration vs exploitation
learning_rate = 0.001       # Neural network learning rate
model_path = "/var/lib/btrmind/model.safetensors"
```

## AI Architecture

### Neural Network
- **Input**: 4-dimensional state vector (normalized)
- **Hidden Layers**: 3 layers × 128 neurons with ReLU activation
- **Output**: Q-values for 5 possible actions
- **Framework**: Candle (pure Rust ML framework)

### Learning Algorithm
- **Deep Q-Network (DQN)** with experience replay
- **Target Network** for stable training
- **ε-greedy exploration** with decay
- **Experience Buffer** (10,000 transitions)

### Continual Learning
- **Model persistence** across restarts
- **Online adaptation** to changing usage patterns
- **Catastrophic forgetting prevention** through experience replay

## Performance

### Resource Usage
- **CPU**: <2% (idle), <10% (peak)
- **Memory**: <50MB RAM
- **Disk**: <50MB for models and logs

### Response Times
- **Monitoring cycle**: ~500ms end-to-end
- **Action execution**: 1-30 seconds depending on action
- **Model updates**: <100ms

### Accuracy Targets
- **False positive rate**: <1% for critical alerts
- **Learning convergence**: Within 7 days of deployment
- **Storage optimization**: 10-20% improvement in available space

## Development

### Project Structure
```
btrmind/
├── src/
│   ├── main.rs          # CLI and main application logic
│   ├── config.rs        # Configuration management
│   ├── btrfs.rs         # BTRFS monitoring and metrics
│   ├── learning.rs      # Reinforcement learning implementation  
│   └── actions.rs       # Storage optimization actions
├── config/
│   └── btrmind.toml     # Default configuration
├── systemd/
│   └── btrmind.service  # Systemd service definition
└── tests/               # Unit and integration tests
```

### Testing

```bash
# Run unit tests
cargo test

# Test with dry-run mode
btrmind --dry-run analyze
btrmind --dry-run cleanup

# Integration testing
sudo systemctl start btrmind
# Monitor logs for learning progress
```

### Contributing

1. **Fork** the repository
2. **Create** a feature branch
3. **Add tests** for new functionality
4. **Run** `cargo fmt` and `cargo clippy`
5. **Submit** a pull request

## Monitoring & Debugging

### Health Checks
```bash
# Service status
systemctl is-active btrmind

# Configuration validation
btrmind config

# AI learning progress
journalctl -u btrmind | grep "Learning update"

# Storage analysis
btrmind analyze
```

### Common Issues

**High CPU usage**: Reduce `poll_interval` or disable compression actions

**Learning not improving**: Check reward function parameters and exploration rate

**Actions not executing**: Verify permissions and enable actions in config

**BTRFS commands failing**: Ensure BTRFS tools are installed and filesystem is mounted

## Integration with RegicideOS

BtrMind is part of the RegicideOS AI ecosystem:
- **Coordination** with PortCL (package management AI)
- **Knowledge sharing** through inter-agent communication
- **System-wide optimization** as part of autonomous OS management

## License

GPL-3.0 - See [LICENSE](../../LICENSE) for details.

## Support

- **Documentation**: [RegicideOS Handbook](../../Handbook.md)
- **Issues**: [GitHub Issues](https://github.com/awdemos/RegicideOS/issues)
- **Discussions**: [GitHub Discussions](https://github.com/awdemos/RegicideOS/discussions)

---

**BtrMind** - Autonomous Storage Intelligence for RegicideOS 🤖📊
