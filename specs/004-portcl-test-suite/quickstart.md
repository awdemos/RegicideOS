# PortCL Test Suite Quick Start Guide

This guide provides step-by-step instructions for setting up, running, and working with the PortCL comprehensive test suite.

## Prerequisites

Before starting, ensure you have:

- **Rust 1.75+** installed with cargo
- **PortCL codebase** available (this should be in your current directory)
- **Basic familiarity** with Rust testing concepts
- **Linux environment** (RegicideOS target platform)

## Quick Setup

### 1. Clone and Navigate

```bash
# Ensure you're in the PortCL directory
cd /path/to/RegicideOS/ai-agents/portcl

# Verify you're on the correct branch
git branch
# Should show: 004-portcl-test-suite (or main with merged changes)
```

### 2. Install Dependencies

```bash
# Install Rust dependencies
cargo build

# Install test-specific dependencies (if not already in Cargo.toml)
cargo install cargo-tarpaulin  # For coverage reports
```

### 3. Verify Test Infrastructure

```bash
# Check that test structure exists
ls -la tests/
# Should show: fixtures/  integration/  performance/  unit/  README.md

# Verify you can run basic tests
cargo test --lib --quiet
```

## Running Tests

### Basic Test Execution

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test unit::

# Run only integration tests
cargo test integration::

# Run tests with verbose output
cargo test -- --nocapture

# Run tests with specific output format
cargo test -- --format pretty
```

### Running Specific Test Modules

```bash
# Run error handling tests
cargo test error

# Run serialization tests
cargo test serde

# Run utility tests
cargo test utils

# Run performance benchmarks
cargo test bench
```

### Running Tests with Features

```bash
# Run tests without ML dependencies
cargo test --no-default-features

# Run tests with ML features enabled
cargo test --features ml

# Run tests with test utilities
cargo test --features test-utils
```

## Understanding Test Results

### Test Output Format

```
running 15 tests
test tests::unit::error_tests::test_error_creation ... ok
test tests::unit::error_tests::test_error_display ... ok
test tests::unit::serde_tests::test_json_round_trip ... ok
test tests::unit::utils_tests::test_format_duration ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Coverage Reports

```bash
# Generate coverage report
cargo tarpaulin

# Generate HTML coverage report
cargo tarpaulin --output-html

# Generate coverage for specific modules
cargo tarpaulin --lib --files src/error.rs src/utils/
```

### Performance Benchmarks

```bash
# Run performance benchmarks
cargo bench

# Run specific benchmark
cargo bench serialization_performance

# Compare benchmark results
cargo bench --save baseline
cargo bench --baseline baseline
```

## Test Organization

### Directory Structure

```
tests/
├── fixtures/           # Test data and utilities
│   ├── mod.rs
│   ├── mock_data.rs    # Mock PortCL data structures
│   └── test_helpers.rs # Test utility functions
├── unit/              # Unit tests by component
│   ├── mod.rs
│   ├── error_tests.rs      # Error handling tests
│   ├── serde_tests.rs      # Serialization tests
│   └── utils_tests.rs      # Core utility tests
├── integration/       # Integration tests (placeholder)
├── performance/       # Performance tests (placeholder)
├── basic_tests.rs     # Basic working components
└── README.md          # This test documentation
```

### Test Naming Conventions

- **Unit Tests**: `test_<functionality>_<scenario>()`
- **Integration Tests**: `test_<component>_integration_<scenario>()`
- **Error Tests**: `test_error_<type>_<scenario>()`
- **Performance Tests**: `bench_<operation>()`

## Working with Test Data

### Using Mock Data

```rust
// Import mock data
use portcl::tests::fixtures::mock_data::*;

// Use predefined mock packages
let packages = sample_packages();
let actions = sample_actions();

// Create custom mock data
let custom_package = MockPackage {
    name: "test-package".to_string(),
    category: "test-category".to_string(),
    // ... other fields
};
```

### Test Helpers

```rust
// Import test helpers
use portcl::tests::fixtures::test_helpers::*;

// Create temporary directory
let temp_dir = create_temp_dir();

// Create temporary file with content
let test_file = create_temp_file_with_content("test content");

// Validate package names
assert!(validate_package_name("sys-apps/portage"));
```

## Writing New Tests

### Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use portcl::error::{PortCLError, Result};
    use portcl::tests::fixtures::test_helpers::*;

    #[test]
    fn test_functionality_success_case() {
        // Arrange
        let input = "test_input";

        // Act
        let result = your_function(input);

        // Assert
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_functionality_error_case() {
        // Arrange
        let invalid_input = "invalid_input";

        // Act
        let result = your_function(invalid_input);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PortCLError::Validation(_)));
    }

    #[tokio::test]
    async fn test_async_functionality() {
        // Arrange
        let temp_file = create_temp_file_with_content("async test content");

        // Act
        let result = your_async_function(&temp_file).await;

        // Assert
        assert!(result.is_ok());
    }
}
```

### Integration Test Template

```rust
#[tokio::test]
async fn test_component_integration() {
    // Arrange
    let config = MockConfig::default();
    let test_env = TestEnvironment::new(config).await?;

    // Act
    let result = test_env.execute_workflow().await;

    // Assert
    assert!(result.is_ok());
    // Verify integration worked correctly
}
```

## Common Test Scenarios

### Error Handling Tests

```rust
#[test]
fn test_error_propagation() {
    // Test that errors are properly handled and propagated
    let result = operation_that_might_fail();

    match result {
        Ok(_) => panic!("Expected error, got success"),
        Err(PortCLError::Io(_)) => {}, // Expected error type
        Err(e) => panic!("Unexpected error type: {}", e),
    }
}

#[test]
fn test_retryable_error_detection() {
    let error = PortCLError::Network(/* network error */);
    assert!(is_retryable_error(&error));

    let error = PortCLError::Validation("invalid input".to_string());
    assert!(!is_retryable_error(&error));
}
```

### Serialization Tests

```rust
#[test]
fn test_json_round_trip() {
    let original = TestStruct::default();

    // Serialize to JSON
    let json_str = to_json_string(&original).unwrap();

    // Deserialize back
    let deserialized: TestStruct = from_json_string(&json_str).unwrap();

    // Verify round-trip
    assert_eq!(original, deserialized);
}

#[test]
fn test_invalid_json_handling() {
    let invalid_json = "{ invalid json }";
    let result: Result<TestStruct> = from_json_string(invalid_json);

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PortCLError::Json(_)));
}
```

## Troubleshooting

### Common Issues

**Compilation Errors**
```bash
# Ensure all dependencies are available
cargo build

# Check for missing features
cargo check --features ml
```

**Test Failures**
```bash
# Run tests with backtrace
RUST_BACKTRACE=1 cargo test

# Run specific failing test
cargo test test_name

# Run tests with detailed output
cargo test -- --nocapture --test-threads=1
```

**Performance Issues**
```bash
# Run tests with timing
cargo test -- --format=pretty

# Check for slow tests
cargo test -- --quiet
```

### Debug Mode

```bash
# Run tests with debug logging
RUST_LOG=debug cargo test

# Run tests with trace logging
RUST_LOG=trace cargo test

# Save test output to file
cargo test > test_output.log 2>&1
```

## Best Practices

### 1. Test Organization
- Group related tests together
- Use descriptive test names
- Keep tests focused and single-purpose

### 2. Test Data Management
- Use fixtures for reusable test data
- Clean up temporary files after tests
- Avoid hardcoded values in tests

### 3. Error Testing
- Test both success and error cases
- Verify error types and messages
- Test error recovery and retry logic

### 4. Performance Considerations
- Keep individual tests fast (< 1 second)
- Use appropriate test timeouts
- Avoid unnecessary I/O in tests

## Next Steps

### For Developers
1. **Run the test suite** to verify everything works
2. **Explore existing tests** to understand patterns
3. **Add new tests** for any uncovered functionality
4. **Check coverage** and improve where needed

### For Maintainers
1. **Set up CI/CD** with automated test execution
2. **Monitor coverage** metrics
3. **Review test quality** regularly
4. **Update documentation** as the suite evolves

## Getting Help

### Resources
- **Test Documentation**: `tests/README.md`
- **Coverage Report**: `test-coverage-report.md`
- **Rust Testing Book**: https://doc.rust-lang.org/book/ch11-00-testing.html
- **Cargo Test Documentation**: https://doc.rust-lang.org/cargo/commands/cargo-test.html

### Commands Cheat Sheet

```bash
# Quick test run
cargo test

# Specific module
cargo test error

# Coverage report
cargo tarpaulin

# Performance benchmarks
cargo bench

# Debug mode
RUST_LOG=debug cargo test -- --nocapture
```

---

This quick start guide should get you up and running with the PortCL test suite quickly. For more detailed information, refer to the full documentation in `tests/README.md` and the coverage analysis in `test-coverage-report.md`.