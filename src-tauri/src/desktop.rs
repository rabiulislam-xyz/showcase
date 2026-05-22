use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopEntry {
    pub path: PathBuf,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub categories: Vec<String>,
    pub no_display: bool,
    pub hidden: bool,
    pub entry_type: Option<String>,
}

impl DesktopEntry {
    /// True if this entry represents a launchable, visible application.
    pub fn should_display(&self) -> bool {
        self.entry_type.as_deref() == Some("Application")
            && !self.no_display
            && !self.hidden
    }
}

/// Parse the `[Desktop Entry]` group only. Localized keys (e.g. `Name[de]`)
/// are ignored in favor of the unlocalized key. Unknown keys are skipped.
pub fn parse_entry(path: PathBuf, text: &str) -> DesktopEntry {
    let mut entry = DesktopEntry { path, ..Default::default() };
    let mut in_group = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_group = line == "[Desktop Entry]";
            continue;
        }
        if !in_group {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim();
        if key.contains('[') {
            continue; // localized variant
        }
        match key {
            "Name" => entry.name = Some(value.to_string()),
            "Comment" => entry.comment = Some(value.to_string()),
            "Icon" => entry.icon = Some(value.to_string()),
            "Exec" => entry.exec = Some(value.to_string()),
            "Type" => entry.entry_type = Some(value.to_string()),
            "NoDisplay" => entry.no_display = value.eq_ignore_ascii_case("true"),
            "Hidden" => entry.hidden = value.eq_ignore_ascii_case("true"),
            "Categories" => {
                entry.categories = value
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
            _ => {}
        }
    }
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[Desktop Entry]\n\
        Version=1.0\n\
        Name=Activity Log Manager\n\
        Name[de]=Aktivitaetsprotokoll\n\
        Comment=Configure logging\n\
        Icon=activity-log-manager\n\
        Exec=activity-log-manager\n\
        Type=Application\n\
        Categories=Settings;Utility;\n";

    #[test]
    fn parses_core_fields_and_ignores_localized() {
        let e = parse_entry(PathBuf::from("/x.desktop"), SAMPLE);
        assert_eq!(e.name.as_deref(), Some("Activity Log Manager"));
        assert_eq!(e.comment.as_deref(), Some("Configure logging"));
        assert_eq!(e.icon.as_deref(), Some("activity-log-manager"));
        assert_eq!(e.entry_type.as_deref(), Some("Application"));
        assert_eq!(e.categories, vec!["Settings", "Utility"]);
        assert!(!e.no_display && !e.hidden);
    }

    #[test]
    fn hidden_or_nodisplay_or_nonapp_excluded() {
        let app = parse_entry(PathBuf::from("/a"), "[Desktop Entry]\nType=Application\n");
        assert!(app.should_display());

        let nodisp = parse_entry(PathBuf::from("/b"), "[Desktop Entry]\nType=Application\nNoDisplay=true\n");
        assert!(!nodisp.should_display());

        let hidden = parse_entry(PathBuf::from("/c"), "[Desktop Entry]\nType=Application\nHidden=true\n");
        assert!(!hidden.should_display());

        let link = parse_entry(PathBuf::from("/d"), "[Desktop Entry]\nType=Link\n");
        assert!(!link.should_display());
    }
}
