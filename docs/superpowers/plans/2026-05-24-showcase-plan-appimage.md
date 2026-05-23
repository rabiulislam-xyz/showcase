# AppImage Support — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) tracking. Extends a mature codebase — follow existing patterns (`enumerate_with` source merge, `desktop.rs` parsing, `build_launch_command`/`build_uninstall`, `Source` enum + `Source::parse`/`as_str`, `avatar.ts`, filter/source chips). Spec: `docs/superpowers/specs/2026-05-24-showcase-appimage-design.md`. **Recommended to land AFTER the Update plan** (smaller blast radius first), but independent.

**Goal:** Discover AppImage apps (filesystem scan + AppImageLauncher-registered entries), show them as a 4th source, launch them, and uninstall (delete the file + integration).

**Architecture:** New `Source::AppImage` variant; new `appimage.rs` (filename parsing + dir scan + registered-entry detection, fixture-tested) wired into `enumerate_with` as a 4th source; `build_launch_command` + the uninstall path extended for AppImage (delete-file with a path guard). Frontend generalizes the existing source badge/filter to a 4th source.

**Tech Stack:** Tauri v2 (Rust), SvelteKit (Svelte 5), Vitest, cargo test.

---

## File Structure
- `src-tauri/src/model.rs` — `Source::AppImage` + `Source::parse`/`as_str` arms.
- `src-tauri/src/appimage.rs` — **new**: `parse_appimage_filename`, `scan_appimages(roots)`, `appimages_from_desktop(entries)`, `list(roots, entries)`, `is_removable_appimage_path`.
- `src-tauri/src/{launch.rs, uninstall.rs, updates.rs}` — add `Source::AppImage` match arms.
- `src-tauri/src/commands.rs` — wire `appimage::list` into `enumerate_with`; handle AppImage in the uninstall command (delete vs exec).
- `src-tauri/src/lib.rs` — `pub mod appimage;`.
- Frontend: `src/lib/types.ts` (`"appimage"`), `src/lib/avatar.ts` (label+color), `src/lib/components/{Header,AppDetail}.svelte` (filter chip + confirm wording), tests.

---

## Phase A — Source variant

### Task A1: add `Source::AppImage`
**Files:** Modify `src-tauri/src/model.rs` (+ every exhaustive `match source` — compiler will list them: `launch.rs`, `uninstall.rs`, `updates.rs`, `Source::as_str`, `Source::parse`).
- [ ] **Step 1 (test first):** extend the `Source` serde test — `Source::AppImage` ↔ `"appimage"`; `Source::parse("appimage") == Some(Source::AppImage)`; `as_str()=="appimage"`.
- [ ] **Step 2:** add the `AppImage` variant; add arms to `as_str` (`"appimage"`) and `parse`. Fix every now-non-exhaustive match: `build_launch_command`/`build_uninstall`/`build_update_command` — for now add `Source::AppImage => unreachable!()`/a clear `todo`-free placeholder that the later tasks replace (B-launch, C-uninstall, and "no update for AppImage" in updates). Keep it compiling.
- [ ] **Step 3:** `cargo test` green. Commit `feat(model): add Source::AppImage`.

---

## Phase B — Discovery (pure, TDD)

### Task B1: `parse_appimage_filename`
**Files:** Create `src-tauri/src/appimage.rs`; `pub mod appimage;` in lib.rs.
- [ ] **Step 1 (test first):**
```rust
/// Derive (display_name, version?) from an AppImage file name.
/// "Foo-1.2.3-x86_64.AppImage" -> ("Foo", Some("1.2.3"))
/// "Bar.AppImage"              -> ("Bar", None)
/// "My_App-2.0.AppImage"       -> ("My App", Some("2.0"))
pub fn parse_appimage_filename(file_name: &str) -> (String, Option<String>) {
    let stem = file_name.strip_suffix(".AppImage").or_else(|| file_name.strip_suffix(".appimage")).unwrap_or(file_name);
    // drop a trailing arch token
    let stem = stem.trim_end_matches("-x86_64").trim_end_matches("-amd64").trim_end_matches("-aarch64");
    // split trailing "-<version>" where version starts with a digit
    if let Some((name, last)) = stem.rsplit_once('-') {
        if last.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return (name.replace(['_'], " ").trim().to_string(), Some(last.to_string()));
        }
    }
    (stem.replace(['_'], " ").trim().to_string(), None)
}
```
Tests for the three cases above + no-extension passthrough.
- [ ] **Step 2:** PASS. Commit `feat(appimage): parse filename → name/version`.

