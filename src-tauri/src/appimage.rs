use crate::desktop::DesktopEntry;
use crate::model::{App, AppError, Source};
use std::path::PathBuf;

/// Derive (display_name, version?) from an AppImage file name.
///
/// Strips the `.AppImage`/`.appimage` extension, removes a trailing arch token
/// (`-x86_64`, `-amd64`, `-aarch64`), and splits off a trailing `-<version>`
/// segment when the version starts with an ASCII digit.  Underscores in the
/// name component are replaced with spaces.
///
/// Examples:
///   "Foo-1.2.3-x86_64.AppImage" -> ("Foo", Some("1.2.3"))
///   "Bar.AppImage"              -> ("Bar", None)
///   "My_App-2.0.AppImage"       -> ("My App", Some("2.0"))
pub fn parse_appimage_filename(file_name: &str) -> (String, Option<String>) {
    let stem = file_name
        .strip_suffix(".AppImage")
        .or_else(|| file_name.strip_suffix(".appimage"))
        .unwrap_or(file_name);
    let stem = stem
        .trim_end_matches("-x86_64")
        .trim_end_matches("-amd64")
        .trim_end_matches("-aarch64");
    if let Some((name, last)) = stem.rsplit_once('-') {
        if last.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return (name.replace('_', " ").trim().to_string(), Some(last.to_string()));
        }
    }
    (stem.replace('_', " ").trim().to_string(), None)
}

/// Scan `roots` for `*.AppImage` files, returning one loosely-scanned App per file.
pub fn scan_appimages(roots: &[PathBuf]) -> Vec<App> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(read) = std::fs::read_dir(root) else { continue };
        for e in read.flatten() {
            let path = e.path();
            let is_appimage = path
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("appimage"))
                .unwrap_or(false);
            if !is_appimage { continue; }
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let (name, version) = parse_appimage_filename(fname);
            let abs = path.to_string_lossy().to_string();
            out.push(App {
                uid: App::make_uid(Source::AppImage, &abs),
                source: Source::AppImage,
                name,
                summary: None,
                description: None,
                version,
                icon_path: None,
                size_bytes: std::fs::metadata(&path).ok().map(|m| m.len()),
                install_date: None,
                publisher: None,
                categories: vec![],
                exec: Some(abs.clone()),
                desktop_path: None,
                pkg_ref: abs,
                removable: true,
                protected_reason: None,
                update_available: None,
            });
        }
    }
    out
}

/// Find AppImages registered by AppImageLauncher: desktop entries whose Exec
/// field points at a *.AppImage path.
///
/// Strips surrounding double-quotes from the Exec token before the `.appimage`
/// suffix check so a quoted path (e.g. `Exec="/a/b/Foo.AppImage" %U`) still
/// matches.
pub fn appimages_from_desktop(entries: &[DesktopEntry]) -> Vec<App> {
    let mut out = Vec::new();
    for entry in entries {
        let Some(exec_val) = entry.exec.as_deref() else { continue };
        let ai_path = exec_val
            .split_whitespace()
            .find_map(|tok| {
                // Strip surrounding double-quotes if present before checking the
                // suffix so `"/path/Foo.AppImage"` and `/path/Foo.AppImage` both match.
                let tok = tok.trim_matches('"');
                tok.to_ascii_lowercase()
                    .ends_with(".appimage")
                    .then_some(tok)
            });
        let Some(ai_path) = ai_path else { continue };
        let name = entry
            .name
            .clone()
            .unwrap_or_else(|| parse_appimage_filename(ai_path).0);
        out.push(App {
            uid: App::make_uid(Source::AppImage, ai_path),
            source: Source::AppImage,
            name,
            summary: entry.comment.clone(),
            description: None,
            version: None,
            icon_path: entry.icon.as_ref().map(PathBuf::from),
            size_bytes: None,
            install_date: None,
            publisher: None,
            categories: entry.categories.clone(),
            exec: Some(ai_path.to_string()),
            desktop_path: Some(entry.path.clone()),
            pkg_ref: ai_path.to_string(),
            removable: true,
            protected_reason: None,
            update_available: None,
        });
    }
    out
}

/// Union of filesystem-scanned AppImages and AppImageLauncher-registered entries,
/// deduplicated by pkg_ref (AppImage file path).
/// When both a scanned file and a registered desktop entry refer to the same
/// AppImage path, the registered entry's name and icon win.
pub fn list(roots: &[PathBuf], entries: &[DesktopEntry]) -> Vec<App> {
    let scanned = scan_appimages(roots);
    let registered = appimages_from_desktop(entries);
    let mut by_path: std::collections::HashMap<String, App> =
        scanned.into_iter().map(|a| (a.pkg_ref.clone(), a)).collect();
    for reg in registered {
        by_path
            .entry(reg.pkg_ref.clone())
            .and_modify(|existing| {
                existing.name = reg.name.clone();
                existing.icon_path = reg.icon_path.clone();
                existing.desktop_path = reg.desktop_path.clone();
                existing.summary = reg.summary.clone();
                existing.categories = reg.categories.clone();
                existing.uid = App::make_uid(Source::AppImage, &existing.pkg_ref);
            })
            .or_insert(reg);
    }
    by_path.into_values().collect()
}

