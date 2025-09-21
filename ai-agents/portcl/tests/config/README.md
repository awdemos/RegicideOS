# PortCL Test Configuration Documentation

## Overview

This directory contains comprehensive test configuration files and templates for the PortCL test suite. The configuration system supports multiple environments, test types, and common testing patterns.

## Directory Structure

```
tests/config/
├── README.md                    # This documentation
├── environments/               # Environment-specific configurations
│   ├── development.toml        # Development environment
│   ├── staging.toml           # Staging environment
│   └── production.toml        # Production environment
├── test_types/                # Test type specific configurations
│   ├── unit.toml             # Unit test configuration
│   ├── integration.toml      # Integration test configuration
│   ├── performance.toml      # Performance test configuration
│   └── property.toml        # Property-based test configuration
└── templates/                # Common test pattern templates
    ├── quick_test.toml       # Quick development test
    ├── comprehensive_test.toml # Comprehensive test suite
    ├── regression_test.toml  # Regression testing
    ├── ci_cd.toml           # CI/CD pipeline testing
    └── load_test.toml       # Load and stress testing
```

## Configuration Hierarchy

Configurations are applied in the following order (later configs override earlier ones):

1. Base configuration (from test type)
2. Environment configuration
3. Template configuration
4. Command line overrides

## Usage

### Environment Selection

Select the appropriate environment for your testing needs:

```bash
# Use development configuration (default)
cargo test --config tests/config/environments/development.toml

# Use staging configuration
cargo test --config tests/config/environments/staging.toml

# Use production configuration
cargo test --config tests/config/environments/production.toml
```

### Test Type Selection

Choose the appropriate test type configuration:

```bash
# Run unit tests
cargo test --config tests/config/test_types/unit.toml

# Run integration tests
cargo test --config tests/config/test_types/integration.toml

# Run performance tests
cargo test --config tests/config/test_types/performance.toml

# Run property-based tests
cargo test --config tests/config/test_types/property.toml
```

### Template Usage

Use templates for common testing scenarios:

```bash
# Quick development test
cargo test --config tests/config/templates/quick_test.toml

# Comprehensive test suite
cargo test --config tests/config/templates/comprehensive_test.toml

# Regression testing
cargo test --config tests/config/templates/regression_test.toml

# CI/CD pipeline
cargo test --config tests/config/templates/ci_cd.toml

# Load testing
cargo test --config tests/config/templates/load_test.toml
```

### Combined Configuration

Combine configurations for specific needs:

```bash
# Run unit tests in staging environment
cargo test --config tests/config/test_types/unit.toml \
           --config tests/config/environments/staging.toml

# Run performance tests with CI/CD template
cargo test --config tests/config/test_types/performance.toml \
           --config tests/config/templates/ci_cd.toml
```

## Configuration Sections

### `[execution]`

Controls test execution parameters:

- `timeout_seconds`: Maximum time for test execution
- `max_parallel_tests`: Number of parallel test threads
- `retry_count`: Number of retries on failure
- `fail_fast`: Stop on first failure
- `randomize_order`: Randomize test execution order

### `[logging]`

Controls logging and output:

- `level`: Log level (debug, info, warn, error)
- `format`: Log format (pretty, compact, json)
- `target`: Log output target (console, file, syslog)
- `show_skipped`: Show skipped tests in output
- `show_output`: Show test output

### `[coverage]`

Controls code coverage settings:

- `enabled`: Enable coverage collection
- `report_formats`: Coverage report formats
- `exclude_paths`: Paths to exclude from coverage
- `threshold_percentage`: Minimum coverage threshold

### `[performance]`

Controls performance testing:

- `enabled`: Enable performance measurements
- `sample_size`: Number of samples for benchmarks
- `warmup_runs`: Number of warmup runs
- `max_execution_time_ms`: Maximum execution time
- `memory_limit_mb`: Memory usage limit

### `[output]`

Controls test output and results:

- `directory`: Output directory for results
- `formats`: Output formats (json, junit, html)
- `preserve_artifacts`: Keep test artifacts
- `generate_report`: Generate HTML report

### `[monitoring]`

Controls test monitoring and metrics:

- `enabled`: Enable monitoring
- `metrics_enabled`: Collect performance metrics
- `tracing_enabled`: Enable distributed tracing
- `profiling_enabled`: Enable profiling

### `[security]`

Controls security settings:

- `api_key_required`: Require API key for tests
- `rate_limiting_enabled`: Enable rate limiting
- `timeout_tolerance_ms`: Timeout tolerance

## Environment Variables

Several configuration values can be overridden with environment variables:

- `TEST_ENVIRONMENT`: Override environment name
- `TEST_CONFIG_PATH`: Path to configuration file
- `TEST_OUTPUT_DIR`: Override output directory
- `TEST_LOG_LEVEL`: Override log level
- `TEST_API_KEY`: API key for tests
- `TEST_TIMEOUT_SECONDS`: Override timeout
- `TEST_PARALLEL_TESTS`: Override parallel test count

## Custom Configuration

### Creating Custom Configurations

1. Copy an existing configuration as a template
2. Modify the settings for your specific needs
3. Use it with the `--config` flag

### Environment-Specific Overrides

Create a `config/local.toml` file for local overrides:

```toml
[execution]
max_parallel_tests = 2

[logging]
level = "debug"
```

This file will be automatically loaded if it exists and will override other configurations.

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      - name: Run tests
        run: cargo test --config tests/config/templates/ci_cd.toml
        env:
          TEST_API_KEY: ${{ secrets.TEST_API_KEY }}
          TEST_WEBHOOK_URL: ${{ secrets.TEST_WEBHOOK_URL }}
```

### Jenkins Pipeline Example

```groovy
pipeline {
    agent any
    stages {
        stage('Test') {
            steps {
                sh 'cargo test --config tests/config/templates/ci_cd.toml'
            }
            environment {
                TEST_API_KEY = credentials('test-api-key')
            }
        }
    }
}
```

## Best Practices

1. **Use appropriate environments**: development for local work, staging for CI/CD, production for final testing
2. **Choose the right test type**: unit for fast feedback, integration for component testing, performance for optimization
3. **Leverage templates**: Use predefined templates for common scenarios
4. **Monitor thresholds**: Set realistic performance and coverage thresholds
5. **Secure credentials**: Use environment variables for sensitive configuration values
6. **Clean up artifacts**: Configure automatic cleanup to save disk space
7. **Parallelize wisely**: Adjust parallel test count based on system resources

## Troubleshooting

### Common Issues

1. **Configuration not found**: Check file paths and ensure config files exist
2. **Permission denied**: Check file permissions and output directory access
3. **Timeout errors**: Increase timeout values or optimize test performance
4. **Memory issues**: Reduce parallel test count or increase memory limits
5. **Coverage failures**: Adjust coverage thresholds or exclude non-test paths

### Debug Configuration

Enable debug logging to troubleshoot configuration issues:

```bash
cargo test --config tests/config/environments/development.toml -- --nocapture
```

### Validate Configuration

Validate configuration syntax:

```bash
cargo run --bin config-validator --config tests/config/environments/development.toml
```

## Support

For issues or questions about the test configuration system:

1. Check this documentation
2. Review existing configuration files for examples
3. Create an issue in the PortCL repository
4. Contact the development team

## License

This configuration system is part of the PortCL project and follows the same license terms.