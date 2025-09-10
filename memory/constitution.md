# RegicideOS Installer Constitution

## Core Principles

### I. UEFI-First (NON-NEGOTIABLE)
All installations require UEFI firmware; BIOS systems are explicitly unsupported due to btrfs requirements; UEFI detection must occur before any destructive operations

### II. Btrfs-Only Architecture
All installations must use btrfs or btrfs with encryption; Traditional filesystem layouts are not permitted; btrfs subvolumes must be properly configured for system organization

### III. Safety-First Operations
Destructive operations require explicit user confirmation; All validations must pass before installation begins; System must validate hardware requirements before proceeding

### IV. Test-First Development
TDD mandatory for all new features; Integration tests required for destructive operations; All failure scenarios must have test coverage

### V. Automation Support
Interactive and automated installation modes must be equally supported; Configuration validation must work consistently across both modes; TOML configuration must be complete and validatable

## System Requirements

### Hardware Constraints
- Minimum drive size: 12GB
- UEFI firmware required (no BIOS support)
- Network connectivity for root image download
- Sufficient RAM for installation process

### Filesystem Standards
- EFI System Partition: 512MB vfat
- Root Partition: btrfs with standard subvolumes
- Encryption: LUKS with btrfs (optional)
- All partitions must be properly labeled

### Operational Safety
- Warn before destructive drive operations
- Validate all configuration parameters
- Handle installation failures gracefully
- Provide clear error messages and recovery options

## Development Workflow

### Quality Gates
- All specifications must be approved before implementation
- Test coverage must exceed 90% for core functionality
- Integration tests must pass for all installation scenarios
- Documentation must be updated with each feature

### Change Management
- Breaking changes require version increment
- All changes must preserve existing functionality
- Backward compatibility must be maintained where possible
- Security updates must be prioritized and tested immediately

## Governance

### Constitution Supremacy
This constitution supersedes all other practices; All development decisions must align with these principles; Amendments require full team approval and migration planning

### Compliance Verification
- All pull requests must verify constitutional compliance
- Code reviews must check for principle violations
- Automated tests must validate core requirements
- Complexity must be justified and documented

**Version**: 1.0.0 | **Ratified**: 2025-09-10 | **Last Amended**: 2025-09-10