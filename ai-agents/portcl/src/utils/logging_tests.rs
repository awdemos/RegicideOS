#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_setup_logging_with_info_level() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("test.log");

        let result = setup_logging("info", Some(log_file.as_path()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_setup_logging_with_debug_level() {
        let result = setup_logging("debug", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_setup_logging_with_invalid_level() {
        let result = setup_logging("invalid", None);
        assert!(result.is_ok()); // Should default to INFO
    }

    #[test]
    fn test_setup_logging_with_file_output() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("test.log");

        let result = setup_logging("info", Some(log_file.as_path()));
        assert!(result.is_ok());

        // Check if log file was created
        assert!(log_file.exists());
    }

    #[test]
    fn test_setup_logging_creates_parent_directory() {
        let temp_dir = tempdir().unwrap();
        let nested_dir = temp_dir.path().join("nested").join("logs");
        let log_file = nested_dir.join("test.log");

        let result = setup_logging("info", Some(log_file.as_path()));
        assert!(result.is_ok());

        // Check if nested directory was created
        assert!(nested_dir.exists());
        assert!(nested_dir.is_dir());
    }
}