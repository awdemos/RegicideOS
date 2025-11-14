# Implementation Plan: Holo Routing Overlay

**Based on**: `/home/a/code/RegicideOS/specs/005-holo-routing/spec.md`
**Created**: November 14, 2025
**Estimated Duration**: 4-5 weeks
**Priority**: HIGH

## 🎯 Implementation Overview

This plan outlines the implementation of Holo routing as a Gentoo overlay for RegicideOS, providing users with a modern, memory-safe, automation-ready networking stack. The implementation will package all Holo components, integrate them with RegicideOS service management, and provide comprehensive configuration and testing frameworks.

## 📋 Project Structure

```
/home/a/code/RegicideOS/overlays/regicide-holo/
├── metadata/
│   ├── about.xml                      # Overlay metadata
│   └── layout.conf                    # Repository layout
├── profiles/
│   ├── categories                     # Package categories
│   ├── package.use/                   # USE flag defaults
│   │   └── holo-defaults             # Default USE flags
│   ├── package.keywords/              # Keyword assignments
│   │   └── holo-keywords              # ~arch keywords
│   └── repo_name                      # Repository name
├── net-misc/
│   ├── holo-daemon/
│   │   ├── holo-daemon-9999.ebuild    # Live ebuild
│   │   └── files/
│   │       └── holo-daemon.conf       # Default config
│   ├── holo-cli/
│   │   └── holo-cli-9999.ebuild
│   ├── holo-bgp/
│   │   └── holo-bgp-9999.ebuild
│   ├── holo-ospf/
│   │   └── holo-ospf-9999.ebuild
│   ├── holo-isis/
│   │   └── holo-isis-9999.ebuild
│   ├── holo-bfd/
│   │   └── holo-bfd-9999.ebuild
│   ├── holo-vrrp/
│   │   └── holo-vrrp-9999.ebuild
│   ├── holo-northbound/
│   │   └── holo-northbound-9999.ebuild
│   └── holo-yang/
│       └── holo-yang-9999.ebuild
├── app-emulation/
│   └── containerlab-topologies-holo/
│       ├── containerlab-topologies-holo-1.0.0.ebuild
│       └── files/
│           ├── bgp-test.clab.yml     # BGP test topology
│           ├── ospf-test.clab.yml    # OSPF test topology
│           └── full-mesh.clab.yml     # Full mesh test
├── sets/
│   ├── holo-base                      # Core components only
│   ├── holo-full                      # Complete routing suite
│   └── holo-testing                   # Development and testing
├── files/
│   ├── holo.service                   # Systemd service
│   ├── holo.initd                     # OpenRC init script
│   ├── holo.conf                      # Main configuration
│   └── holo.apparmor                  # AppArmor profile
├── tests/
│   ├── unit/                          # Unit tests
│   ├── integration/                   # Integration tests
│   └── security/                      # Security tests
├── docs/
│   ├── INSTALL.md                     # Installation guide
│   ├── CONFIGURATION.md               # Configuration guide
│   ├── PROTOCOLS.md                   # Protocol-specific guides
│   └── AUTOMATION.md                  # Automation examples
└── README.md                          # Overlay documentation
```

## 🗓️ Implementation Timeline

### **Week 1: Foundation Setup** (Priority: CRITICAL)

#### **Day 1-2: Overlay Structure**
- [ ] Create overlay directory structure
- [ ] Set up metadata files (about.xml, layout.conf)
- [ ] Define package categories and profiles
- [ ] Create basic README and documentation

#### **Day 3-4: Core Ebuilds**
- [ ] Create holo-daemon ebuild (main package)
- [ ] Create holo-cli ebuild (management interface)
- [ ] Set up live ebuild structure (9999 version)
- [ ] Define dependencies and USE flags

#### **Day 5-7: Protocol Modules**
- [ ] Create ebuilds for BGP, OSPF, IS-IS modules
- [ ] Create ebuilds for BFD, VRRP, LDP
- [ ] Create northbound and YANG modules
- [ ] Test basic compilation

### **Week 2: System Integration** (Priority: HIGH)

#### **Day 8-10: Service Integration**
- [ ] Create systemd service files
- [ ] Create OpenRC init scripts
- [ ] Implement user/group management
- [ ] Add security profiles (AppArmor)

#### **Day 11-12: Configuration Management**
- [ ] Create default configuration templates
- [ ] Implement YANG model integration
- [ ] Add configuration validation
- [ ] Create configuration examples

#### **Day 13-14: Package Sets**
- [ ] Define holo-base package set
- [ ] Define holo-full package set
- [ ] Define holo-testing package set
- [ ] Test package set installation

### **Week 3: Testing & Validation** (Priority: HIGH)

#### **Day 15-17: Containerlab Integration**
- [ ] Create containerlab topology ebuild
- [ ] Develop test topologies (BGP, OSPF, full-mesh)
- [ ] Implement automated testing framework
- [ ] Validate multi-protocol interoperability

#### **Day 18-19: Security Testing**
- [ ] Implement container security scanning
- [ ] Create security test suite
- [ ] Validate privilege separation
- [ ] Test DoS resistance

