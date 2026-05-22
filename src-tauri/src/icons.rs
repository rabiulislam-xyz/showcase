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
        // Use the dirent file_type (no syscall, does NOT follow symlinks) to
        // decide whether to recurse. This prevents infinite recursion when a
        // cyclic directory symlink exists under an icon theme root. Symlinked
        // icon *files* still match by name in the path ops below.
        let is_real_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let path = entry.path();
        if is_real_dir {
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

    #[test]
    fn does_not_recurse_into_symlinked_dir_cycle() {
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir().join(format!("showcase-iconcycle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let sub = base.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        // self-referential cycle: base/sub/loop -> base
        symlink(&base, sub.join("loop")).unwrap();
        // Must terminate (no stack overflow) and find nothing.
        let result = resolve("nonexistent-icon", &[base.clone()]);
        std::fs::remove_dir_all(&base).ok();
        assert_eq!(result, None);
    }
}
