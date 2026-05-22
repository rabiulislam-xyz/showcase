use crate::desktop::DesktopEntry;
use crate::dpkg::{self, DpkgInfo};
use crate::model::{App, AppError, Source};
use crate::runner::CommandRunner;
use std::collections::HashMap;

/// Build apt-sourced apps from already-scanned desktop entries.
/// `index` maps desktop path → package; `runner` provides dpkg-query output.
pub fn list_from(
    entries: &[DesktopEntry],
    index: &HashMap<String, String>,
    runner: &dyn CommandRunner,
) -> Result<Vec<App>, AppError> {
    // Resolve each entry to its owning package.
    let mut pkg_of: HashMap<String, String> = HashMap::new(); // path -> pkg
    for e in entries {
        let key = e.path.to_string_lossy().to_string();
        if let Some(pkg) = index.get(&key) {
            pkg_of.insert(key, pkg.clone());
        }
    }
    if pkg_of.is_empty() {
        return Ok(Vec::new());
    }

    // One batched dpkg-query for all packages.
    let packages: Vec<&str> = pkg_of.values().map(String::as_str).collect();
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

    let mut apps = Vec::new();
    for e in entries {
        let key = e.path.to_string_lossy().to_string();
        let Some(pkg) = pkg_of.get(&key) else { continue };
        let info = infos.get(pkg);
        apps.push(App {
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
        });
    }
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
}
