use crate::error::{PortCLError, Result};
use serde::{de::DeserializeOwned, Serialize};

pub fn to_json_string<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}

pub fn to_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

pub fn from_json_string<T: DeserializeOwned>(json: &str) -> Result<T> {
    Ok(serde_json::from_str(json)?)
}

pub fn from_json_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}

pub fn to_toml_string<T: Serialize>(value: &T) -> Result<String> {
    Ok(toml::to_string_pretty(value)?)
}

pub fn from_toml_string<T: DeserializeOwned>(toml: &str) -> Result<T> {
    Ok(toml::from_str(toml)?)
}
