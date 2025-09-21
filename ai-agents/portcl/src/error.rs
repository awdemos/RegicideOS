use thiserror::Error;

pub type Result<T> = std::result::Result<T, PortCLError>;

#[derive(Error, Debug)]
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
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("System error: {0}")]
    System(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

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
}