use std::collections::HashMap;
use std::path::{Path, PathBuf};

const EXTS: [&str; 3] = ["png", "svg", "xpm"];

/// Build a name -> path index by walking each root once (skipping symlinked
/// dirs to prevent infinite recursion). First occurrence of a name wins.
pub fn build_index(roots: &[PathBuf]) -> HashMap<String, PathBuf> {
    let mut index = HashMap::new();
    for root in roots {
        index_dir(root, &mut index);
    }
    index
}

fn index_dir(dir: &Path, index: &mut HashMap<String, PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut subdirs = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        // Use dirent file_type (no syscall, does NOT follow symlinks) so
        // cyclic directory symlinks are never recursed into.
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        if is_dir {
            subdirs.push(path);
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if EXTS.contains(&ext) {
                index.entry(stem.to_string()).or_insert(path);
            }
        }
    }
    for sub in subdirs {
        index_dir(&sub, index);
    }
}

/// Resolve a `.desktop` Icon= value using a prebuilt index.
/// - Absolute existing paths are returned as-is.
/// - Otherwise looks up the name in the index.
///
/// Returns the first match, or None.
pub fn resolve_with_index(icon: &str, index: &HashMap<String, PathBuf>) -> Option<PathBuf> {
    if icon.is_empty() {
        return None;
    }
    let p = Path::new(icon);
    if p.is_absolute() && p.exists() {
        return Some(p.to_path_buf());
    }
    index.get(icon).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> Vec<PathBuf> {
        vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/icons")]
    }

    #[test]
    fn build_index_finds_fixture_icons() {
        let index = build_index(&roots());
        assert!(index.contains_key("gedit"));
        assert!(index.contains_key("inkscape"));
    }

    #[test]
    fn resolve_with_index_passthrough_and_missing() {
        let idx: HashMap<String, PathBuf> = HashMap::new();
        assert_eq!(resolve_with_index("", &idx), None);
        assert_eq!(resolve_with_index("does-not-exist", &idx), None);
        // Absolute existing path passes through (use this source file itself).
        let me = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/icons.rs");
        assert_eq!(resolve_with_index(me.to_str().unwrap(), &idx), Some(me));
    }

    #[test]
    fn build_index_does_not_recurse_into_symlinked_dir_cycle() {
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir()
            .join(format!("showcase-iconcycle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let sub = base.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        symlink(&base, sub.join("loop")).unwrap();
        // Must terminate (no stack overflow); cycle dir has no icons.
        let idx = build_index(std::slice::from_ref(&base));
        std::fs::remove_dir_all(&base).ok();
        assert!(idx.is_empty());
    }
}
