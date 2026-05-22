use crate::aggregate::{self, Aggregated};
use crate::desktop;
use crate::details;
use crate::dpkg;
use crate::icons;
use crate::model::{App, AppError, AppList, Source};
use crate::runner::{CommandRunner, SystemRunner};
use crate::sources::{apt, flatpak, snap};
use std::collections::HashMap;
use std::path::PathBuf;

fn app_dirs() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from(format!("{home}/.local/share/applications")),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from(format!("{home}/.local/share/flatpak/exports/share/applications")),
    ]
}

fn icon_roots() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    vec![
        PathBuf::from(format!("{home}/.local/share/icons")),
        PathBuf::from("/usr/share/icons/hicolor"),
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
        PathBuf::from("/var/lib/flatpak/exports/share/icons"),
    ]
}

/// Enumerate all apps. Pure orchestration so it can be exercised without Tauri.
pub fn enumerate() -> Aggregated {
    // Scan desktop entries once; reuse for apt + icon names.
    let entries: Vec<_> = app_dirs().iter().flat_map(|d| desktop::scan_dir(d)).collect();
    let apt_entries: Vec<_> = entries
        .iter()
        .filter(|e| desktop::classify_source(&e.path) == crate::model::Source::Apt)
        .cloned()
        .collect();
    let index = dpkg::build_desktop_index(std::path::Path::new("/var/lib/dpkg/info"));

    // Run the three sources in parallel threads (std-only).
    let apt_handle = std::thread::spawn(move || {
        let runner = SystemRunner;
        apt::list_from(&apt_entries, &index, &runner)
    });
    let flatpak_handle = std::thread::spawn(|| flatpak::list(&SystemRunner));
    let snap_handle = std::thread::spawn(snap::list);

    let results = vec![
        ("apt", apt_handle.join().unwrap_or_else(|_| Err(crate::model::AppError::Backend("apt source thread panicked".into())))),
        ("flatpak", flatpak_handle.join().unwrap_or_else(|_| Err(crate::model::AppError::Backend("flatpak source thread panicked".into())))),
        ("snap", snap_handle.join().unwrap_or_else(|_| Err(crate::model::AppError::Backend("snap source thread panicked".into())))),
    ];
    let mut agg = aggregate::merge(results);
    // Build the icon index ONCE (O(tree), not O(apps × tree)).
    let icon_index = icons::build_index(&icon_roots());
    resolve_icons(&mut agg.apps, &entries, &icon_index);
    agg
}

/// Build a lookup map: (source, lowercase_key) -> icon_name.
///
/// Each desktop entry contributes up to three keys:
///   1. Its `Name=` value lowercased (works for apt; also helps snap/flatpak when
///      the human name matches).
///   2. For snap entries: the snap package name extracted from the filename stem
///      before the first '_' (e.g. `firefox_firefox.desktop` → `firefox`).
///   3. For flatpak entries: the last dot-segment of the app-id encoded in the
///      filename stem (e.g. `com.github.wwmm.easyeffects.desktop` → `easyeffects`).
///
/// O(entries) build, O(1) per-app lookup.
fn build_icon_lookup(
    entries: &[desktop::DesktopEntry],
) -> HashMap<(Source, String), String> {
    let mut map: HashMap<(Source, String), String> = HashMap::new();

    for entry in entries {
        let Some(icon) = entry.icon.as_deref() else { continue };
        let source = desktop::classify_source(&entry.path);

        // Key 1: lowercased human Name=.
        if let Some(name) = entry.name.as_deref() {
            map.entry((source, name.to_lowercase()))
                .or_insert_with(|| icon.to_string());
        }

        let stem = entry
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match source {
            Source::Snap => {
                // snap desktop files are named `<snap>_<app>.desktop`.
                let pkg = stem.split('_').next().unwrap_or(stem);
                if !pkg.is_empty() {
                    map.entry((Source::Snap, pkg.to_lowercase()))
                        .or_insert_with(|| icon.to_string());
                }
            }
            Source::Flatpak => {
                // Flatpak desktop file stem is the app-id; last segment is the short name.
                if let Some(short) = stem.rsplit('.').next() {
                    if !short.is_empty() {
                        map.entry((Source::Flatpak, short.to_lowercase()))
                            .or_insert_with(|| icon.to_string());
                    }
                }
            }
            Source::Apt => {}
        }
    }

    map
}

