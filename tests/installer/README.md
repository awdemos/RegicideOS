# Installer Testing Infrastructure

This directory contains comprehensive tests for the RegicideOS installer to ensure safety and reliability.

## Test Structure

### Unit Tests (`unit/`)
- **Configuration validation**: Test config parsing and validation logic
- **UEFI detection**: Test firmware detection mechanisms
- **Drive operations**: Test disk size calculation and validation
- **Filesystem operations**: Test btrfs layout validation

### Integration Tests (`integration/`)
- **Full workflow**: Test complete installation process with mocked operations
- **Error handling**: Test graceful failure on various error conditions
- **Configuration modes**: Test both interactive and automated installation

### Safety Tests (`safety/`)
- **Destructive operations**: Ensure safety checks prevent data loss
- **Validation gates**: Test all pre-conditions before dangerous operations
- **Recovery mechanisms**: Test error recovery and rollback procedures

## Running Tests

```bash
# Run all installer tests
python -m pytest tests/installer/

# Run specific test categories
python -m pytest tests/installer/unit/
python -m pytest tests/installer/safety/

# Run with coverage
python -m pytest --cov=installer tests/installer/
```

## Safety Requirements

All tests must pass before the installer can be used in production. Safety tests have the highest priority.

## Mock Operations

All destructive operations are mocked using:
- `unittest.mock` for Python function calls
- `subprocess` mocking for system commands
- `tempfile` for temporary file operations
- Fake disk images for partitioning tests