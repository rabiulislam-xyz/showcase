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
            let size_kib: u64 = parts.next()?.trim().parse().ok()?;
            let essential = matches!(parts.next().map(str::trim), Some("yes"));
            if package.is_empty() {
                return None;
            }
            Some(DpkgInfo {
                package: package.to_string(),
                version: version.to_string(),
                size_bytes: size_kib * 1024,
                essential,
            })
        })
        .collect()
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
}
