# RegicideOS Development Roadmap

> ⚠️ **ASPIRATIONAL DOCUMENT**: For the actual project state, see [STATUS.md](STATUS.md).

## Current Status Assessment

### ✅ Completed
- [x] Rust-based installer with EFI reliability improvements (basic, needs refactor)
- [x] Technical specifications (PortCL, BtrMind, Rust overlay)
- [x] Comprehensive handbook documentation (partially aspirational)
- [x] BTRFS read-only system architecture design (conceptual)

### 🚧 In Progress / Next Steps

## Phase 1: Foundation Implementation

### Sprint 1: BtrMind AI Agent (2-3 weeks) 🟡 MOSTLY COMPLETE
**Priority**: CRITICAL - Core differentiator
**Goal**: Working BTRFS monitoring with basic AI

**Tasks**:
1. **Create `btrmind` Rust project** ✅
   ```bash
   mkdir -p ai-agents/btrmind
   cargo init ai-agents/btrmind
   ```

2. **Implement core components** ✅:
   - BTRFS metrics collection (`btrfs filesystem usage`, `df` fallback)
   - Reinforcement learning agent with action success rate tracking
   - Reward function implementation (disk usage improvement based)
   - systemd service integration

3. **Dependencies** ✅:
   ```toml
   [dependencies]
   tokio = { version = "1.0", features = ["full"] }
   serde = { version = "1.0", features = ["derive"] }
   anyhow = "1.0"
   tracing = "0.1"
   rand = "0.8"
   chrono = "0.4"
   clap = "4.0"
   ```

4. **Success criteria** 🟡:
   - ✅ Collects BTRFS metrics every 60 seconds (configurable)
   - ✅ Responds to >90% disk usage with cleanup actions
   - ✅ Learns from cleanup effectiveness using action success rates
   - ✅ Runs as systemd service with proper configuration (service file exists)
   - ✅ 17/17 unit tests passing
   - ✅ Cross-platform compilation (tested on macOS, targets Linux)
   - ✅ Complete CLI with analyze, cleanup, stats, config commands
   - ✅ Comprehensive documentation and installation scripts
   - ⚠️ Integration testing on real installed system pending (no bootable ISO yet)

### Sprint 2: Regicide-Rust Overlay (1-2 weeks) 🟡 PARTIAL
**Priority**: HIGH - Essential for package management
**Goal**: Working overlay with basic Rust packages

**Tasks**:
1. **Create overlay repository**:
   - Set up `regicide-rust-overlay` GitHub repo ✅
   - Implement overlay structure per spec ✅ (basic)
   - Create initial ebuilds for Rust with embedded targets 🟡

2. **Integration**:
   - Configure overlay priority system 📋
   - Test with GURU overlay 📋
   - Document installation process ✅

### Sprint 3: Cosmic Desktop Integration (2-3 weeks) 📋 NOT STARTED
**Priority**: HIGH - Main user differentiator
**Goal**: RegicideOS boots to Cosmic Desktop

**Tasks**:
1. **Cosmic Desktop customization**:
   - Fork/configure Cosmic for RegicideOS (not started)
   - Create custom theming (not started)
   - Integrate AI agent status widgets (not started)

2. **Installer integration**:
   - Update installer to configure Cosmic (not started)
   - Remove other desktop environment options (not started)
   - Add post-install Cosmic setup (not started)

### Sprint 4: PortCL AI Agent (3-4 weeks) 🟡 NOT COMPLETED
**Priority**: HIGH - Package optimization
**Goal**: Working package management optimization

**Tasks**:
1. **PortCL implementation** ⚠️ PARTIAL:
   - CLI framework exists
   - RL engine stubbed (not implemented)
   - Portage monitoring integration (not implemented)
   - Build parallelism optimization (not implemented)
   - Test suite exists but is mostly stubs (not 90%+ coverage)
   - See [STATUS.md](STATUS.md) for honest assessment

## Phase 2: System Integration

### Sprint 5: Base System Image (3-4 weeks)
**Priority**: HIGH - Independence from Xenia
**Goal**: Custom RegicideOS system image

**Tasks**:
1. **Build system**:
   - Fork Xenia build infrastructure
   - Integrate AI agents into base image
   - Configure Cosmic Desktop defaults
   - Automated image generation

2. **Testing infrastructure**:
   - VM testing environment
   - Automated installation testing
   - AI agent validation tests

### Sprint 6: Core Rust Utilities (4-6 weeks)  
**Priority**: MEDIUM - Rust-first architecture
**Goal**: Replace core utilities with Rust implementations

**Candidates for replacement**:
- `ls`, `cat`, `grep` → `exa`, `bat`, `ripgrep`
- `find` → `fd`
- `ps`, `top` → `procs`, `bottom`
- `du` → `dust`

## Phase 3: Advanced Features

### Sprint 7: Multi-Agent Coordination (4-6 weeks)
**Goal**: AI agents share knowledge and coordinate actions

### Sprint 8: Advanced Learning (6-8 weeks)
**Goal**: Continual learning, transfer learning, natural language interface

## Development Setup Instructions

### 1. Set up development environment:
```bash
cd RegicideOS
mkdir -p ai-agents/{btrmind,portcl}
mkdir -p overlays/regicide-rust
mkdir -p cosmic-integration
```

### 2. Start with BtrMind:
```bash
cd ai-agents/btrmind
cargo init
# Copy btrmind.md spec as reference
# Implement based on technical specification
```

### 3. Testing approach:
```bash
# Create test BTRFS filesystem
sudo truncate -s 1G test.img
sudo losetup /dev/loop0 test.img
sudo mkfs.btrfs /dev/loop0
sudo mkdir /mnt/test-btrfs
sudo mount /dev/loop0 /mnt/test-btrfs

# Test btrmind against this filesystem
```

## Success Metrics

### Sprint 1 Success (BtrMind):
- [x] Unit tests pass: 17/17
- [x] Collects metrics: `btrmind analyze` works
- [x] Responds to high usage: Cleanup actions triggered in tests
- [ ] Service starts without errors on real system: `systemctl status btrmind` (pending bootable ISO)
- [ ] Learning improves performance: Multiple cleanup cycles on real filesystem (pending integration test)

### Phase 1 Success (Foundation):
- [ ] RegicideOS boots to a desktop environment (no ISO exists yet)
- [ ] BtrMind runs automatically as systemd service (pending installed system)
- [ ] PortCL runs automatically (pending real implementation)
- [ ] Package management works via overlays (partial — dotfiles only)
- [x] Documentation exists (extensive but partially aspirational)

## Resource Allocation

**Time estimate**: 12-16 weeks for Phase 1
**Team size**: 1-3 developers
**Skills needed**: Rust, AI/ML, Linux system administration, BTRFS

## Risk Mitigation

**Risk**: AI complexity slows development
**Mitigation**: Start with simple rule-based agents, add ML gradually

**Risk**: Cosmic Desktop integration issues  
**Mitigation**: Have fallback plan with minimal window manager

**Risk**: Package management complexity
**Mitigation**: Start with subset of packages, expand gradually

---

**Next Action**: Build bootable ISO / base system image
**Timeline**: Start immediately, 4-6 week sprint
**Success Criterion**: Working Gentoo stage4 that boots to a desktop environment
