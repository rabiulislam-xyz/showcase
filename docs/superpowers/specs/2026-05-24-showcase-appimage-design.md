# Showcase — AppImage Support — Design Spec

- **Date:** 2026-05-24
- **Status:** Approved design
- **Builds on:** shipped Showcase (apt/flatpak/snap)

## Goal
Surface AppImage apps alongside apt/Flatpak/Snap — list them with metadata, launch them, and "uninstall" (delete the file + any menu integration). AppImages have no package manager, so discovery is filesystem-based.

## Scope
**In:** discover AppImages, show them as a 4th source, launch, uninstall (delete). 
**Out:** self-update (`--appimage-self-update` unreliable), installing/downloading AppImages, sandbox/permission editing, directory-watch.

## Data model
- `Source` gains an **`AppImage`** variant (serde `"appimage"`) — touches `model.rs`, `Source::parse`/`as_str`, frontend `types.ts`, source colors/labels (`avatar.ts`), and any exhaustive matches (`build_uninstall`, `build_launch_command`).
- For an AppImage `App`: `pkg_ref` = the AppImage's **absolute file path** (stable id); `desktop_path` = the registered `.desktop` path if integrated, else `None`; `removable: true`.

## Backend (`src-tauri/src/appimage.rs` — discovery + parsing, tested)
**Discover** = union of:
1. **Filesystem scan** of `~/Applications`, `~/.local/bin`, `/opt`, `~/Downloads`, `~/Desktop` for `*.AppImage` (case-insensitive). Each file → an App: name + version parsed from the filename (e.g. `Foo-1.2.3-x86_64.AppImage` → name "Foo", version "1.2.3"); generic/no icon; `pkg_ref` = path.
2. **Registered `.desktop` entries** (AppImageLauncher/appimaged) — already scanned by `desktop.rs`; identify them by `Exec=`/`TryExec=` pointing at a `*.AppImage` (or `X-AppImage-*` keys). These give real name + icon.
**Dedup** by the AppImage path: when both a scanned file and a registered entry resolve to the same `*.AppImage`, keep one (prefer the registered entry's name/icon, keep the path). Wire `appimage::list()` into `enumerate_with` as a 4th concurrent source. Pure parsing (`parse_appimage_filename`, classify-registered) is fixture-tested; the FS scan takes injectable roots.

**Launch** (extend `build_launch_command`): AppImage with a registered `desktop_path` → `gio launch <desktop_path>`; else run the file directly (`<path>`), detached — ensure executable bit first (`chmod +x` if missing).

**Uninstall** (extend the uninstall path): for AppImage, **delete the file** (and its registered `.desktop` + hicolor icon if present) rather than calling a package manager. Guard: only delete paths under the known scan dirs; if the path is outside `$HOME` (e.g. `/opt`) use `pkexec rm`/elevated delete, otherwise a plain `std::fs::remove_file`. Refuse anything not ending in `.AppImage`. Confirm dialog wording reflects "delete the file, frees ~X".

## Frontend
- 4th source: badge color + label ("AppImage"), a filter chip + count in the segmented control, and the category/source plumbing already generalizes.
- Uninstall confirm for an AppImage says "This deletes the AppImage file (frees ~X)" — not "remove package".
- Open works via the launch command.

## Testing
Rust: `parse_appimage_filename` (name/version variants, no-version, arch suffix); FS scan over a fixture dir (finds `*.AppImage`, ignores others); registered-entry detection + path dedup; `Source` serde round-trip incl. AppImage; uninstall path-guard (refuse non-`.AppImage`, refuse outside scan dirs); launch command for registered vs loose. Frontend: source badge/filter/count renders for AppImage; uninstall wording.

## Errors / edge cases
- AppImage file deleted/moved between list and action → `NotFound`/clear error toast.
- Non-executable loose AppImage → set `+x` then launch; if that fails, error toast.
- A registered `.desktop` whose AppImage no longer exists → skip (stale).
