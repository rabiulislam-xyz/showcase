use crate::aggregate::{self, Aggregated};
use crate::desktop;
use crate::details;
use crate::dpkg;
use crate::icons;
use crate::model::{App, AppError, AppList, Source};
use crate::runner::{CommandRunner, SystemRunner};
use crate::sources::snap::{SnapSource, SnapdSocket};
use crate::sources::{apt, flatpak};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
/// Thin wrapper over [`enumerate_with`] supplying the real system seams.
pub fn enumerate() -> Aggregated {
    enumerate_with(
        &SystemRunner,
        &SnapdSocket,
        &app_dirs(),
        &icon_roots(),
        Path::new("/var/lib/dpkg/info"),
    )
}

/// Enumerate apps from injectable seams so the full pipeline is testable.
///
/// Scans desktop dirs, classifies entries, builds the dpkg index from
/// `dpkg_info_dir`, runs the apt/flatpak/snap sources in parallel, merges them,
/// and resolves icons against `icon_roots`.
///
/// The three sources run inside a `std::thread::scope` so the borrowed `runner`
/// and `snap` refs can be shared without `'static`. A scoped-thread panic is
/// mapped to a per-source `Backend` warning, exactly as before — one failing
/// source never drops the others.
pub fn enumerate_with(
    runner: &(dyn CommandRunner + Sync),
    snap: &dyn SnapSource,
    app_dirs: &[PathBuf],
    icon_roots: &[PathBuf],
    dpkg_info_dir: &Path,
) -> Aggregated {
    // Scan desktop entries once; reuse for apt + icon names.
    let entries: Vec<_> = app_dirs.iter().flat_map(|d| desktop::scan_dir(d)).collect();
    let apt_entries: Vec<_> = entries
        .iter()
        .filter(|e| desktop::classify_source(&e.path) == Source::Apt)
        .cloned()
        .collect();
    let index = dpkg::build_desktop_index(dpkg_info_dir);

    // Run the three sources in parallel scoped threads so the borrowed
    // `runner`/`snap` refs can cross thread boundaries without `'static`.
    let results = std::thread::scope(|scope| {
        let apt_handle = scope.spawn(|| apt::list_from(&apt_entries, &index, runner));
        let flatpak_handle = scope.spawn(|| flatpak::list(runner));
        let snap_handle = scope.spawn(|| snap.list());

        vec![
            ("apt", join_source(apt_handle, "apt")),
            ("flatpak", join_source(flatpak_handle, "flatpak")),
            ("snap", join_source(snap_handle, "snap")),
        ]
    });

    let mut agg = aggregate::merge(results);
    // Build the icon index ONCE (O(tree), not O(apps × tree)).
    let icon_index = icons::build_index(icon_roots);
    resolve_icons(&mut agg.apps, &entries, &icon_index);
    agg
}

/// Join a scoped source thread, turning a panic into a `Backend` warning so a
/// single source crash degrades to a warning instead of killing enumeration.
fn join_source(
    handle: std::thread::ScopedJoinHandle<'_, Result<Vec<App>, AppError>>,
    name: &str,
) -> Result<Vec<App>, AppError> {
    handle
        .join()
        .unwrap_or_else(|_| Err(AppError::Backend(format!("{name} source thread panicked"))))
}

