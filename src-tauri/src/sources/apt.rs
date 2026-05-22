use crate::desktop::DesktopEntry;
use crate::dpkg::{self, DpkgInfo};
use crate::model::{App, AppError, Source};
use crate::runner::CommandRunner;
use std::collections::HashMap;

/// Build apt-sourced apps from already-scanned desktop entries.
/// `index` maps desktop path → package; `runner` provides dpkg-query output.
///
/// Emits exactly one App per distinct package — the first entry encountered for
/// that package supplies name/summary/icon/exec/categories.
pub fn list_from(
    entries: &[DesktopEntry],
    index: &HashMap<String, String>,
    runner: &dyn CommandRunner,
) -> Result<Vec<App>, AppError> {
    // Single pass: resolve each entry to its package, keeping the first entry
    // per package as the representative.
    let mut seen: HashMap<String, &DesktopEntry> = HashMap::new(); // pkg -> first entry
    for e in entries {
        let key = e.path.to_string_lossy();
        if let Some(pkg) = index.get(key.as_ref()) {
            seen.entry(pkg.clone()).or_insert(e);
        }
    }
    if seen.is_empty() {
        return Ok(Vec::new());
    }

    // One batched dpkg-query for all distinct packages.
    let packages: Vec<&str> = seen.keys().map(String::as_str).collect();
    let mut args = vec![
        "-W",
        "-f=${Package}\t${Version}\t${Installed-Size}\t${Essential}\n",
    ];
    args.extend(packages.iter().copied());
    let output = runner.run("dpkg-query", &args)?;
    let infos: HashMap<String, DpkgInfo> = dpkg::parse_query(&output)
        .into_iter()
        .map(|i| (i.package.clone(), i))
        .collect();

    let apps = seen
        .iter()
        .map(|(pkg, e)| {
            let info = infos.get(pkg);
            App {
                uid: App::make_uid(Source::Apt, pkg),
                source: Source::Apt,
                name: e.name.clone().unwrap_or_else(|| pkg.clone()),
                summary: e.comment.clone(),
                description: None,
                version: info.map(|i| i.version.clone()),
                icon_path: None, // resolved later by icons module
                size_bytes: info.map(|i| i.size_bytes),
                install_date: None,
                publisher: None,
                categories: e.categories.clone(),
                exec: e.exec.clone(),
                pkg_ref: pkg.clone(),
                removable: info.map(|i| !i.essential).unwrap_or(true),
                protected_reason: info
                    .filter(|i| i.essential)
                    .map(|_| "Essential system package".to_string()),
            }
        })
        .collect();
    Ok(apps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::parse_entry;
    use crate::runner::FakeRunner;
    use std::path::PathBuf;

    #[test]
    fn builds_app_with_metadata_and_protection() {
        let entry = parse_entry(
            PathBuf::from("/usr/share/applications/org.gnome.gedit.desktop"),
            "[Desktop Entry]\nType=Application\nName=Text Editor\nComment=Edit text\nIcon=gedit\nExec=gedit %U\n",
        );
        let mut index = HashMap::new();
        index.insert(
            "/usr/share/applications/org.gnome.gedit.desktop".to_string(),
            "gedit".to_string(),
        );
        let runner = FakeRunner::new()
            .with("dpkg-query", "gedit\t41.0-2\t4096\t\n");

        let apps = list_from(&[entry], &index, &runner).unwrap();
        assert_eq!(apps.len(), 1);
        let a = &apps[0];
        assert_eq!(a.uid, "apt:gedit");
        assert_eq!(a.name, "Text Editor");
        assert_eq!(a.version.as_deref(), Some("41.0-2"));
        assert_eq!(a.size_bytes, Some(4096 * 1024));
        assert!(a.removable);
        assert_eq!(a.pkg_ref, "gedit");
    }

    #[test]
    fn essential_package_is_not_removable() {
        let entry = parse_entry(
            PathBuf::from("/usr/share/applications/bash.desktop"),
            "[Desktop Entry]\nType=Application\nName=Bash\n",
        );
        let mut index = HashMap::new();
        index.insert("/usr/share/applications/bash.desktop".to_string(), "bash".to_string());
        let runner = FakeRunner::new().with("dpkg-query", "bash\t5.1\t1864\tyes\n");

        let apps = list_from(&[entry], &index, &runner).unwrap();
        assert!(!apps[0].removable);
        assert_eq!(apps[0].protected_reason.as_deref(), Some("Essential system package"));
    }

    #[test]
    fn dedups_multiple_entries_for_same_package() {
        let entry1 = parse_entry(
            PathBuf::from("/usr/share/applications/gimp.desktop"),
            "[Desktop Entry]\nType=Application\nName=GIMP\nExec=gimp %U\n",
        );
        let entry2 = parse_entry(
            PathBuf::from("/usr/share/applications/gimp-2.10.desktop"),
            "[Desktop Entry]\nType=Application\nName=GIMP 2.10\nExec=gimp-2.10 %U\n",
        );
        let mut index = HashMap::new();
        index.insert("/usr/share/applications/gimp.desktop".to_string(), "gimp".to_string());
        index.insert("/usr/share/applications/gimp-2.10.desktop".to_string(), "gimp".to_string());
        let runner = FakeRunner::new().with("dpkg-query", "gimp\t2.10\t9000\t\n");

        let apps = list_from(&[entry1, entry2], &index, &runner).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].uid, "apt:gimp");
    }

    #[test]
    fn degraded_mode_no_dpkg_info() {
        // Entry maps to a package, but dpkg-query returns nothing for it.
        let entry = parse_entry(
            PathBuf::from("/usr/share/applications/foo.desktop"),
            "[Desktop Entry]\nType=Application\nName=Foo\nExec=foo\n",
        );
        let mut index = HashMap::new();
        index.insert("/usr/share/applications/foo.desktop".to_string(), "foo".to_string());
        // Runner returns empty output — package not found in dpkg db.
        let runner = FakeRunner::new().with("dpkg-query", "");

        let apps = list_from(&[entry], &index, &runner).unwrap();
        assert_eq!(apps.len(), 1);
        let a = &apps[0];
        assert!(a.version.is_none());
        assert!(a.size_bytes.is_none());
        assert!(a.removable);
    }
}