/// Fill icon_path for each app. Uses a two-level strategy:
///   1. Exact case-insensitive name match via the pre-built lookup map.
///   2. Source-specific heuristic key (snap pkg_ref, flatpak last dot-segment).
///   3. Falls back to None if no key hits.
///
/// O(apps + entries) overall.
fn resolve_icons(
    apps: &mut [App],
    entries: &[desktop::DesktopEntry],
    index: &HashMap<String, PathBuf>,
) {
    let lookup = build_icon_lookup(entries);

    for app in apps.iter_mut() {
        // Try keys in priority order; first hit wins.
        let icon_name = resolve_icon_name(app, &lookup);
        if let Some(name) = icon_name {
            app.icon_path = icons::resolve_with_index(&name, index);
        }
    }
}

/// Return the icon name for `app` by trying keys against `lookup`.
fn resolve_icon_name(
    app: &App,
    lookup: &HashMap<(Source, String), String>,
) -> Option<String> {
    let src = app.source;

    // Key 1: case-insensitive human name.
    if let Some(icon) = lookup.get(&(src, app.name.to_lowercase())) {
        return Some(icon.clone());
    }

    // Key 2: source-specific heuristic.
    let heuristic_key: Option<String> = match src {
        Source::Snap => Some(app.pkg_ref.to_lowercase()),
        Source::Flatpak => app
            .pkg_ref
            .rsplit('.')
            .next()
            .map(|s| s.to_lowercase()),
        Source::Apt => None,
    };
    if let Some(key) = heuristic_key {
        if let Some(icon) = lookup.get(&(src, key)) {
            return Some(icon.clone());
        }
    }

    None
}

/// Verify a uid corresponds to an installed, removable app. Defense against a
/// renderer requesting removal of an app that isn't shown / isn't removable.
pub fn validate_uninstall(source: Source, pkg_ref: &str, apps: &[App]) -> Result<(), AppError> {
    match apps.iter().find(|a| a.source == source && a.pkg_ref == pkg_ref) {
        None => Err(AppError::NotFound(format!(
            "{source:?}:{pkg_ref} is not an installed app"
        ))),
        Some(a) if !a.removable => Err(AppError::Protected(
            a.protected_reason
                .clone()
                .unwrap_or_else(|| "not removable".into()),
        )),
        Some(_) => Ok(()),
    }
}

#[tauri::command]
pub fn list_apps() -> AppList {
    let agg = enumerate();
    AppList { apps: agg.apps, warnings: agg.warnings }
}

