# Holo Routing Test Suite Quickstart Guide

## Overview

This guide provides quick instructions for setting up and running the Holo routing test suite. The test suite validates all routing protocols (BGP, OSPF, IS-IS, BFD, VRRP), network topologies, performance benchmarks, and security compliance.

## Prerequisites

### System Requirements
- Linux system (Ubuntu 20.04+, Gentoo, or RegicideOS)
- Docker or Podman container runtime
- Rust toolchain 1.70+
- At least 4GB RAM and 2 CPU cores
- Network namespace support

### Software Dependencies
```bash
# Install container runtime
sudo apt update && sudo apt install -y docker.io
# OR
sudo apt install -y podman

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install network tools
sudo apt install -y iproute2 bridge-utils

# Install Containerlab
bash <(curl -sL https://get.containerlab.dev)
```

## Quick Setup

### 1. Clone Repository
```bash
git clone https://github.com/regicideos/RegicideOS.git
cd RegicideOS/specs/005-holo-routing
```

### 2. Install Test Dependencies
```bash
# Install Rust test dependencies
cargo install cargo-nextest cargo-tarpaulin

# Install test utilities
pip install pytest pyyaml
```

### 3. Set Up Environment
```bash
# Create test directories
mkdir -p tests/{unit,integration,network,security,performance,fixtures}

# Set permissions for network operations
sudo usermod -aG docker $USER
newgrp docker
```

## Running Tests

### Basic Protocol Tests
```bash
# Run all BGP tests
cargo test --package holo-bgp

# Run all OSPF tests
cargo test --package holo-ospf

# Run all protocol tests
cargo test --package holo-daemon --test protocol_tests
```

### Network Simulation Tests
```bash
# Deploy BGP test topology
containerlab deploy -t tests/network/topologies/bgp-test.clab.yml

# Deploy OSPF test topology
containerlab deploy -t tests/network/topologies/ospf-test.clab.yml

# Run network validation
python tests/network/validation/validate_bgp.py --topology bgp-test
```

### Performance Benchmarks
```bash
# Run routing performance benchmarks
cargo test --release bench --package holo-daemon

# Run convergence time tests
cargo test --release --test convergence_benchmarks

# Generate performance report
cargo test --release bench | tee performance-report.txt
```

### Security Tests
```bash
# Run container security scan
./tests/security/container-security.sh

# Run protocol authentication tests
cargo test security --package holo-bgp

# Run DoS protection tests
cargo test security --package holo-daemon --test dos_protection
```

### Coverage Reports
```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# Generate protocol-specific coverage
cargo tarpaulin --package holo-bgp --out Json
cargo tarpaulin --package holo-ospf --out Json
cargo tarpaulin --package holo-isis --out Json
```

## Test Categories

### Unit Tests
Fast tests that validate individual components:
```bash
# Run all unit tests
cargo test --lib

# Run specific protocol unit tests
cargo test bgp::session
cargo test ospf::lsa
cargo test isis::adjacency
```

### Integration Tests
Tests that validate component interactions:
```bash
# Run multi-protocol integration tests
cargo test --test integration_tests

# Run system integration tests
cargo test --test system_integration
```

### Network Tests
Tests that use network simulation:
```bash
# Deploy and test full mesh topology
containerlab deploy -t tests/network/topologies/full-mesh.clab.yml
python tests/network/scenarios/test_full_mesh.py

# Deploy and test hub-spoke topology
containerlab deploy -t tests/network/topologies/hub-spoke.clab.yml
python tests/network/scenarios/test_hub_spoke.py
```

## Network Topologies

### Available Topologies
- **bgp-test**: Simple 2-router BGP peering
- **ospf-test**: 3-router OSPF area
- **isis-test**: 2-router IS-IS adjacency
- **full-mesh**: 4-router full mesh BGP
- **hub-spoke**: 1 hub, 3 spokes OSPF
- **datacenter**: Leaf-spine data center fabric

### Topology Structure
```yaml
# Example: tests/network/topologies/bgp-test.clab.yml
name: holo-bgp-test
topology:
  nodes:
    router1:
      kind: linux
      image: ghcr.io/holo-routing/holo:latest
      cmd: holo-daemon --config /etc/holo/bgp-router1.conf
    router2:
      kind: linux
      image: ghcr.io/holo-routing/holo:latest
      cmd: holo-daemon --config /etc/holo/bgp-router2.conf
  links:
    - endpoints: ["router1:eth1", "router2:eth1"]
```

