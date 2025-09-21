//! Contract tests for PortCL
//!
//! These tests verify that PortCL adheres to its API contracts and
//! maintains backward compatibility. They test against formal specifications.

// Import our new contract tests
mod test_list_tests;
mod test_execute_tests;
mod test_get_test;
mod test_coverage;
mod test_benchmarks;

#[cfg(test)]
mod contract_tests {
    use super::*;
    use portcl::error::{PortCLError, Result};

    #[test]
    fn test_error_contract_implementations() {
        // Test that PortCLError implements required traits
        let error = PortCLError::Portage("test".to_string());

        // Should implement Display
        let display_str = format!("{}", error);
        assert!(!display_str.is_empty());

        // Should implement Debug
        let debug_str = format!("{:?}", error);
        assert!(!debug_str.is_empty());

        // Should implement Clone
        let cloned_error = error.clone();
        assert_eq!(format!("{}", error), format!("{}", cloned_error));
    }

    #[test]
    fn test_result_type_alias_contract() {
        // Test that Result type alias works correctly
        let success: Result<i32> = Ok(42);
        let failure: Result<i32> = Err(PortCLError::Portage("test".to_string()));

        assert!(success.is_ok());
        assert!(failure.is_err());
        assert_eq!(success.unwrap(), 42);
    }

    #[test]
    fn test_utility_functions_contract() {
        use portcl::utils::{format_duration, format_bytes};

        // Test that utility functions return non-empty strings
        assert!(!format_duration(60).is_empty());
        assert!(!format_bytes(1024).is_empty());

        // Test that they handle edge cases
        assert!(!format_duration(0).is_empty());
        assert!(!format_bytes(0).is_empty());
    }

    #[test]
    fn test_module_structure_contract() {
        // Test that all expected modules are accessible
        use portcl::error;
        use portcl::utils;

        // Should be able to access core types
        let _error_type: error::PortCLError = error::PortCLError::Portage("test".to_string());

        // Should be able to access utility functions
        let _formatted_duration = utils::format_duration(60);
        let _formatted_bytes = utils::format_bytes(1024);
    }
}