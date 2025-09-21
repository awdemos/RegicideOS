//! Unit tests for PortCL

pub mod error_tests;
pub mod serde_tests;
pub mod utils_tests;
pub mod test_monitor;
pub mod test_actions;
pub mod test_rl_engine;
pub mod test_config;
pub mod test_error;

#[cfg(test)]
mod tests {
    use super::*;
    use portcl::utils::{format_duration, format_bytes};

    #[test]
    fn test_basic_functionality() {
        // Test that basic utilities work
        assert_eq!(format_duration(60), "1m 0s");
        assert_eq!(format_bytes(1024), "1.0 KB");
    }

    #[test]
    fn test_error_types() {
        // Test that error types work correctly
        use portcl::error::PortCLError;
        let error = PortCLError::Portage("test error".to_string());
        assert_eq!(error.to_string(), "Portage API error: test error");
    }

    #[test]
    fn test_result_types() {
        // Test Result type alias
        use portcl::error::Result;
        let success: Result<i32> = Ok(42);
        assert!(success.is_ok());
    }
}