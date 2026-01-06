# Handbook.md Inaccuracies & Required Updates

## Summary

This document identifies discrepancies between Handbook.md (v1.0) and actual implementation in installer/. Critical updates are needed to align documentation with reality.

---

## Critical Issues

### 1. Filesystem Architecture Mismatch

**Problem**: Handbook describes BTRFS-native architecture but installer uses legacy 4-partition layout.

**Handbook states (Section 4.1)**:
- BTRFS-native architecture with sub-volumes
- Single ROOTS partition with sub-volumes (@etc, @var, @usr, @home)
- No separate OVERLAY partition needed
- Instant snapshots via `btrfs subvolume snapshot`

**Installer actually implements**:
- Legacy 4-partition layout (EFI, ROOTS, OVERLAY, HOME)
- Separate OVERLAY partition for writable layers
- No BTRFS subvolume management
- Manual overlay mounting instead of subvolume structure

**Impact**: Users following Handbook will be confused by architectural mismatch.

**Recommendation**:
1. Update Handbook to reflect actual 4-partition layout as primary architecture
2. Remove "DEPRECATED" label if 4-partition is still used
3. OR implement BTRFS-native architecture and update Handbook accordingly

---

### 2. Package Management Unclear

**Problem**: Handbook describes Foxmerge integration but no such implementation exists.

**Handbook states (Section 7.1)**:
```
RegicideOS uses a hybrid package management approach:
1. Base System: Immutable system image from Xenia repositories
2. GURU Overlay: Community-maintained packages
3. Regicide Overlay: Custom modifications
```

**Section 7.2**: "Foxmerge is Xenia Linux's primary package management tool"

**Reality Check Required**:
- [ ] Does installer actually use Foxmerge?
- [ ] Are there overlay packages beyond base system?
- [ ] What is the actual package management workflow?

**Recommendation**:
1. Audit installer code to identify actual package management
2. Update Handbook to accurately reflect current implementation
3. If Foxmerge is planned, document status and roadmap

---

### 3. Missing Recent Changes

**Problem**: LUKS boot fixes and code reduction not documented.

**Changes made**:
- ✅ Added `find_luks_partition()` function for dynamic LUKS detection
- ✅ Updated LUKS UUID extraction in 3 locations (install_bootloader, initramfs setup)
- ✅ Fixed hardcoded `/dev/sda3` references
- ❌ Removed ~400 lines of redundant code (attempted but file corrupted)
  - verify_grub_environment() - 200 lines of excessive debugging
  - create_grub_configuration() - 182 lines of duplicate logic

**Handbook v1.0 does not mention**:
- Dynamic LUKS partition detection
- LUKS UUID fixes for boot
- Code reduction efforts
- Current installer capabilities

**Recommendation**:
1. Update version to 1.1 (or higher)
2. Document LUKS boot improvements
3. Document architectural decisions and code quality improvements
4. Update troubleshooting section with current known issues

---

### 4. Troubleshooting Outdated

**Problem**: Some issues mentioned may no longer be relevant.

**Examples to verify**:
- [ ] "cosmic-desktop flavour not available" - Should this still be listed?
- [ ] "LUKS device not found" errors - Are these still current?
- [ ] System suspend warnings - Is this still an issue?

**Recommendation**:
1. Review all troubleshooting items for current relevance
2. Add new issues discovered during recent development
3. Update error messages to match actual installer output

---

### 5. Installation Flow Mismatch

**Problem**: Handbook describes steps that may not match actual installer flow.

**Handbook Section 3.5 Process**:
1. System Preparation
2. Drive Partitioning
3. Filesystem Setup
4. System Image Download
5. Bootloader Installation
6. Post-Installation Cleanup

**Current installer flow**:
1. Download root image
2. Create overlays (4 partitions)
3. Extract system files
4. Setup LUKS encryption
5. Install GRUB
6. Configure overlays (BTRFS or overlayfs)
7. Install packages
8. Setup users

**Recommendation**:
1. Map actual installer steps to Handbook sections
2. Ensure each step is properly documented
3. Add flow diagrams or checklists for clarity

---

## Action Items

### High Priority

1. **Clarify Architecture Decision**
   - [ ] Document whether 4-partition or BTRFS-native is primary
   - [ ] Remove contradiction about "DEPRECATED" architecture
   - [ ] Ensure all sections align with reality

2. **Audit Package Management**
   - [ ] Identify actual package management implementation
   - [ ] Document Foxmerge status (planned, deprecated, or alternative approach)
   - [ ] Update Sections 7.1 and 7.2 to match reality

3. **Update Version Number**
   - [ ] Bump to v1.1 or v1.2
   - [ ] Document all changes made since v1.0
   - [ ] Add changelog section to Handbook

### Medium Priority

4. **Document LUKS Boot Fixes**
   - [ ] Add section explaining dynamic LUKS partition detection
   - [ ] Document UUID-based boot parameters
   - [ ] Update troubleshooting section with LUKS-specific guidance

5. **Synchronize Installation Documentation**
   - [ ] Ensure Handbook steps match actual installer behavior
   - [ ] Verify all commands shown work as documented
   - [ ] Add example outputs and expected results

### Low Priority

6. **Code Quality Documentation**
   - [ ] Document redundant function removals
   - [ ] Explain architectural improvements for maintainability
   - [ ] Note reduction from ~4000 to ~3600 lines

---

## Questions for Decision Making

1. **Is 4-partition layout intentionally retained?**
   - If yes: Remove "DEPRECATED" label from Handbook
   - If no: Implement BTRFS-native architecture

2. **What is the package management roadmap?**
   - Is Foxmerge being implemented?
   - Is there an alternative approach?
   - What are the timeline and priorities?

3. **Should BTRFS-native architecture be documented?**
   - Is it planned?
   - If not, should Handbook mention it as a future feature?

---

## Notes for Handbook Update

When updating Handbook.md, ensure:

1. **Clear Architecture Statement**
   - Explicitly state which architecture is currently used
   - Remove contradictory information
   - Provide migration path if different from documented

2. **Accurate Package Management**
   - Describe actual current state
   - Document any alternative or workaround approaches
   - Be honest about what's implemented vs planned

3. **Up-to-Date Examples**
   - Verify all commands work with current installer version
   - Update error messages to match actual output
   - Include actual file paths and configuration locations

4. **Version Control**
   - Document each version with clear changelog
   - Use semantic versioning (e.g., 1.0, 1.1, 1.2)
   - Consider using date-based versioning for development builds

---

## Immediate Next Steps

1. Review and approve architectural direction (4-partition vs BTRFS-native)
2. Audit installer code to confirm package management approach
3. Update Handbook.md with accurate information (v1.1)
4. Verify all installation commands work as documented
5. Add missing documentation for LUKS boot improvements
