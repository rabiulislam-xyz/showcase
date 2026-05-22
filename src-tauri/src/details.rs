/// Extract the long description from `apt-cache show <pkg>` output.
///
/// Finds the first `Description:` or `Description-en:` header, then collects
/// the indented continuation lines underneath it. Leading space is stripped,
/// and lines that are exactly "." (the apt paragraph separator) are dropped.
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
            // A non-indented line ends the description block.
            break;
        }
    }

    // Drop trailing blank lines introduced by " ." separators.
    while lines_out.last().map(|s: &String| s.is_empty()).unwrap_or(false) {
        lines_out.pop();
    }

    if lines_out.is_empty() {
        None
    } else {
        Some(lines_out.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn description_colon_variant_also_works() {
        let input = "\
Description: short desc
 Long description text here.
";
        let desc = parse_apt_description(input).unwrap();
        assert_eq!(desc, "Long description text here.");
    }
}
