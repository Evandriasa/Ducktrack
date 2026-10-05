use std::fmt;
use std::error::Error as StdError;

use serde::ser::{Serialize, SerializeMap, Serializer};

#[derive(Debug)]
pub enum AppError {
    Db(rusqlite::Error),
    Validation(String),
    NotFound(String),
    Io(std::io::Error),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(e) => write!(f, "Database error: {e}"),
            AppError::Validation(m) => write!(f, "{m}"),
            AppError::NotFound(m) => write!(f, "{m}"),
            AppError::Io(e) => write!(f, "File error: {e}"),
            AppError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl StdError for AppError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            AppError::Db(e) => Some(e),
            AppError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("message", &self.to_string())?;
        map.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

pub fn ensure_key_valid(key: &str) -> Result<(), AppError> {
    if key.is_empty() || key.len() > 12 {
        return Err(AppError::Validation(
            "Key must be 1-12 characters long.".into(),
        ));
    }
    let ok = key
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    if !ok {
        return Err(AppError::Validation(
            "Key may only contain uppercase letters, digits and underscores.".into(),
        ));
    }
    Ok(())
}

pub fn ensure_non_empty(value: &str, field: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field} must not be empty.")));
    }
    Ok(())
}