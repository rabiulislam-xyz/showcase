use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Apt,
    Flatpak,
    Snap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    pub uid: String,
    pub source: Source,
    pub name: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub icon_path: Option<PathBuf>,
    pub size_bytes: Option<u64>,
    pub install_date: Option<String>,
    pub publisher: Option<String>,
    pub categories: Vec<String>,
    pub exec: Option<String>,
    pub pkg_ref: String,
    pub removable: bool,
    pub protected_reason: Option<String>,
}

impl Source {
    /// Lowercase id; matches the `serde(rename_all = "lowercase")` representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::Apt => "apt",
            Source::Flatpak => "flatpak",
            Source::Snap => "snap",
        }
    }
}

impl App {
    /// Stable identifier: "{source}:{pkg_ref}".
    pub fn make_uid(source: Source, pkg_ref: &str) -> String {
        format!("{}:{}", source.as_str(), pkg_ref)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("source unavailable: {0}")]
    SourceUnavailable(String),
    #[error("protected: {0}")]
    Protected(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("operation cancelled")]
    Cancelled,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Source::Apt).unwrap(), "\"apt\"");
        assert_eq!(serde_json::to_string(&Source::Flatpak).unwrap(), "\"flatpak\"");
        assert_eq!(serde_json::to_string(&Source::Snap).unwrap(), "\"snap\"");
    }

    #[test]
    fn uid_combines_source_and_ref() {
        assert_eq!(App::make_uid(Source::Snap, "firefox"), "snap:firefox");
    }
}
