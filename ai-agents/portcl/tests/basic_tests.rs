//! Basic tests for working PortCL components

use portcl::utils::{format_duration, format_bytes, parse_package_name, validate_package_name};
use portcl::error::{PortCLError, Result};

#[test]
fn test_format_duration() {
    assert_eq!(format_duration(30), "30s");
    assert_eq!(format_duration(90), "1m 30s");
    assert_eq!(format_duration(3661), "1h 1m 1s");
}

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1536), "1.5 KB");
    assert_eq!(format_bytes(1048576), "1.0 MB");
}

#[test]
fn test_parse_package_name() {
    assert!(parse_package_name("sys-apps/portage").is_ok());
    assert!(parse_package_name("dev-lang/rust").is_ok());
    assert!(parse_package_name("invalid-package-name").is_err());
}

#[test]
fn test_validate_package_name() {
    assert!(validate_package_name("sys-apps/portage"));
    assert!(validate_package_name("dev-lang/rust"));
    assert!(!validate_package_name("invalid_package"));
    assert!(!validate_package_name("Sys-Apps/Portage"));
}

#[test]
fn test_error_types() {
    let error = PortCLError::Portage("test error".to_string());
    assert_eq!(error.to_string(), "Portage API error: test error");
}

#[test]
fn test_result_type() {
    let success: Result<i32> = Ok(42);
    assert!(success.is_ok());
    assert_eq!(success.unwrap(), 42);

    let failure: Result<i32> = Err(PortCLError::Validation("test error".to_string()));
    assert!(failure.is_err());
}