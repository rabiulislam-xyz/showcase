use std::path::PathBuf;
use crate::model::Source;

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

/// Scan one directory, returning displayable entries from `*.desktop` files.
/// Unreadable files are skipped (logged by the caller if desired).
pub fn scan_dir(dir: &std::path::Path) -> Vec<DesktopEntry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            let parsed = parse_entry(path, &text);
            if parsed.should_display() {
                out.push(parsed);
            }
        }
    }
    out
}

/// Classify which package source owns a `.desktop` file by its location.
pub fn classify_source(path: &std::path::Path) -> Source {
    let p = path.to_string_lossy();
    if p.contains("/flatpak/") {
        Source::Flatpak
    } else if p.contains("/snapd/desktop/") {
        Source::Snap
    } else {
        Source::Apt
    }
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

    #[test]
    fn classifies_by_path() {
        use std::path::Path;
        assert_eq!(
            classify_source(Path::new("/var/lib/flatpak/exports/share/applications/x.desktop")),
            Source::Flatpak
        );
        assert_eq!(
            classify_source(Path::new("/var/lib/snapd/desktop/applications/firefox_firefox.desktop")),
            Source::Snap
        );
        assert_eq!(
            classify_source(Path::new("/usr/share/applications/gedit.desktop")),
            Source::Apt
        );
    }

    #[test]
    fn scan_dir_returns_only_displayable_desktop_files() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/apps");
        let entries = scan_dir(&dir);
        let names: Vec<_> = entries.iter().filter_map(|e| e.name.clone()).collect();
        assert!(names.contains(&"Good App".to_string()));
        assert!(!names.contains(&"Hidden App".to_string()));
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn parses_tricky_lines_and_ignores_other_groups() {
        let text = "[Desktop Entry]\n\
            Exec = env FOO=bar app %U\n\
            Type=Application\n\
            [Desktop Action New]\n\
            Name=Should Be Ignored\n";
        let e = parse_entry(PathBuf::from("/t.desktop"), text);
        // First '=' splits; embedded '=' in the value is preserved; spaces trimmed.
        assert_eq!(e.exec.as_deref(), Some("env FOO=bar app %U"));
        // Keys under a non-[Desktop Entry] group are not read.
        assert_eq!(e.name, None);

        // Missing Type must not be displayable.
        let no_type = parse_entry(PathBuf::from("/n.desktop"), "[Desktop Entry]\nName=X\n");
        assert!(!no_type.should_display());
    }
}