## Configuration Examples

### BGP Configuration
```toml
# /etc/holo/bgp-router1.conf
[routing]
router_id = "10.0.1.1"

[routing.bgp]
as_number = 65001
local_preference = 100

[[routing.bgp.neighbors]]
address = "10.0.1.2"
remote_as = 65002
```

### OSPF Configuration
```toml
# /etc/holo/ospf-router1.conf
[routing]
router_id = "10.0.1.1"

[routing.ospf]
reference_bandwidth = 1000000

[[routing.ospf.areas]]
area_id = 0
networks = ["10.0.1.0/24"]
```

## Test Results

### Understanding Test Output
```bash
# Example test output
test bgp::session::test_session_establishment ... ok
test bgp::session::test_route_advertisement ... ok
test ospf::neighbor::test_hello_packets ... ok
test isis::adjacency::test_lsp_flooding ... ok

# Summary
test result: ok. 45 passed; 0 failed; 0 skipped; 0 measured
```

### Coverage Report Interpretation
- **90%+**: Excellent coverage
- **80-89%**: Good coverage
- **70-79%**: Acceptable coverage
- **<70%**: Needs improvement

### Performance Benchmarks
- **BGP convergence**: <2 seconds for 1,000 routes
- **OSPF convergence**: <1 second for area changes
- **Memory usage**: <200MB base + 1MB per 1,000 routes
- **CPU usage**: <5% idle, <20% peak load

## Troubleshooting

### Common Issues

#### Containerlab Deployment Fails
```bash
# Check container runtime
docker version
# OR
podman version

# Check network namespaces
ip netns list

# Clean up previous deployments
containerlab destroy -a
```

#### Permission Denied Errors
```bash
# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker

# Check file permissions
ls -la tests/network/topologies/
```

#### Test Timeouts
```bash
# Increase timeout in test configuration
export RUST_TEST_TIMEOUT=600
export CONTAINERLAB_TIMEOUT=300
```

#### Memory Issues
```bash
# Check available memory
free -h

# Limit parallel test execution
cargo test -- --test-threads=2
```

### Debug Mode
```bash
# Enable debug logging
RUST_LOG=debug cargo test

# Enable containerlab debug
containerlab deploy -t topology.yml --debug

# Preserve network resources for debugging
export HOLO_TEST_PRESERVE=1
```

## Continuous Integration

### GitHub Actions Example
```yaml
name: Holo Routing Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - name: Install Containerlab
      run: bash <(curl -sL https://get.containerlab.dev)
    - name: Run Tests
      run: |
        cargo test --package holo-bgp
        cargo test --package holo-ospf
        cargo test --package holo-isis
    - name: Generate Coverage
      run: cargo tarpaulin --out Xml
    - name: Upload Coverage
      uses: codecov/codecov-action@v1
```

## Best Practices

### Test Development
1. Write tests before implementation (TDD)
2. Use descriptive test names
3. Test both success and failure scenarios
4. Mock external dependencies appropriately
5. Keep tests fast and deterministic

### Network Testing
1. Clean up network resources after tests
2. Use isolated network namespaces
3. Test with realistic topologies
4. Validate both data plane and control plane
5. Test failure scenarios

### Performance Testing
1. Establish baseline measurements
2. Test with realistic data volumes
3. Monitor resource usage
4. Test under various load conditions
5. Document performance expectations

## Getting Help

### Documentation
- [Holo Routing Documentation](https://github.com/holo-routing/holo)
- [Containerlab Documentation](https://containerlab.dev/)
- [RegicideOS Handbook](../Handbook.md)

### Community
- GitHub Issues: [Report bugs](https://github.com/regicideos/RegicideOS/issues)
- Discussions: [Q&A](https://github.com/regicideos/RegicideOS/discussions)
- Matrix Chat: [#regicideos:matrix.org](https://matrix.to/#/#regicideos:matrix.org)

### Support
For test suite specific issues:
1. Check existing GitHub issues
2. Provide detailed error logs
3. Include system information
4. Describe reproduction steps
5. Attach configuration files if applicable

---

**Next Steps**:
1. Run the basic protocol tests to verify setup
2. Deploy a simple network topology
3. Run performance benchmarks
4. Explore advanced test scenarios
5. Contribute test improvements

**Remember**: The test suite is designed to be extensible. Feel free to add new topologies, test scenarios, and validation scripts to improve coverage and reliability.