#### **Day 20-21: Performance Testing**
- [ ] Create performance benchmarks
- [ ] Test resource usage under load
- [ ] Validate convergence times
- [ ] Optimize configuration

### **Week 4: Documentation & Polish** (Priority: MEDIUM)

#### **Day 22-24: Documentation**
- [ ] Write comprehensive installation guide
- [ ] Create protocol-specific configuration guides
- [ ] Document automation examples
- [ ] Create troubleshooting guide

#### **Day 25-26: Integration Testing**
- [ ] Test with RegicideOS AI agents
- [ ] Validate system monitoring integration
- [ ] Test cross-platform compatibility
- [ ] Perform end-to-end validation

#### **Day 27-28: Final Polish**
- [ ] Review and optimize ebuilds
- [ ] Update documentation based on testing
- [ ] Prepare for overlay submission
- [ ] Create release notes

## 🔧 Technical Implementation

### **Core Ebuild Template**

```bash
# net-misc/holo-daemon/holo-daemon-9999.ebuild
EAPI=8

CRATES=""
inherit cargo systemd

DESCRIPTION="Holo Routing Daemon - Modern Rust-based routing suite"
HOMEPAGE="https://github.com/holo-routing/holo"
SRC_URI="https://github.com/holo-routing/holo/archive/${EGIT_COMMIT}.tar.gz -> ${P}.tar.gz
        $(cargo_crate_uris ${CRATES})"

LICENSE="Apache-2.0 MIT"
SLOT="0"
KEYWORDS="~amd64 ~arm64"
IUSE="systemd apparmor selinux test"

DEPEND="
    >=dev-lang/rust-1.70.0
    >=dev-libs/libyang-2.0.0
    >=dev-libs/protobuf-c-1.4.0
    net-misc/curl
    sys-apps/systemd
    sys-libs/ncurses
    dev-libs/openssl
"

RDEPEND="${DEPEND}
    acct-user/holo
    acct-group/holo
    apparmor? ( sys-libs/apparmor )
    selinux? ( sec-policy/selinux-holo )
"

BDEPEND="
    test? ( dev-util/cargo-nextest )
"

src_unpack() {
    cargo_src_unpack
    default
}

src_configure() {
    local myfeatures=(
        $(usev systemd)
        $(usev apparmor)
        $(usev selinux)
    )
    cargo_src_configure
}

src_compile() {
    cargo_src_compile --bin holo-daemon
}

src_install() {
    cargo_src_install --bin holo-daemon

    # Install configuration
    insinto /etc/holo
    newins files/holo-daemon.conf holo.conf

    # Install service files
    if use systemd; then
        systemd_newunit files/holo.service holo.service
    else
        newinitd files/holo.initd holo
    fi

    # Install AppArmor profile
    if use apparmor; then
        insinto /etc/apparmor.d
        newins files/holo.apparmor usr.bin.holo-daemon
    fi

    # Create directories
    keepdir /var/lib/holo
    keepdir /var/log/holo
    keepdir /run/holo

    fowners holo:holo /var/lib/holo /var/log/holo /run/holo
    fperms 0750 /var/lib/holo /var/log/holo /run/holo
}

pkg_postinst() {
    if use systemd; then
        systemd_reenable holo.service
    fi

    elog "Holo routing daemon has been installed."
    elog "Edit /etc/holo/holo.conf to configure routing protocols."
    elog "Start the service with: systemctl start holo"
}
```

### **Package Set Definitions**

```bash
# sets/holo-base
net-misc/holo-daemon
net-misc/holo-cli
net-misc/holo-yang

# sets/holo-full
net-misc/holo-daemon
net-misc/holo-cli
net-misc/holo-bgp
net-misc/holo-ospf
net-misc/holo-isis
net-misc/holo-bfd
net-misc/holo-vrrp
net-misc/holo-northbound
net-misc/holo-yang

# sets/holo-testing
@holo-full
app-emulation/containerlab-topologies-holo
```

### **Systemd Service Template**

```ini
# files/holo.service
[Unit]
Description=Holo Routing Daemon
Documentation=https://github.com/holo-routing/holo
After=network.target
Wants=network.target

[Service]
Type=simple
User=holo
Group=holo
ExecStart=/usr/bin/holo-daemon --config /etc/holo/holo.conf
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=10
LimitNOFILE=65536
CapabilityBoundingSet=CAP_NET_RAW CAP_NET_ADMIN
AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN

# Security settings
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/holo /var/lib/holo /run/holo
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

### **Containerlab Topology Template**

```yaml
# files/bgp-test.clab.yml
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
    host1:
      kind: linux
      image: alpine:latest
      cmd: sleep infinity
    host2:
      kind: linux
      image: alpine:latest
      cmd: sleep infinity

  links:
    - endpoints: ["router1:eth1", "router2:eth1"]
    - endpoints: ["host1:eth1", "router1:eth2"]
    - endpoints: ["host2:eth1", "router2:eth2"]