/// Lazily fetch the long description for an app identified by `uid` ("source:pkg_ref").
/// Returns None on any error or when no description is available.
#[tauri::command]
pub fn get_app_details(uid: String) -> Option<String> {
    // Split on the first ':' only — pkg_ref may contain colons (flatpak app-ids do not,
    // but be defensive).
    let (source_str, pkg_ref) = uid.split_once(':')?;
    match source_str {
        "apt" => {
            let out = SystemRunner.run("apt-cache", &["show", pkg_ref]).ok()?;
            details::parse_apt_description(&out)
        }
        "flatpak" => {
            // `flatpak info` output is NOT Debian RFC822-indented, so parse_apt_description
            // must not be used here. Scan for the first "Description:" or "Comment:" line
            // and return the value on that same line.
            let out = SystemRunner.run("flatpak", &["info", pkg_ref]).ok()?;
            out.lines().find_map(|l| {
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
        "snap" => crate::snapd::get_snap_description(pkg_ref),
        _ => None,
    }
}

/// Remove an installed app identified by `uid` ("source:pkg_ref").
///
/// Guards fire before any privileged call; the heavy work runs off the async
/// thread via `spawn_blocking` so the Tauri runtime is not stalled.
#[tauri::command]
pub async fn uninstall_app(uid: String) -> Result<(), AppError> {
    let (src, pkg) = uid
        .split_once(':')
        .ok_or_else(|| AppError::NotFound(uid.clone()))?;
    let source = match src {
        "apt" => Source::Apt,
        "flatpak" => Source::Flatpak,
        "snap" => Source::Snap,
        _ => return Err(AppError::NotFound(uid.clone())),
    };
    let pkg = pkg.to_string();

    // Static guard (snap base/core packages) — runs before any privileged call.
    if let Some(r) = crate::uninstall::protected_reason(source, &pkg) {
        return Err(AppError::Protected(r));
    }

    tauri::async_runtime::spawn_blocking(move || {
        let runner = SystemRunner;

        // Authoritative server-side check: the app must actually be in the
        // current enumerated set AND be removable. Defends against a renderer
        // requesting removal of something it isn't allowed to (the rest below
        // stay as defense-in-depth).
        let agg = enumerate();
        validate_uninstall(source, &pkg, &agg.apps)?;

        // Dynamic apt-essential re-check (defense in depth).
        if source == Source::Apt && crate::uninstall::apt_is_essential(&runner, &pkg) {
            return Err(AppError::Protected(format!("{pkg} is an essential package")));
        }

        let (prog, args) = crate::uninstall::build_uninstall(source, &pkg);
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();

        CommandRunner::run(&runner, prog, &argv)
            .map(|_| ())
            .map_err(|e| match e {
                AppError::Backend(msg) => crate::uninstall::classify_error(&msg),
                other => other,
            })
    })
    .await
    .map_err(|e| AppError::Backend(format!("join: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(source: Source, pkg_ref: &str, removable: bool, reason: Option<&str>) -> App {
        App {
            uid: App::make_uid(source, pkg_ref),
            source,
            name: pkg_ref.to_string(),
            summary: None,
            description: None,
            version: None,
            icon_path: None,
            size_bytes: None,
            install_date: None,
            publisher: None,
            categories: vec![],
            exec: None,
            pkg_ref: pkg_ref.to_string(),
            removable,
            protected_reason: reason.map(str::to_string),
        }
    }

    #[test]
    fn validate_uninstall_ok_when_found_and_removable() {
        let apps = vec![app(Source::Apt, "gimp", true, None)];
        assert!(validate_uninstall(Source::Apt, "gimp", &apps).is_ok());
    }

    #[test]
    fn validate_uninstall_not_found_when_absent() {
        let apps = vec![app(Source::Apt, "gimp", true, None)];
        // Wrong source, same pkg_ref → still NotFound.
        assert!(matches!(
            validate_uninstall(Source::Snap, "gimp", &apps),
            Err(AppError::NotFound(_))
        ));
        // Unknown pkg_ref → NotFound.
        assert!(matches!(
            validate_uninstall(Source::Apt, "ghost", &apps),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn validate_uninstall_protected_when_not_removable() {
        let apps = vec![app(Source::Apt, "bash", false, Some("essential package"))];
        match validate_uninstall(Source::Apt, "bash", &apps) {
            Err(AppError::Protected(reason)) => assert_eq!(reason, "essential package"),
            other => panic!("expected Protected, got {other:?}"),
        }
    }

    #[test]
    fn validate_uninstall_protected_falls_back_to_default_reason() {
        let apps = vec![app(Source::Apt, "bash", false, None)];
        match validate_uninstall(Source::Apt, "bash", &apps) {
            Err(AppError::Protected(reason)) => assert_eq!(reason, "not removable"),
            other => panic!("expected Protected, got {other:?}"),
        }
    }
}