### Task B2: `scan_appimages(roots)`
**Files:** Modify `appimage.rs`. Test fixtures under `src-tauri/tests/fixtures/appimages/` (create empty `Foo-1.2.3-x86_64.AppImage` + a non-AppImage `readme.txt`).
- [ ] **Step 1 (test first):**
```rust
use crate::model::{App, Source};
use std::path::{Path, PathBuf};

/// Scan dirs for *.AppImage files → one App each (loose entry; generic icon).
pub fn scan_appimages(roots: &[PathBuf]) -> Vec<App> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(read) = std::fs::read_dir(root) else { continue };
        for e in read.flatten() {
            let path = e.path();
            let is_ai = path.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("appimage")).unwrap_or(false);
            if !is_ai { continue; }
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let (name, version) = parse_appimage_filename(fname);
            let abs = path.to_string_lossy().to_string();
            out.push(App {
                uid: App::make_uid(Source::AppImage, &abs),
                source: Source::AppImage, name, summary: None, description: None,
                version, icon_path: None, size_bytes: std::fs::metadata(&path).ok().map(|m| m.len()),
                install_date: None, publisher: None, categories: vec![], exec: Some(abs.clone()),
                desktop_path: None, pkg_ref: abs, removable: true, protected_reason: None,
                update_available: None, // (field exists only if Update plan landed; omit otherwise)
            });
        }
    }
    out
}
```
Test: scanning the fixtures dir returns one App (`Foo`, `1.2.3`), `source==AppImage`, `pkg_ref` = the file path, ignores `readme.txt`.
- [ ] **Step 2:** PASS. Commit `feat(appimage): scan dirs for AppImage files`.

### Task B3: registered-entry detection + `list` + dedup
**Files:** Modify `appimage.rs`.
- [ ] **Step 1 (test first):**
```rust
use crate::desktop::DesktopEntry;
/// AppImages integrated by AppImageLauncher: a desktop entry whose Exec/TryExec
/// points at a *.AppImage. Returns (App with name/icon from the entry, AppImage path).
pub fn appimages_from_desktop(entries: &[DesktopEntry]) -> Vec<App> { /* find exec token ending .AppImage; build App source=AppImage, pkg_ref=that path, desktop_path=entry.path, icon from entry */ }

/// Union of scanned files + registered entries, deduped by AppImage path
/// (prefer the registered entry's name/icon; keep the path).
pub fn list(roots: &[PathBuf], entries: &[DesktopEntry]) -> Vec<App> { /* merge + dedup by pkg_ref */ }
```
Tests: an entry with `Exec=/home/u/Apps/Bar-2.0.AppImage %U` → an AppImage App named from the entry; `list` dedups a scanned file + its registered entry into one (registered name/icon wins, path preserved).
- [ ] **Step 2:** PASS. Commit `feat(appimage): registered entries + dedup list`.

---

## Phase C — Enumerate integration

### Task C1: wire AppImage into `enumerate_with`
**Files:** Modify `commands.rs`.
- [ ] **Step 1:** add AppImage scan roots helper (`~/Applications`, `~/.local/bin`, `/opt`, `~/Downloads`, `~/Desktop`) and call `appimage::list(&appimage_roots(), &entries)` as a 4th source inside `enumerate_with` (it already has the scanned `entries`; pass them). Merge into the aggregate (same `merge`/sort path). Resolve icons for AppImage entries that have a registered icon name.
- [ ] **Step 2:** extend the `enumerate_with` fixture test to include an AppImage fixture root and assert an AppImage app appears in the merged list. `cargo test` green. Commit `feat(commands): include AppImage source in enumerate`.

---

## Phase D — Launch + Uninstall

### Task D1: launch AppImage
**Files:** Modify `launch.rs`.
- [ ] **Step 1 (test first):** replace the Phase-A placeholder arm:
```rust
// in build_launch_command, when desktop_path is None and source is AppImage:
Source::AppImage => ("setsid", vec!["--fork".into(), pkg_ref.into()]),  // run the AppImage file detached
```
(When a `desktop_path` IS present the existing `gio launch <desktop_path>` branch already handles it.) Test: AppImage with no desktop_path → runs the file path as a single argv; with desktop_path → gio launch.
Note: the command runner that LAUNCHES already detaches (`process_group(0)`); ensure the file is `+x` first — add a `chmod +x` best-effort in the launch command path (set perms via `std::fs::set_permissions` before spawn) or document that AppImageLauncher-integrated files are already executable.
- [ ] **Step 2:** PASS + clippy. Commit `feat(launch): run AppImage files`.