/// Return the error for update attempts on AppImage apps.
pub fn update_error() -> AppError {
    AppError::Backend("AppImage apps cannot be updated".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_versioned_with_arch() {
        let (name, ver) = parse_appimage_filename("Foo-1.2.3-x86_64.AppImage");
        assert_eq!(name, "Foo");
        assert_eq!(ver, Some("1.2.3".to_string()));
    }

    #[test]
    fn parses_name_only() {
        let (name, ver) = parse_appimage_filename("Bar.AppImage");
        assert_eq!(name, "Bar");
        assert_eq!(ver, None);
    }

    #[test]
    fn parses_underscored_name_with_version() {
        let (name, ver) = parse_appimage_filename("My_App-2.0.AppImage");
        assert_eq!(name, "My App");
        assert_eq!(ver, Some("2.0".to_string()));
    }

    #[test]
    fn passthrough_no_extension() {
        let (name, ver) = parse_appimage_filename("SomeTool");
        assert_eq!(name, "SomeTool");
        assert_eq!(ver, None);
    }

    #[test]
    fn lowercase_extension_accepted() {
        let (name, ver) = parse_appimage_filename("Baz-3.1.appimage");
        assert_eq!(name, "Baz");
        assert_eq!(ver, Some("3.1".to_string()));
    }

    #[test]
    fn scan_finds_appimage_ignores_other_files() {
        let fixture_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/appimages");
        let apps = scan_appimages(&[fixture_dir]);
        assert_eq!(apps.len(), 1, "expected exactly one AppImage app, got: {apps:?}");
        let a = &apps[0];
        assert_eq!(a.name, "Foo");
        assert_eq!(a.version.as_deref(), Some("1.2.3"));
        assert_eq!(a.source, Source::AppImage);
        assert!(
            a.pkg_ref.ends_with("Foo-1.2.3-x86_64.AppImage"),
            "pkg_ref should be the file path, got: {}",
            a.pkg_ref
        );
    }

    #[test]
    fn scan_missing_dir_returns_empty() {
        let apps = scan_appimages(&[PathBuf::from("/nonexistent/path/xyz")]);
        assert!(apps.is_empty());
    }

    fn desktop_entry_fixture(name: &str, exec: &str) -> DesktopEntry {
        DesktopEntry {
            path: PathBuf::from(format!(
                "/home/u/.local/share/applications/{name}.desktop"
            )),
            name: Some(name.to_string()),
            comment: None,
            icon: Some(format!("{name}-icon")),
            exec: Some(exec.to_string()),
            categories: vec![],
            no_display: false,
            hidden: false,
            entry_type: Some("Application".to_string()),
        }
    }

    #[test]
    fn detects_registered_appimage_entry() {
        let entries = vec![desktop_entry_fixture(
            "Bar",
            "/home/u/Apps/Bar-2.0.AppImage %U",
        )];
        let apps = appimages_from_desktop(&entries);
        assert_eq!(apps.len(), 1);
        let a = &apps[0];
        assert_eq!(a.name, "Bar");
        assert_eq!(a.pkg_ref, "/home/u/Apps/Bar-2.0.AppImage");
        assert_eq!(a.source, Source::AppImage);
        assert!(a.desktop_path.is_some());
        assert_eq!(
            a.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            Some("Bar-icon".to_string())
        );
    }

    #[test]
    fn ignores_non_appimage_exec() {
        let entries = vec![desktop_entry_fixture("Snap App", "snap run myapp")];
        let apps = appimages_from_desktop(&entries);
        assert!(apps.is_empty());
    }

    #[test]
    fn detects_quoted_exec_path() {
        // Quoted path: Exec="/home/u/Apps/Foo.AppImage" %U
        let entries = vec![desktop_entry_fixture(
            "Foo",
            "\"/home/u/Apps/Foo.AppImage\" %U",
        )];
        let apps = appimages_from_desktop(&entries);
        assert_eq!(apps.len(), 1, "quoted Exec path must be detected: {apps:?}");
        let a = &apps[0];
        assert_eq!(a.pkg_ref, "/home/u/Apps/Foo.AppImage", "quotes must be stripped from path");
        assert_eq!(a.name, "Foo");
    }

    #[test]
    fn list_deduplicates_scanned_and_registered() {
        let fixture_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/appimages");
        let fixture_path = fixture_dir.join("Foo-1.2.3-x86_64.AppImage");
        let registered_name = "Foo Application";
        let entry = DesktopEntry {
            path: PathBuf::from("/home/u/.local/share/applications/Foo.desktop"),
            name: Some(registered_name.to_string()),
            comment: None,
            icon: Some("foo-registered-icon".to_string()),
            exec: Some(format!("{} %U", fixture_path.display())),
            categories: vec![],
            no_display: false,
            hidden: false,
            entry_type: Some("Application".to_string()),
        };
        let apps = list(&[fixture_dir], &[entry]);
        assert_eq!(apps.len(), 1, "dedup should yield one app, got: {apps:?}");
        let a = &apps[0];
        assert_eq!(a.name, registered_name);
        assert_eq!(
            a.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            Some("foo-registered-icon".to_string())
        );
        assert!(a.pkg_ref.ends_with("Foo-1.2.3-x86_64.AppImage"));
    }

    #[test]
    fn list_includes_registered_only_entry() {
        let entry = DesktopEntry {
            path: PathBuf::from("/home/u/.local/share/applications/Remote.desktop"),
            name: Some("Remote App".to_string()),
            comment: None,
            icon: Some("remote-icon".to_string()),
            exec: Some("/opt/Remote-1.0.AppImage %U".to_string()),
            categories: vec![],
            no_display: false,
            hidden: false,
            entry_type: Some("Application".to_string()),
        };
        let apps = list(&[], &[entry]);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Remote App");
    }
}
