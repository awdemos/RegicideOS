use crate::error::{PortCLError, Result};
use tracing::{error, warn, info};

pub fn handle_error(error: PortCLError) -> Result<()> {
    match error {
        PortCLError::Portage(msg) => {
            error!("Portage error: {}", msg);
            Err(PortCLError::Portage(msg))
        }
        PortCLError::RLEngine(msg) => {
            error!("RL engine error: {}", msg);
            Err(PortCLError::RLEngine(msg))
        }
        PortCLError::ActionExecution(msg) => {
            error!("Action execution error: {}", msg);
            Err(PortCLError::ActionExecution(msg))
        }
        PortCLError::Configuration(msg) => {
            warn!("Configuration error: {}", msg);
            Err(PortCLError::Configuration(msg))
        }
        PortCLError::System(msg) => {
            warn!("System error: {}", msg);
            Err(PortCLError::System(msg))
        }
        PortCLError::Network(msg) => {
            warn!("Network error: {}", msg);
            Err(PortCLError::Network(msg))
        }
        PortCLError::Timeout(msg) => {
            warn!("Timeout error: {}", msg);
            Err(PortCLError::Timeout(msg))
        }
        PortCLError::Validation(msg) => {
            warn!("Validation error: {}", msg);
            Err(PortCLError::Validation(msg))
        }
        PortCLError::Io(err) => {
            warn!("IO error: {}", err);
            Err(PortCLError::Io(err))
        }
        PortCLError::Json(err) => {
            warn!("JSON error: {}", err);
            Err(PortCLError::Json(err))
        }
        PortCLError::TomlDeserialize(err) => {
            warn!("TOML deserialization error: {}", err);
            Err(PortCLError::TomlDeserialize(err))
        }
        PortCLError::TomlSerialize(err) => {
            warn!("TOML serialization error: {}", err);
            Err(PortCLError::TomlSerialize(err))
        }
        PortCLError::NotFound(msg) => {
            warn!("Not found error: {}", msg);
            Err(PortCLError::NotFound(msg))
        }
        PortCLError::Service(msg) => {
            warn!("Service error: {}", msg);
            Err(PortCLError::Service(msg))
        }
        PortCLError::Resource(msg) => {
            warn!("Resource error: {}", msg);
            Err(PortCLError::Resource(msg))
        }
        PortCLError::Safety(msg) => {
            warn!("Safety error: {}", msg);
            Err(PortCLError::Safety(msg))
        }
    }
}

pub fn log_result<T>(result: Result<T>, operation: &str) -> Result<T> {
    match &result {
        Ok(_) => {
            info!("{} completed successfully", operation);
            result
        }
        Err(e) => {
            error!("{} failed: {}", operation, e);
            result
        }
    }
}

pub fn is_retryable_error(error: PortCLError) -> bool {
    match error {
        PortCLError::Network(_) => true,
        PortCLError::Timeout(_) => true,
        PortCLError::Io(_) => true,
        PortCLError::Portage(msg) => {
            // Some Portage errors might be retryable (e.g., temporary network issues)
            msg.contains("timeout") || msg.contains("network") || msg.contains("temporary")
        }
        PortCLError::System(msg) => {
            // Some system errors might be retryable (e.g., temporary resource issues)
            msg.contains("temporarily") || msg.contains("resource") || msg.contains("busy")
        }
        PortCLError::Resource(msg) => {
            // Resource errors might be retryable if temporary
            msg.contains("temporarily") || msg.contains("busy") || msg.contains("unavailable")
        }
        _ => false,
    }
}

pub fn error_severity(error: PortCLError) -> ErrorSeverity {
    match error {
        PortCLError::Portage(_) => ErrorSeverity::High,
        PortCLError::RLEngine(_) => ErrorSeverity::Medium,
        PortCLError::ActionExecution(_) => ErrorSeverity::High,
        PortCLError::Configuration(_) => ErrorSeverity::Critical,
        PortCLError::System(_) => ErrorSeverity::High,
        PortCLError::Network(_) => ErrorSeverity::Low,
        PortCLError::Timeout(_) => ErrorSeverity::Low,
        PortCLError::Validation(_) => ErrorSeverity::Medium,
        PortCLError::Io(_) => ErrorSeverity::Medium,
        PortCLError::Json(_) => ErrorSeverity::Low,
        PortCLError::TomlDeserialize(_) => ErrorSeverity::Low,
        PortCLError::TomlSerialize(_) => ErrorSeverity::Low,
        PortCLError::NotFound(_) => ErrorSeverity::Medium,
        PortCLError::Service(_) => ErrorSeverity::High,
        PortCLError::Resource(_) => ErrorSeverity::High,
        PortCLError::Safety(_) => ErrorSeverity::Critical,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl ErrorSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Critical => "critical",
            ErrorSeverity::High => "high",
            ErrorSeverity::Medium => "medium",
            ErrorSeverity::Low => "low",
        }
    }
}