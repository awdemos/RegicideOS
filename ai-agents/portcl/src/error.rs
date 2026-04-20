use thiserror::Error;

pub type Result<T> = std::result::Result<T, PortCLError>;

#[derive(Error, Debug, Clone)]
pub enum PortCLError {
    #[error("Portage API error: {0}")]
    Portage(String),

    #[error("RL engine error: {0}")]
    RLEngine(String),

    #[error("Action execution error: {0}")]
    ActionExecution(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("JSON serialization error: {0}")]
    Json(String),

    #[error("TOML parsing error: {0}")]
    TomlDeserialize(String),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(String),

    #[error("System error: {0}")]
    System(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Service error: {0}")]
    Service(String),

    #[error("Resource error: {0}")]
    Resource(String),

    #[error("Safety error: {0}")]
    Safety(String),

    #[error("Mock error: {0}")]
    Mock(String),
}

impl From<std::io::Error> for PortCLError {
    fn from(e: std::io::Error) -> Self {
        PortCLError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for PortCLError {
    fn from(e: serde_json::Error) -> Self {
        PortCLError::Json(e.to_string())
    }
}

impl From<toml::de::Error> for PortCLError {
    fn from(e: toml::de::Error) -> Self {
        PortCLError::TomlDeserialize(e.to_string())
    }
}

impl From<toml::ser::Error> for PortCLError {
    fn from(e: toml::ser::Error) -> Self {
        PortCLError::TomlSerialize(e.to_string())
    }
}

impl From<reqwest::Error> for PortCLError {
    fn from(e: reqwest::Error) -> Self {
        PortCLError::Network(e.to_string())
    }
}
