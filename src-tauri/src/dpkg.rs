use std::collections::HashMap;
use std::path::Path;

/// One package's fields from `dpkg-query -W -f='${Package}\t${Version}\t${Installed-Size}\t${Essential}\n'`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpkgInfo {
    pub package: String,
    pub version: String,
    pub size_bytes: u64, // Installed-Size is reported in KiB
    pub essential: bool,
}

/// Parse multi-line dpkg-query output. Malformed lines are skipped.
pub fn parse_query(output: &str) -> Vec<DpkgInfo> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let package = parts.next()?.trim();
            let version = parts.next()?.trim();
            // Empty ${Installed-Size} means "unknown" → treat as 0, not a reason to
            // drop the row (which would also discard the essential flag below).
            let size_field = parts.next()?.trim();
            let size_kib: u64 = if size_field.is_empty() {
                0
            } else {
                size_field.parse().ok()?
            };
            let essential = matches!(parts.next().map(str::trim), Some("yes"));
            if package.is_empty() {
                return None;
            }
            Some(DpkgInfo {
                package: package.to_string(),
                version: version.to_string(),
                size_bytes: size_kib.saturating_mul(1024),
                essential,
            })
        })
        .collect()
}

/// Build a map of `installed file path → owning package` by reading the
/// `*.list` files under a dpkg info dir (default `/var/lib/dpkg/info`).
/// Only `.desktop` paths are retained (all we need for app→package mapping).
pub fn build_desktop_index(info_dir: &Path) -> HashMap<String, String> {
    let mut index = HashMap::new();
    let Ok(read) = std::fs::read_dir(info_dir) else { return index };
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("list") {
            continue;
        }
        // file name like "gedit:amd64.list" or "gedit.list" -> package "gedit"
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let package = stem.split(':').next().unwrap_or(stem).to_string();
        if let Ok(text) = std::fs::read_to_string(&path) {
            for line in text.lines() {
                if line.ends_with(".desktop") {
                    index.insert(line.to_string(), package.clone());
                }
            }
        }
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lines_and_converts_kib_to_bytes() {
        let out = "bash\t5.1-6ubuntu1.1\t1864\tyes\ngedit\t41.0-2\t4096\t\nbad-line\n";
        let infos = parse_query(out);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0], DpkgInfo {
            package: "bash".into(), version: "5.1-6ubuntu1.1".into(),
            size_bytes: 1864 * 1024, essential: true,
        });
        assert_eq!(infos[1].package, "gedit");
        assert!(!infos[1].essential);
    }

    #[test]
    fn empty_installed_size_keeps_row_with_zero_size() {
        // An empty ${Installed-Size} must NOT drop the package — that would also
        // lose its essential flag, making a protected metapackage look removable.
        let infos = parse_query("foo\t1.0\t\tyes\n");
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0], DpkgInfo {
            package: "foo".into(), version: "1.0".into(),
            size_bytes: 0, essential: true,
        });
    }

    #[test]
    fn reverse_index_maps_desktop_paths_to_package() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dpkg-info");
        let idx = build_desktop_index(&dir);
        assert_eq!(
            idx.get("/usr/share/applications/org.gnome.gedit.desktop").map(String::as_str),
            Some("gedit")
        );
        assert!(!idx.contains_key("/usr/bin/gedit"));
    }
}
