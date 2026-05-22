use crate::aggregate::{self, Aggregated};
use crate::desktop;
use crate::dpkg;
use crate::icons;
use crate::model::App;
use crate::runner::SystemRunner;
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

/// Fill icon_path for each app using a name pulled from its desktop entry,
/// resolving against a prebuilt name -> path index.
fn resolve_icons(
    apps: &mut [App],
    entries: &[desktop::DesktopEntry],
    index: &HashMap<String, PathBuf>,
) {
    for app in apps.iter_mut() {
        // Match by name to a desktop entry's Icon= value.
        let icon_name = entries.iter().find_map(|e| {
            let matches_name = e.name.as_deref() == Some(app.name.as_str());
            if matches_name { e.icon.clone() } else { None }
        });
        if let Some(name) = icon_name {
            app.icon_path = icons::resolve_with_index(&name, index);
        }
    }
}

#[tauri::command]
pub fn list_apps() -> Vec<App> {
    enumerate().apps
}