### Task D2: uninstall (delete) AppImage + path guard
**Files:** Modify `uninstall.rs` + `commands.rs`.
- [ ] **Step 1 (test first):**
```rust
/// True if a path is a deletable AppImage: ends in .AppImage AND lives under $HOME or /opt.
pub fn is_removable_appimage_path(path: &str, home: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".appimage") && (path.starts_with(home) || path.starts_with("/opt/"))
}
```
Tests: `~/Applications/Foo.AppImage` (home) → true; `/opt/Bar.AppImage` → true; `/usr/bin/x` → false; `/home/u/notes.txt` → false.
- [ ] **Step 2:** in the uninstall command, branch on `Source::AppImage`: validate `is_removable_appimage_path`, then delete the file (`std::fs::remove_file` if under `$HOME`, else `pkexec rm -f <path>` for `/opt`) and remove the registered `.desktop` + hicolor icon if `desktop_path`/icon known. (Other sources keep the existing `perform_uninstall` exec path.) Map errors to `AppError`.
- [ ] **Step 3:** spy/fs test for the guard + that a non-AppImage path is refused. `cargo test` + clippy green. Commit `feat(uninstall): delete AppImage files (guarded)`.

### Task D3: updates arm
**Files:** Modify `updates.rs` (only if the Update plan landed).
- [ ] `build_update_command(Source::AppImage, _)` → return a clear `AppError`/no-op (AppImage self-update is out of scope); the frontend never shows an Update button for AppImage. Commit `chore(updates): AppImage has no update path`.

---

## Phase E — Frontend

### Task E1: source plumbing
**Files:** `src/lib/types.ts`, `src/lib/avatar.ts`.
- [ ] `Source = "apt"|"flatpak"|"snap"|"appimage"`. `sourceLabel("appimage")="AppImage"`; add a 4th badge color (e.g. a slate/indigo token) in `avatar.ts` + `theme.css` (`--appimage`, `--appimage-tint`). `npm run check`. Commit `feat(fe): AppImage source label + color`.

### Task E2: filter chip + count + confirm wording
**Files:** `src/lib/components/Header.svelte`, `AppDetail.svelte`.
- [ ] Add an "AppImage" chip to the source segmented control with its count (the source filter already generalizes over the union). In the uninstall ConfirmDialog, when `app.source==="appimage"`, the message says "This deletes the AppImage file" (and "frees ~{size}") instead of "remove package". Open/Update buttons behave per source (no Update for AppImage). `npm run check` + `npm run build`. Commit `feat(fe): AppImage filter chip + delete wording`.

### Task E3: tests
- [ ] `avatar.test.ts`: AppImage label/color. Component: source chip renders + uninstall wording for AppImage. `npm test` green. Commit `test(fe): AppImage source`.

---

## Phase F — Verify
- [ ] `cargo test`, `cargo clippy --all-targets`, `npm test`, `npm run check`, `npm run build` green; CI gate passes.
- [ ] Manual (human): drop a real `*.AppImage` into `~/Applications`, refresh → it appears as an AppImage app; Open runs it; Uninstall deletes it (confirm dialog says "delete the file"). Do not delete real files in CI.

---

## Self-Review
- Spec coverage: discovery scan+registered+dedup (B2,B3,C1) ✓; 4th source variant (A1) ✓; launch (D1) ✓; uninstall=delete w/ guard (D2) ✓; no self-update (D3) ✓; frontend badge/filter/wording (E1,E2) ✓; tests (B,D,E3,F) ✓.
- Placeholders: `appimages_from_desktop`/`list` bodies are described with their contract + tests rather than full code (clear interface; the engineer fills the small merge/dedup logic against the provided tests) — acceptable bite-sized tasks; everything else has complete code.
- Type consistency: `Source::AppImage`/`"appimage"` used identically across model/types/avatar; `App` literal in `scan_appimages` matches the current struct (drop `update_available` if the Update plan hasn't landed); `pkg_ref` = AppImage path used consistently as the uid/launch/delete key.
