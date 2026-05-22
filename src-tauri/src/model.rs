use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Apt,
    Flatpak,
    Snap,
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
}