```

## 🧪 Testing Strategy

### **Unit Tests** (Target: 85%+ coverage)
- [ ] Ebuild syntax validation
- [ ] Dependency resolution testing
- [ ] Configuration parsing validation
- [ ] Service file syntax checking

### **Integration Tests**
- [ ] Package installation and removal
- [ ] Service start/stop/restart
- [ ] Multi-protocol functionality
- [ ] Configuration reload testing

### **Security Tests**
- [ ] Container security scanning (trivy)
- [ ] AppArmor profile validation
- [ ] Privilege escalation testing
- [ ] DoS resistance validation

### **Performance Tests**
- [ ] Resource usage monitoring
- [ ] Route convergence timing
- [ ] Throughput measurement
- [ ] Memory leak detection

## 🚀 Deployment Strategy

### **Overlay Registration**
```bash
# Add overlay to system
eselect repository add holo git https://github.com/regicideos/regicide-holo.git
emerge --sync

# Install base routing
emerge @holo-base

# Install full suite
emerge @holo-full

# Install testing tools
emerge @holo-testing
```

### **Service Management**
```bash
# Systemd
systemctl enable holo
systemctl start holo

# OpenRC
rc-update add holo default
/etc/init.d/holo start
```

### **Configuration Management**
```bash
# Generate initial configuration
holo-cli generate-config --protocol bgp --as 65001 > /etc/holo/holo.conf

# Validate configuration
holo-cli validate-config /etc/holo/holo.conf

# Apply configuration
holo-cli apply-config /etc/holo/holo.conf
```

## 📊 Success Metrics

### **Functional Metrics**
- [ ] All Holo components installable via emerge
- [ ] Service management working (systemd + OpenRC)
- [ ] Multi-protocol interoperability validated
- [ ] Containerlab topologies functional

### **Performance Metrics**
- [ ] Package installation time <5 minutes
- [ ] Service startup time <10 seconds
- [ ] Route convergence within protocol specifications
- [ ] Resource usage within targets

### **Security Metrics**
- [ ] Container security scan passes (no critical CVEs)
- [ ] AppArmor profile functional
- [ ] Non-root execution validated
- [ ] Privilege separation working

### **Integration Metrics**
- [ ] RegicideOS AI agent coordination working
- [ ] System monitoring integration functional
- [ ] Configuration management automation ready
- [ ] Documentation complete and tested

## 🔐 Security Considerations

### **Access Control**
- [ ] Non-root user execution
- [ ] Capability-based privilege management
- [ ] AppArmor/SELinux profile support
- [ ] Secure inter-process communication

### **Network Security**
- [ ] Protocol authentication support
- [ ] GTSM implementation
- [ ] DoS protection mechanisms
- [ ] Input validation and sanitization

### **Container Security**
- [ ] Minimal base images
- [ ] Non-root container execution
- [ ] Security scanning integration
- [ ] SBOM generation

## 🎯 Integration Points

### **With RegicideOS AI Agents**
- [ ] BtrMind: Storage optimization for routing data
- [ ] PortCL: Package management optimization
- [ ] Holo: Network optimization and automation

### **With System Components**
- [ ] Systemd/OpenRC service management
- [ ] AppArmor/SELinux security integration
- [ ] System monitoring and logging
- [ ] Configuration management frameworks

### **With External Tools**
- [ ] Containerlab for network testing
- [ ] Prometheus for metrics collection
- [ ] OpenTelemetry for observability
- [ ] gNMI for network automation

## 📋 Risk Mitigation

### **Implementation Risks**
- **Ebuild Complexity**: Start with simple ebuilds, incrementally add features
- **Dependency Management**: Use virtual packages for complex dependencies
- **Service Integration**: Test both systemd and OpenRC from the beginning

### **Operational Risks**
- **Network Stability**: Implement conservative defaults and extensive testing
- **Security Vulnerabilities**: Regular security scanning and updates
- **Performance Issues**: Continuous monitoring and optimization

## 🔄 Progress Tracking

### **Week 1: Foundation**
- [ ] Overlay structure complete
- [ ] Core ebuilds created
- [ ] Basic compilation working
- [ ] Documentation started

### **Week 2: Integration**
- [ ] Service files working
- [ ] Configuration management ready
- [ ] Package sets functional
- [ ] Basic installation tested

### **Week 3: Testing**
- [ ] Containerlab integration working
- [ ] Security validation passed
- [ ] Performance benchmarks met
- [ ] Integration tests passing

### **Week 4: Polish**
- [ ] Documentation complete
- [ ] AI agent integration working
- [ ] Final validation passed
- [ ] Ready for release

---

**Next Steps**:
1. Create overlay directory structure
2. Begin Week 1 implementation with core ebuilds
3. Set up development and testing environment
4. Coordinate with Holo routing team for support

**Dependencies**:
- Holo routing project stability and API compatibility
- Gentoo ebuild review process
- RegicideOS integration testing environment

**Success Criteria**:
- Working overlay with all Holo components
- Service integration with both systemd and OpenRC
- Security validation passed
- Documentation complete and user-tested
- Integration with RegicideOS ecosystem functional