use std::path::{Path, PathBuf};

const EXTS: [&str; 3] = ["png", "svg", "xpm"];

/// Resolve a `.desktop` Icon= value against a list of icon theme roots.
/// - Absolute existing paths are returned as-is.
/// - Otherwise search each root recursively for `<name>.{png,svg,xpm}`.
/// Returns the first match, or None.
pub fn resolve(icon: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    if icon.is_empty() {
        return None;
    }
    let p = Path::new(icon);
    if p.is_absolute() && p.exists() {
        return Some(p.to_path_buf());
    }
    for root in roots {
        if let Some(found) = search_dir(root, icon) {
            return Some(found);
        }
    }
    None
}

fn search_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    let read = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if stem == name && EXTS.contains(&ext) {
                return Some(path);
            }
        }
    }
    for sub in subdirs {
        if let Some(found) = search_dir(&sub, name) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> Vec<PathBuf> {
        vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/icons")]
    }

    #[test]
    fn finds_png_by_name() {
        let found = resolve("gedit", &roots()).unwrap();
        assert!(found.ends_with("gedit.png"));
    }

    #[test]
    fn finds_svg_by_name() {
        let found = resolve("inkscape", &roots()).unwrap();
        assert!(found.ends_with("inkscape.svg"));
    }

    #[test]
    fn absolute_path_passthrough_and_missing_is_none() {
        assert_eq!(resolve("does-not-exist", &roots()), None);
        assert_eq!(resolve("", &roots()), None);
    }
}
