//! Test helpers and utilities for PortCL testing

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// Creates a temporary directory for tests
pub fn create_temp_dir() -> TempDir {
    tempdir().expect("Failed to create temporary directory")
}

/// Creates a temporary file with content
pub fn create_temp_file_with_content(content: &str) -> PathBuf {
    let temp_dir = create_temp_dir();
    let file_path = temp_dir.path().join("test_file.txt");
    fs::write(&file_path, content).expect("Failed to write temporary file");
    file_path
}

/// Creates a mock JSON configuration file
pub fn create_mock_config_file() -> PathBuf {
    let config_content = r#"
{
    "api_key": "test_api_key",
    "base_url": "https://api.test.com",
    "timeout_seconds": 30,
    "max_retries": 3,
    "log_level": "info",
    "enable_ml": false
}
"#;
    create_temp_file_with_content(config_content)
}

/// Creates a mock TOML configuration file
pub fn create_mock_toml_file() -> PathBuf {
    let toml_content = r#"
[api]
key = "test_api_key"
base_url = "https://api.test.com"

[performance]
timeout_seconds = 30
max_retries = 3

[logging]
level = "info"
enable_ml = false
"#;
    create_temp_file_with_content(toml_content)
}

/// Returns test data directory path
pub fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data")
}

/// Ensures test data directory exists
pub fn ensure_test_data_dir() -> PathBuf {
    let data_dir = test_data_dir();
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).expect("Failed to create test data directory");
    }
    data_dir
}

/// Helper for testing async functions
pub async fn async_test_wrapper<F, R>(test_fn: F) -> R
where
    F: std::future::Future<Output = R>,
{
    test_fn.await
}

/// Mock implementation for testing error conditions
pub fn mock_io_error() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, "Mock IO error")
}

/// Mock implementation for testing JSON errors
pub fn mock_json_error() -> serde_json::Error {
    serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err()
}

/// Mock implementation for testing TOML errors
pub fn mock_toml_error() -> toml::de::Error {
    toml::from_str::<toml::Value>("invalid toml").unwrap_err()
}

/// Creates a mock HTTP response for testing
pub fn mock_http_response() -> reqwest::Response {
    reqwest::Response::from(
        http::Response::builder()
            .status(200)
            .body("mock response".to_string())
            .unwrap(),
    )
}

/// Helper to measure execution time
pub fn measure_execution_time<F, R>(f: F) -> (R, std::time::Duration)
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Assertion helper for comparing floating point values with tolerance
pub fn assert_almost_equal(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "Expected {} ± {}, got {} (diff: {})",
        expected, tolerance, actual, diff
    );
}

/// Helper to create mock UUID for testing
pub fn mock_uuid() -> uuid::Uuid {
    uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
}

/// Helper to create mock timestamp for testing
pub fn mock_timestamp() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}

/// Helper to validate file permissions
pub fn validate_file_permissions(path: &Path, expected_mode: u32) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            return permissions.mode() & 0o777 == expected_mode;
        }
    }
    false
}

/// Helper to validate file ownership (Unix only)
#[cfg(unix)]
pub fn validate_file_ownership(path: &Path, expected_uid: u32, expected_gid: u32) -> bool {
    use std::os::unix::fs::MetadataExt;
    if let Ok(metadata) = fs::metadata(path) {
        return metadata.uid() == expected_uid && metadata.gid() == expected_gid;
    }
    false
}

/// Helper to check if a path is secure (no symlinks, proper permissions)
pub fn is_secure_path(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    // Check for symlinks
    if path.is_symlink() {
        return false;
    }

    // Check parent directories
    if let Some(parent) = path.parent() {
        if !is_secure_path(parent) {
            return false;
        }
    }

    // Basic permission check (world-writable is insecure)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            if mode & 0o002 != 0 { // world-writable
                return false;
            }
        }
    }

    true
}