/// Build a lookup map: (source, lowercase_key) -> icon_name.
///
/// Each desktop entry contributes up to three keys:
///   1. Its `Name=` value lowercased (works for apt; also helps snap/flatpak when
///      the human name matches).
///   2. For snap entries: the snap package name extracted from the filename stem
///      before the first '_' (e.g. `firefox_firefox.desktop` → `firefox`).
///   3. For flatpak entries: BOTH the full app-id (filename stem, e.g.
///      `org.gnome.Calculator`) as the primary key AND the last dot-segment
///      (e.g. `calculator`) as a fallback. The full app-id avoids collisions
///      between apps that share a short segment (org.gnome.Calculator vs
///      org.kde.Calculator).
///
/// O(entries) build, O(1) per-app lookup.
pub(crate) fn build_icon_lookup(
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
                // Flatpak desktop file stem is the app-id. Key on the FULL app-id
                // first (collision-free), then the short last segment as fallback.
                if !stem.is_empty() {
                    map.entry((Source::Flatpak, stem.to_lowercase()))
                        .or_insert_with(|| icon.to_string());
                }
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
pub(crate) fn resolve_icon_name(
    app: &App,
    lookup: &HashMap<(Source, String), String>,
) -> Option<String> {
    let src = app.source;

    // Key 1: case-insensitive human name.
    if let Some(icon) = lookup.get(&(src, app.name.to_lowercase())) {
        return Some(icon.clone());
    }

    // Key 2: source-specific heuristic key(s), tried in priority order. For
    // flatpak the FULL app-id is tried before the short last-segment so two
    // apps sharing a short segment (org.gnome.Calculator / org.kde.Calculator)
    // each resolve to their own icon instead of colliding.
    let heuristic_keys: Vec<String> = match src {
        Source::Snap => vec![app.pkg_ref.to_lowercase()],
        Source::Flatpak => {
            let mut keys = vec![app.pkg_ref.to_lowercase()];
            if let Some(short) = app.pkg_ref.rsplit('.').next() {
                keys.push(short.to_lowercase());
            }
            keys
        }
        Source::Apt => Vec::new(),
    };
    for key in heuristic_keys {
        if let Some(icon) = lookup.get(&(src, key)) {
            return Some(icon.clone());
        }
    }

    None
}

/// Verify a uid corresponds to an installed, removable app. Defense against a
/// renderer requesting removal of an app that isn't shown / isn't removable.
pub(crate) fn validate_uninstall(source: Source, pkg_ref: &str, apps: &[App]) -> Result<(), AppError> {
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
    app_details_with(source_str, pkg_ref, &SystemRunner, &SnapdSocket)
}

/// Resolve a long description from injectable seams. `source` is the raw source
/// id ("apt"/"flatpak"/"snap"); unknown ids and colon-less uids return None.
pub fn app_details_with(
    source: &str,
    pkg_ref: &str,
    runner: &dyn CommandRunner,
    snap: &dyn SnapSource,
) -> Option<String> {
    match source {
        "apt" => {
            let out = runner.run("apt-cache", &["show", pkg_ref]).ok()?;
            details::parse_apt_description(&out)
        }
        "flatpak" => {
            let out = runner.run("flatpak", &["info", pkg_ref]).ok()?;
            details::parse_flatpak_description(&out)
        }
        "snap" => snap.description(pkg_ref),
        _ => None,
    }
}

/// Run the full uninstall guard chain, then the privileged removal. Sync and
/// runner-injectable so the whole sequence is testable without real processes.
///
/// Guard order (all run before any privileged call):
///   1. `protected_reason` — static base/core-snap denylist.
///   2. `validate_uninstall` — app must be in the enumerated set AND removable.
///   3. `apt_is_essential` — dynamic dpkg re-check for apt (defense in depth).
///
/// Then `build_uninstall` + `runner.run`, mapping a Backend stderr through
/// `classify_error`. Note `protected_reason` is checked first so base/core
/// snaps (which are not enumerated as apps) still report Protected, not
/// NotFound — matching prior behavior.
pub fn perform_uninstall(
    source: Source,
    pkg_ref: &str,
    runner: &dyn CommandRunner,
    apps: &[App],
) -> Result<(), AppError> {
    if let Some(r) = crate::uninstall::protected_reason(source, pkg_ref) {
        return Err(AppError::Protected(r));
    }

    // Authoritative server-side check: the app must actually be in the current
    // enumerated set AND be removable. Defends against a renderer requesting
    // removal of something it isn't allowed to.
    validate_uninstall(source, pkg_ref, apps)?;

    // Dynamic apt-essential re-check (defense in depth).
    if source == Source::Apt && crate::uninstall::apt_is_essential(runner, pkg_ref) {
        return Err(AppError::Protected(format!("{pkg_ref} is an essential package")));
    }

    let (prog, args) = crate::uninstall::build_uninstall(source, pkg_ref);
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();

    runner
        .run(prog, &argv)
        .map(|_| ())
        .map_err(|e| match e {
            AppError::Backend(msg) => crate::uninstall::classify_error(&msg),
            other => other,
        })
}

/// Launch an installed app identified by `uid` ("source:pkg_ref").
///
/// Performs an authoritative lookup to confirm the app is installed before
/// spawning. The child process is detached (new process group, stdio closed)
/// so it outlives Showcase and does not block the Tauri runtime.
/// Launch is an unprivileged user action — no polkit.
#[tauri::command]
pub fn launch_app(uid: String) -> Result<(), crate::model::AppError> {
    use crate::model::{AppError, Source};
    let (src, pkg) = uid.split_once(':').ok_or_else(|| AppError::NotFound(uid.clone()))?;
    let source = Source::parse(src).ok_or_else(|| AppError::NotFound(uid.clone()))?;
    // Authoritative lookup: the app must actually be installed.
    let apps = enumerate().apps;
    let app = apps
        .iter()
        .find(|a| a.source == source && a.pkg_ref == pkg)
        .ok_or_else(|| AppError::NotFound(uid.clone()))?;
    let dp = app.desktop_path.as_ref().map(|p| p.to_string_lossy().into_owned());
    let (prog, args) = crate::launch::build_launch_command(source, dp.as_deref(), pkg);
    // Detached fire-and-forget: new process group so SIGHUP does not propagate;
    // all stdio closed so the child has no reference to our file descriptors.
    use std::os::unix::process::CommandExt as _;
    std::process::Command::new(prog)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .map(|_child| ())
        .map_err(|e| AppError::Backend(format!("launch failed: {e}")))
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
    let source = Source::parse(src)
        .ok_or_else(|| AppError::NotFound(uid.clone()))?;
    let pkg = pkg.to_string();

    // Enumerate the app set AND run the full guard + removal chain off the async
    // runtime: enumeration does blocking file/process/socket I/O, so it must not
    // run on the executor thread.
    tauri::async_runtime::spawn_blocking(move || {
        let apps = enumerate().apps;
        perform_uninstall(source, &pkg, &SystemRunner, &apps)
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
            desktop_path: None,
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

    fn flatpak_entry(app_id: &str, icon: &str) -> desktop::DesktopEntry {
        desktop::DesktopEntry {
            // Path must contain "/flatpak/" so classify_source returns Flatpak,
            // and the file stem is the app-id.
            path: PathBuf::from(format!(
                "/var/lib/flatpak/exports/share/applications/{app_id}.desktop"
            )),
            // Same human Name on purpose: forces resolution onto the app-id key,
            // not the (also-colliding) Name= key.
            name: Some("Calculator".into()),
            comment: None,
            icon: Some(icon.into()),
            exec: None,
            categories: vec![],
            no_display: false,
            hidden: false,
            entry_type: Some("Application".into()),
        }
    }

    #[test]
    fn flatpak_same_short_segment_different_app_ids_resolve_to_own_icon() {
        let entries = vec![
            flatpak_entry("org.gnome.Calculator", "org.gnome.Calculator"),
            flatpak_entry("org.kde.Calculator", "org.kde.Calculator"),
        ];
        let lookup = build_icon_lookup(&entries);

        let gnome = app(Source::Flatpak, "org.gnome.Calculator", true, None);
        let kde = app(Source::Flatpak, "org.kde.Calculator", true, None);

        assert_eq!(
            resolve_icon_name(&gnome, &lookup).as_deref(),
            Some("org.gnome.Calculator")
        );
        assert_eq!(
            resolve_icon_name(&kde, &lookup).as_deref(),
            Some("org.kde.Calculator")
        );
    }

    #[test]
    fn flatpak_exact_name_match_still_resolves() {
        let entries = vec![flatpak_entry("org.gnome.Calculator", "calc-icon")];
        let lookup = build_icon_lookup(&entries);
        // App whose name matches the entry Name= (case-insensitively).
        let mut app = app(Source::Flatpak, "org.gnome.Calculator", true, None);
        app.name = "CALCULATOR".into();
        assert_eq!(resolve_icon_name(&app, &lookup).as_deref(), Some("calc-icon"));
    }

    #[test]
    fn snap_pkg_ref_heuristic_resolves_icon() {
        // No Name match; the snap pkg_ref keys the icon via the `<snap>_<app>`
        // desktop filename stem.
        let entry = desktop::DesktopEntry {
            path: PathBuf::from(
                "/var/lib/snapd/desktop/applications/firefox_firefox.desktop",
            ),
            name: Some("Firefox Web Browser".into()),
            comment: None,
            icon: Some("firefox-snap-icon".into()),
            exec: None,
            categories: vec![],
            no_display: false,
            hidden: false,
            entry_type: Some("Application".into()),
        };
        let lookup = build_icon_lookup(std::slice::from_ref(&entry));
        let mut snap = app(Source::Snap, "firefox", true, None);
        snap.name = "Mozilla Firefox".into(); // no Name= collision
        assert_eq!(resolve_icon_name(&snap, &lookup).as_deref(), Some("firefox-snap-icon"));
    }

    #[test]
    fn resolve_icon_name_returns_none_when_no_key_hits() {
        let entries = vec![flatpak_entry("org.gnome.Calculator", "calc-icon")];
        let lookup = build_icon_lookup(&entries);
        // Apt app: no heuristic keys, no Name match → None.
        let orphan = app(Source::Apt, "nonexistent", true, None);
        assert_eq!(resolve_icon_name(&orphan, &lookup), None);
    }

    // ── enumerate_with ─────────────────────────────────────────────────────────

    use crate::runner::FakeRunner;
    use crate::sources::snap::test_support::FakeSnap;
    use std::sync::Mutex;

    fn icon_fixture_roots() -> Vec<PathBuf> {
        vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/icons")]
    }

    /// Build a self-contained temp tree: an apt + a flatpak desktop file, plus a
    /// dpkg `.list` mapping the apt desktop path to a package. Returns
    /// (base, app_dirs, dpkg_info_dir). Caller removes the base dir.
    fn make_enumerate_tree(tag: &str) -> (PathBuf, Vec<PathBuf>, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "showcase-enum-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let apt_dir = base.join("usr/share/applications");
        let flatpak_dir = base.join("flatpak/exports/share/applications");
        let dpkg_dir = base.join("dpkg-info");
        std::fs::create_dir_all(&apt_dir).unwrap();
        std::fs::create_dir_all(&flatpak_dir).unwrap();
        std::fs::create_dir_all(&dpkg_dir).unwrap();

        // Apt desktop entry; Icon=gedit resolves against the icon fixture.
        let apt_desktop = apt_dir.join("org.gnome.gedit.desktop");
        std::fs::write(
            &apt_desktop,
            "[Desktop Entry]\nType=Application\nName=Text Editor\nComment=Edit text\nIcon=gedit\nExec=gedit %U\n",
        )
        .unwrap();

        // Flatpak desktop entry; path contains "/flatpak/" so it classifies as
        // Flatpak. Stem is the app-id; Icon=inkscape resolves via the fixture.
        std::fs::write(
            flatpak_dir.join("org.x.App.desktop"),
            "[Desktop Entry]\nType=Application\nName=X App\nIcon=inkscape\nExec=xapp\n",
        )
        .unwrap();

        // dpkg .list mapping the apt desktop's REAL path → package "gedit".
        std::fs::write(
            dpkg_dir.join("gedit.list"),
            format!("{}\n/usr/bin/gedit\n", apt_desktop.display()),
        )
        .unwrap();

        (base, vec![apt_dir, flatpak_dir], dpkg_dir)
    }

    #[test]
    fn enumerate_with_merges_sorts_and_resolves_icons() {
        let (base, app_dirs, dpkg_dir) = make_enumerate_tree("merge");

        let runner = FakeRunner::new()
            // apt: dpkg-query for the one mapped package.
            .with("dpkg-query", "gedit\t42.0\t4096\t\n")
            // flatpak: one app row (app_id matches the flatpak desktop stem).
            .with("flatpak", "org.x.App\tX App\t1.0\t10 MB\tflathub\n");

        let snap = FakeSnap::new().with_apps(vec![app(Source::Snap, "firefox", true, None)]);

        let agg = enumerate_with(&runner, &snap, &app_dirs, &icon_fixture_roots(), &dpkg_dir);
        std::fs::remove_dir_all(&base).ok();

        // All three sources merged.
        let uids: Vec<&str> = agg.apps.iter().map(|a| a.uid.as_str()).collect();
        assert!(uids.contains(&"apt:gedit"), "apt app missing: {uids:?}");
        assert!(uids.contains(&"flatpak:org.x.App"), "flatpak app missing: {uids:?}");
        assert!(uids.contains(&"snap:firefox"), "snap app missing: {uids:?}");
        assert!(agg.warnings.is_empty(), "unexpected warnings: {:?}", agg.warnings);

        // Sorted case-insensitively by name then uid: Firefox, Text Editor, X App.
        let names: Vec<&str> = agg.apps.iter().map(|a| a.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_by_key(|n| n.to_lowercase());
        assert_eq!(names, sorted, "apps not sorted by name: {names:?}");

        // Icon resolved for the apt fixture app (Icon=gedit → gedit.png).
        let apt = agg.apps.iter().find(|a| a.uid == "apt:gedit").unwrap();
        let icon = apt.icon_path.as_ref().expect("apt icon_path not resolved");
        assert!(icon.ends_with("gedit.png"), "unexpected icon: {icon:?}");

        // Icon resolved for the flatpak app via its app-id stem (Icon=inkscape).
        let flat = agg.apps.iter().find(|a| a.uid == "flatpak:org.x.App").unwrap();
        let ficon = flat.icon_path.as_ref().expect("flatpak icon_path not resolved");
        assert!(ficon.ends_with("inkscape.svg"), "unexpected flatpak icon: {ficon:?}");
    }

    #[test]
    fn enumerate_with_failing_source_becomes_warning_others_survive() {
        let (base, app_dirs, dpkg_dir) = make_enumerate_tree("warn");

        let runner = FakeRunner::new()
            .with("dpkg-query", "gedit\t42.0\t4096\t\n")
            .with("flatpak", "org.x.App\tX App\t1.0\t10 MB\tflathub\n");

        // Snap source fails; apt + flatpak must still produce apps.
        let snap = FakeSnap::failing(AppError::SourceUnavailable("snapd down".into()));

        let agg = enumerate_with(&runner, &snap, &app_dirs, &icon_fixture_roots(), &dpkg_dir);
        std::fs::remove_dir_all(&base).ok();

        let uids: Vec<&str> = agg.apps.iter().map(|a| a.uid.as_str()).collect();
        assert!(uids.contains(&"apt:gedit"));
        assert!(uids.contains(&"flatpak:org.x.App"));
        assert!(!uids.contains(&"snap:firefox"));
        assert_eq!(agg.warnings.len(), 1, "expected exactly one warning");
        assert!(agg.warnings[0].contains("snap"), "warning: {}", agg.warnings[0]);
    }

    // ── app_details_with ───────────────────────────────────────────────────────

    #[test]
    fn app_details_with_apt_parses_apt_cache_show() {
        let runner = FakeRunner::new().with(
            "apt-cache",
            "Package: gedit\nDescription: short\n First long line.\n Second line.\n",
        );
        let snap = FakeSnap::new();
        let desc = app_details_with("apt", "gedit", &runner, &snap).unwrap();
        assert!(desc.contains("First long line."), "got: {desc}");
        assert!(desc.contains("Second line."), "got: {desc}");
    }

    #[test]
    fn app_details_with_flatpak_scans_single_line_description() {
        // `flatpak info` is NOT RFC822-indented: the value sits on the same line.
        let runner = FakeRunner::new().with(
            "flatpak",
            "        ID: org.x.App\n   Description: A neat flatpak app\n   Version: 1.0\n",
        );
        let snap = FakeSnap::new();
        let desc = app_details_with("flatpak", "org.x.App", &runner, &snap).unwrap();
        assert_eq!(desc, "A neat flatpak app");
    }

    #[test]
    fn app_details_with_flatpak_falls_back_to_comment() {
        let runner = FakeRunner::new()
            .with("flatpak", "   Comment: Short comment line\n   Version: 1.0\n");
        let snap = FakeSnap::new();
        let desc = app_details_with("flatpak", "org.x.App", &runner, &snap).unwrap();
        assert_eq!(desc, "Short comment line");
    }

    #[test]
    fn app_details_with_snap_uses_snap_description() {
        let runner = FakeRunner::new();
        let snap = FakeSnap::new().with_description("firefox", "A web browser from Mozilla");
        let desc = app_details_with("snap", "firefox", &runner, &snap).unwrap();
        assert_eq!(desc, "A web browser from Mozilla");
        // Unknown snap → None.
        assert_eq!(app_details_with("snap", "ghost", &runner, &snap), None);
    }

    #[test]
    fn app_details_with_unknown_source_is_none() {
        let runner = FakeRunner::new();
        let snap = FakeSnap::new();
        assert_eq!(app_details_with("wat", "anything", &runner, &snap), None);
    }

    #[test]
    fn app_details_with_apt_failure_is_none() {
        // No fake registered → runner errors → None (no panic).
        let runner = FakeRunner::new();
        let snap = FakeSnap::new();
        assert_eq!(app_details_with("apt", "gedit", &runner, &snap), None);
    }

    // ── perform_uninstall (guard ordering, via a spy runner) ────────────────────

    /// Runner that records every (program, argv) call and returns canned output.
    struct SpyRunner {
        calls: Mutex<Vec<(String, Vec<String>)>>,
        responses: HashMap<String, Result<String, AppError>>,
    }

    impl SpyRunner {
        fn new() -> Self {
            Self { calls: Mutex::new(Vec::new()), responses: HashMap::new() }
        }
        fn with(mut self, program: &str, out: &str) -> Self {
            self.responses.insert(program.to_string(), Ok(out.to_string()));
            self
        }
        fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }
        fn programs(&self) -> Vec<String> {
            self.calls().into_iter().map(|(p, _)| p).collect()
        }
    }

    impl CommandRunner for SpyRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, AppError> {
            self.calls.lock().unwrap().push((
                program.to_string(),
                args.iter().map(|s| s.to_string()).collect(),
            ));
            self.responses
                .get(program)
                .cloned()
                .unwrap_or_else(|| Err(AppError::Backend(format!("no fake for {program}"))))
        }
    }

    #[test]
    fn perform_uninstall_unknown_uid_is_not_found_and_runs_nothing() {
        let runner = SpyRunner::new();
        let apps = vec![app(Source::Apt, "gimp", true, None)];
        let res = perform_uninstall(Source::Apt, "ghost", &runner, &apps);
        assert!(matches!(res, Err(AppError::NotFound(_))));
        assert!(runner.calls().is_empty(), "no privileged/dpkg call expected");
    }

    #[test]
    fn perform_uninstall_non_removable_is_protected_and_runs_nothing() {
        let runner = SpyRunner::new();
        let apps = vec![app(Source::Apt, "bash", false, Some("essential package"))];
        let res = perform_uninstall(Source::Apt, "bash", &runner, &apps);
        assert!(matches!(res, Err(AppError::Protected(_))));
        assert!(runner.calls().is_empty());
    }

    #[test]
    fn perform_uninstall_protected_snap_is_protected_and_runs_nothing() {
        let runner = SpyRunner::new();
        // core22 included as removable; protected_reason must still reject it
        // (static guard) before any runner call.
        let apps = vec![app(Source::Snap, "core22", true, None)];
        let res = perform_uninstall(Source::Snap, "core22", &runner, &apps);
        assert!(matches!(res, Err(AppError::Protected(_))));
        assert!(runner.calls().is_empty(), "protected snap must not invoke runner");
    }

    #[test]
    fn perform_uninstall_apt_essential_recheck_blocks_before_pkexec() {
        // Removable per the app set, but the dynamic dpkg essential re-check says
        // "yes" → Protected, and ONLY the dpkg-query runs (never pkexec).
        let runner = SpyRunner::new().with("dpkg-query", "yes");
        let apps = vec![app(Source::Apt, "base-files", true, None)];
        let res = perform_uninstall(Source::Apt, "base-files", &runner, &apps);
        assert!(matches!(res, Err(AppError::Protected(_))));
        assert_eq!(runner.programs(), vec!["dpkg-query".to_string()]);
        assert!(
            !runner.programs().contains(&"pkexec".to_string()),
            "pkexec must not run when essential re-check blocks"
        );
    }

    #[test]
    fn perform_uninstall_happy_path_invokes_pkexec_with_argv() {
        let runner = SpyRunner::new()
            .with("dpkg-query", "no") // essential re-check → not essential
            .with("pkexec", ""); // removal succeeds
        let apps = vec![app(Source::Apt, "gimp", true, None)];
        let res = perform_uninstall(Source::Apt, "gimp", &runner, &apps);
        assert!(res.is_ok(), "expected Ok, got {res:?}");

        let calls = runner.calls();
        // dpkg-query (essential re-check), then pkexec apt-get remove.
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "dpkg-query");
        assert_eq!(calls[1].0, "pkexec");
        assert_eq!(calls[1].1, vec!["apt-get", "-y", "remove", "gimp"]);
    }

    #[test]
    fn perform_uninstall_flatpak_happy_path_skips_essential_check() {
        // Non-apt: the dpkg essential re-check is skipped entirely.
        let runner = SpyRunner::new().with("flatpak", "");
        let apps = vec![app(Source::Flatpak, "org.gimp.GIMP", true, None)];
        let res = perform_uninstall(Source::Flatpak, "org.gimp.GIMP", &runner, &apps);
        assert!(res.is_ok(), "expected Ok, got {res:?}");

        let calls = runner.calls();
        assert_eq!(calls.len(), 1, "only the flatpak uninstall should run");
        assert_eq!(calls[0].0, "flatpak");
        assert_eq!(calls[0].1, vec!["uninstall", "--app", "-y", "org.gimp.GIMP"]);
    }

    #[test]
    fn perform_uninstall_maps_auth_failure_to_permission_denied() {
        let mut runner = SpyRunner::new().with("dpkg-query", "no");
        // Realistic polkit-cancel stderr from the privileged call.
        runner.responses.insert(
            "pkexec".to_string(),
            Err(AppError::Backend("Not authorized to perform operation".into())),
        );
        let apps = vec![app(Source::Apt, "gimp", true, None)];
        let res = perform_uninstall(Source::Apt, "gimp", &runner, &apps);
        assert!(
            matches!(res, Err(AppError::PermissionDenied(_))),
            "auth-cancel stderr must classify as PermissionDenied, got {res:?}"
        );
    }
}
