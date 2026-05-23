/// Scan `flatpak info` output for a single-line description.
///
/// `flatpak info` is NOT Debian RFC822-indented: the value sits on the same
/// line as the key. Looks for the first `Description:` line; falls back to
/// `Comment:` if none is found. Returns None if neither is present or both
/// have an empty value.
pub fn parse_flatpak_description(info_output: &str) -> Option<String> {
    info_output.lines().find_map(|l| {
        let trimmed = l.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("description:") || lower.starts_with("comment:") {
            let val = trimmed.split_once(':').map(|(_, v)| v).unwrap_or("").trim();
            if !val.is_empty() { Some(val.to_string()) } else { None }
        } else {
            None
        }
    })
}

/// Extract the long description from `apt-cache show <pkg>` output.
///
/// Finds a `Description:` or `Description-en:` header, then collects the
/// indented continuation lines underneath it. Leading space is stripped, and
/// lines that are exactly "." (the apt paragraph separator) are dropped.
///
/// `apt-cache show` can emit several stanzas (one per available version). If the
/// first stanza's Description has no long body, scanning continues into the
/// later stanzas before giving up.
pub fn parse_apt_description(show_output: &str) -> Option<String> {
    let mut lines_out: Vec<String> = Vec::new();
    let mut in_desc = false;

    for line in show_output.lines() {
        if !in_desc {
            // Match "Description:" or "Description-en:" (case-insensitive prefix).
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("description:") || lower.starts_with("description-en:") {
                in_desc = true;
                // The first line may carry a short description after the colon; skip it
                // here — the caller already has the summary from the App struct.
            }
            continue;
        }

        // Continuation lines start with exactly one space.
        if line.starts_with(' ') {
            let trimmed = line.trim_start_matches(' ');
            if trimmed == "." {
                // apt uses " ." as a blank-line separator between paragraphs.
                lines_out.push(String::new());
            } else {
                lines_out.push(trimmed.to_string());
            }
        } else {
            // A non-indented line ends this Description block. If we collected a
            // long description, we're done; otherwise keep scanning subsequent
            // stanzas for one that has a non-empty body.
            if collected_long_description(&lines_out).is_some() {
                break;
            }
            lines_out.clear();
            in_desc = false;
        }
    }

    collected_long_description(&lines_out)
}

/// Trim trailing blank lines (from " ." separators) and join, or None if empty.
fn collected_long_description(lines: &[String]) -> Option<String> {
    let mut end = lines.len();
    while end > 0 && lines[end - 1].is_empty() {
        end -= 1;
    }
    if end == 0 {
        None
    } else {
        Some(lines[..end].join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_flatpak_description ──────────────────────────────────────────────

    #[test]
    fn flatpak_description_line_is_extracted() {
        let input = "   ID: org.x.App\n   Description: A neat flatpak app\n   Version: 1.0\n";
        assert_eq!(parse_flatpak_description(input).as_deref(), Some("A neat flatpak app"));
    }

    #[test]
    fn flatpak_comment_is_fallback_when_no_description() {
        let input = "   Comment: Short comment line\n   Version: 1.0\n";
        assert_eq!(parse_flatpak_description(input).as_deref(), Some("Short comment line"));
    }

    #[test]
    fn flatpak_returns_none_when_neither_field_present() {
        let input = "   ID: org.x.App\n   Version: 1.0\n   Branch: stable\n";
        assert_eq!(parse_flatpak_description(input), None);
    }

    // ── parse_apt_description ──────────────────────────────────────────────────

    // Realistic `apt-cache show` excerpt.
    const SHOW_BLOCK: &str = "\
Package: gedit
Version: 42.2-0ubuntu1
Description-en: official text editor of the GNOME desktop environment
 GEdit is the official text editor of the GNOME desktop environment.
 While aiming at simplicity and ease of use, gedit is a powerful general
 purpose text editor.
 .
 This package contains the core gedit application.
Homepage: https://wiki.gnome.org/Apps/Gedit
";

    #[test]
    fn extracts_long_description_lines() {
        let desc = parse_apt_description(SHOW_BLOCK).unwrap();
        assert!(desc.contains("GEdit is the official"), "got: {desc}");
        assert!(desc.contains("powerful general"), "got: {desc}");
        assert!(desc.contains("core gedit application"), "got: {desc}");
    }

    #[test]
    fn drops_dot_separator_lines() {
        let desc = parse_apt_description(SHOW_BLOCK).unwrap();
        // " ." separators must not appear verbatim.
        assert!(!desc.contains("\n.\n"), "got: {desc}");
        assert!(!desc.starts_with('.'), "got: {desc}");
    }

    #[test]
    fn dot_becomes_blank_line_between_paragraphs() {
        let desc = parse_apt_description(SHOW_BLOCK).unwrap();
        // The " ." between the two paragraphs becomes an empty line.
        assert!(desc.contains("\n\n"), "expected blank line paragraph break, got: {desc}");
    }

    #[test]
    fn short_desc_line_itself_is_excluded() {
        // The text on the "Description:" line is the short summary; it must not
        // appear in the returned long description.
        let desc = parse_apt_description(SHOW_BLOCK).unwrap();
        assert!(
            !desc.starts_with("official text editor"),
            "short-desc line leaked into long desc: {desc}"
        );
    }

    #[test]
    fn returns_none_when_no_description_header() {
        let input = "Package: foo\nVersion: 1.0\n";
        assert_eq!(parse_apt_description(input), None);
    }

    #[test]
    fn returns_none_for_empty_input() {
        assert_eq!(parse_apt_description(""), None);
    }

    #[test]
    fn stops_at_next_unindented_field() {
        let input = "\
Description: short desc
 First long line.
 Second long line.
Homepage: https://example.com
 This should NOT be included.
";
        let desc = parse_apt_description(input).unwrap();
        assert!(desc.contains("First long line."), "got: {desc}");
        assert!(desc.contains("Second long line."), "got: {desc}");
        assert!(!desc.contains("This should NOT"), "got: {desc}");
    }

    #[test]
    fn scans_later_stanza_when_first_has_no_long_description() {
        // apt-cache show can emit multiple stanzas (e.g. several versions). The
        // first stanza here has only a short description on the header line and
        // no indented continuation; the long description lives in the second.
        let input = "\
Package: gimp
Version: 2.10.0
Description: image editor

Package: gimp
Version: 2.10.34
Description: GNU Image Manipulation Program
 The GIMP is an advanced picture editor.
 It can be used to edit photos and compose images.
";
        let desc = parse_apt_description(input).unwrap();
        assert!(desc.contains("advanced picture editor"), "got: {desc}");
        assert!(desc.contains("compose images"), "got: {desc}");
    }

    #[test]
    fn description_colon_variant_also_works() {
        let input = "\
Description: short desc
 Long description text here.
";
        let desc = parse_apt_description(input).unwrap();
        assert_eq!(desc, "Long description text here.");
    }
}
