use crate::error::{PortCLError, Result};
use serde::{de::DeserializeOwned, Serialize};

pub fn to_json_string<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value)
        .map_err(|e| PortCLError::Json(e))
}

pub fn to_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value)
        .map_err(|e| PortCLError::Json(e))
}

pub fn from_json_string<T: DeserializeOwned>(json: &str) -> Result<T> {
    serde_json::from_str(json)
        .map_err(|e| PortCLError::Json(e))
}

pub fn from_json_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes)
        .map_err(|e| PortCLError::Json(e))
}

pub fn to_toml_string<T: Serialize>(value: &T) -> Result<String> {
    toml::to_string_pretty(value)
        .map_err(|e| PortCLError::Toml(e))
}

pub fn from_toml_string<T: DeserializeOwned>(toml: &str) -> Result<T> {
    toml::from_str(toml)
        .map_err(|e| PortCLError::Toml(e))
}