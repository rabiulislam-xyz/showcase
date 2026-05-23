# Update Apps — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) tracking. This extends a mature codebase — **follow existing patterns** (the `CommandRunner` seam, `uninstall_app`'s async + `spawn_blocking` + guard structure, `classify_error`, `parseAppError`, the `Dropdown`/toast/store patterns). Spec: `docs/superpowers/specs/2026-05-24-showcase-updates-design.md`.

**Goal:** Detect available updates per source and apply them (per-app + "Update all") via each source's polkit-backed mechanism.

**Architecture:** A new pure `updates.rs` (parsers + command builders, fixture-tested through the existing `CommandRunner` seam) plus three Tauri commands (`check_updates`, `update_app`, `update_all`) mirroring `uninstall_app`. `App` gains `update_available: Option<String>`. Frontend adds a check button, update badges, and an Update button.

**Tech Stack:** Tauri v2 (Rust), SvelteKit (Svelte 5), Vitest, cargo test.

---

## File Structure
- `src-tauri/src/model.rs` — add `App.update_available`.
- `src-tauri/src/updates.rs` — **new**: `parse_apt_upgradable`, `parse_flatpak_updates`, `parse_snap_refresh_list`, `build_update_command`, `check_updates_with(runner)` (pure/injected).
- `src-tauri/src/commands.rs` — `check_updates`, `update_app`, `update_all` commands.
- `src-tauri/src/lib.rs` — register the 3 commands; `pub mod updates;`.
- Frontend: `src/lib/types.ts`, `src/lib/api.ts`, `src/lib/stores.ts`, `src/lib/components/{Header,AppCard,AppDetail}.svelte`, tests.

---

## Phase A — Model

### Task A1: `App.update_available`
**Files:** Modify `src-tauri/src/model.rs`; update every `App { … }` literal (`grep -rn "removable:" src-tauri/src`).
- [ ] **Step 1:** add `pub update_available: Option<String>,` to `App` (near `protected_reason`).
- [ ] **Step 2:** set it to `None` in every constructor + test helper (apt/flatpak/snapd sources and all `#[cfg(test)]` literals). It's only populated by the frontend after a check.
- [ ] **Step 3:** `cargo test` green (existing tests compile with the new field). Commit `feat(model): add App.update_available`.

---

## Phase B — Parsers + command builder (pure, TDD)

### Task B1: `parse_apt_upgradable`
**Files:** Create `src-tauri/src/updates.rs`; add `pub mod updates;` to `lib.rs`.
- [ ] **Step 1 (test first):**
```rust
/// Parse `apt list --upgradable` lines → (package, new_version).
/// Line shape: "pkg/suite 1.2.3 amd64 [upgradable from: 1.2.0]".
/// Skips the "Listing..." header and blank lines.
pub fn parse_apt_upgradable(output: &str) -> Vec<(String, String)> {
    output.lines().filter_map(|l| {
        let l = l.trim();
        if l.is_empty() || l.starts_with("Listing") { return None; }
        let (pkg_part, rest) = l.split_once('/')?;          // "pkg", "suite 1.2.3 amd64 [..]"
        let version = rest.split_whitespace().nth(1)?;       // suite, VERSION, arch...
        Some((pkg_part.trim().to_string(), version.to_string()))
    }).collect()
}
```
Test: `"Listing...\nfirefox/jammy-updates 125.0 amd64 [upgradable from: 124.0]\nvim/jammy 2:9.0 amd64 [upgradable from: 2:8.2]\n"` → `[("firefox","125.0"),("vim","2:9.0")]`; empty/"Listing..." only → `[]`.
- [ ] **Step 2:** `cargo test --manifest-path src-tauri/Cargo.toml parse_apt_upgradable` PASS. Commit `feat(updates): parse apt --upgradable`.

### Task B2: `parse_flatpak_updates`
**Files:** Modify `updates.rs`.
- [ ] **Step 1 (test first):**
```rust
/// Parse tab-separated `flatpak remote-ls --updates --app --columns=application,version`
/// → (app_id, version). Blank lines skipped.
pub fn parse_flatpak_updates(output: &str) -> Vec<(String, String)> {
    output.lines().filter_map(|l| {
        let mut p = l.split('\t');
        let app = p.next()?.trim();
        let ver = p.next().unwrap_or("").trim();
        (!app.is_empty()).then(|| (app.to_string(), ver.to_string()))
    }).collect()
}
```
Test: `"org.gimp.GIMP\t2.10.38\ncom.x.App\t1.1\n"` → `[("org.gimp.GIMP","2.10.38"),("com.x.App","1.1")]`; `""` → `[]`.
- [ ] **Step 2:** PASS. Commit `feat(updates): parse flatpak updates`.

### Task B3: `parse_snap_refresh_list`
**Files:** Modify `updates.rs`.
- [ ] **Step 1 (test first):** `snap refresh --list` columns: `Name Version Rev Publisher Notes` (header line), or `All snaps up to date.`
```rust
/// Parse `snap refresh --list` → (name, available_version). Skips the header row
/// and the "All snaps up to date." message.
pub fn parse_snap_refresh_list(output: &str) -> Vec<(String, String)> {
    output.lines().filter_map(|l| {
        let l = l.trim();
        if l.is_empty() || l.starts_with("Name") || l.starts_with("All snaps") { return None; }
        let mut p = l.split_whitespace();
        let name = p.next()?;
        let ver = p.next()?;
        Some((name.to_string(), ver.to_string()))
    }).collect()
}
```
Test: `"Name    Version  Rev  Publisher  Notes\nfirefox 126.0    1234 mozilla    -\n"` → `[("firefox","126.0")]`; `"All snaps up to date.\n"` → `[]`.
- [ ] **Step 2:** PASS. Commit `feat(updates): parse snap refresh --list`.

### Task B4: `build_update_command`
**Files:** Modify `updates.rs`.
- [ ] **Step 1 (test first):** mirror `uninstall::build_uninstall` (arg-arrays, no shell).
```rust
use crate::model::Source;
/// Command (program + args) that updates one package for a source. Arg-array only.
pub fn build_update_command(source: Source, pkg_ref: &str) -> (&'static str, Vec<String>) {
    match source {
        Source::Apt => ("pkexec", vec!["apt-get".into(), "-y".into(), "install".into(), "--only-upgrade".into(), pkg_ref.into()]),
        Source::Flatpak => ("flatpak", vec!["update".into(), "-y".into(), pkg_ref.into()]),
        Source::Snap => ("pkexec", vec!["snap".into(), "refresh".into(), pkg_ref.into()]),
    }
}
```
Tests: each source maps correctly; a `pkg_ref` with shell metacharacters stays a single argv element (injection-safety). (When `Source::AppImage` exists — see the AppImage plan — add an arm that returns an error/no-op; AppImage has no update.)
- [ ] **Step 2:** PASS + `cargo clippy --all-targets` clean. Commit `feat(updates): build per-source update command`.

### Task B5: `check_updates_with(runner)` (injected, tested)
**Files:** Modify `updates.rs`.
- [ ] **Step 1 (test first):** compose the parsers behind the `CommandRunner` seam so it's hermetically testable.
```rust
use crate::runner::CommandRunner;
use crate::model::{App, Source, AppError};

/// Run the per-source update checks and return (uid, available_version) pairs.
/// apt requires a privileged metadata refresh first (`pkexec apt-get update`).
/// Per-source failure is isolated (logged via the returned warnings vec).
pub fn check_updates_with(runner: &dyn CommandRunner) -> (Vec<(String, String)>, Vec<String>) {
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    // apt: refresh then list
    match runner.run("pkexec", &["apt-get", "update"]).and_then(|_| runner.run("apt", &["list", "--upgradable"])) {
        Ok(o) => out.extend(parse_apt_upgradable(&o).into_iter().map(|(p,v)| (App::make_uid(Source::Apt, &p), v))),
        Err(e) => warnings.push(format!("apt: {e}")),
    }
    match runner.run("flatpak", &["remote-ls", "--updates", "--app", "--columns=application,version"]) {
        Ok(o) => out.extend(parse_flatpak_updates(&o).into_iter().map(|(p,v)| (App::make_uid(Source::Flatpak, &p), v))),
        Err(e) => warnings.push(format!("flatpak: {e}")),
    }
    match runner.run("snap", &["refresh", "--list"]) {
        Ok(o) => out.extend(parse_snap_refresh_list(&o).into_iter().map(|(p,v)| (App::make_uid(Source::Snap, &p), v))),
        Err(e) => warnings.push(format!("snap: {e}")),
    }
    (out, warnings)
}
```
Test with `FakeRunner` (canned apt/flatpak/snap outputs) → asserts the merged `(uid, version)` list and that a failing source becomes a warning, others survive.
- [ ] **Step 2:** PASS. Commit `feat(updates): injectable check_updates_with`.

---

## Phase C — Commands

### Task C1: `check_updates` command
**Files:** Modify `commands.rs`, `lib.rs`.
- [ ] **Step 1:** add (async + off-thread, since apt refresh is slow/privileged):
```rust
#[tauri::command]
pub async fn check_updates() -> Result<Vec<(String, String)>, crate::model::AppError> {
    tauri::async_runtime::spawn_blocking(|| {
        let (pairs, _warnings) = crate::updates::check_updates_with(&crate::runner::SystemRunner);
        Ok(pairs)
    }).await.map_err(|e| crate::model::AppError::Backend(format!("join: {e}")))?
}
```
- [ ] **Step 2:** register `check_updates` in the handler. `cargo build`. Commit `feat(commands): check_updates`.

### Task C2: `update_app` command (mirror `uninstall_app`)
**Files:** Modify `commands.rs`, `lib.rs`.
- [ ] **Step 1:** add an async `update_app(uid)` that mirrors `uninstall_app`'s structure exactly — parse uid via `Source::parse`, `enumerate()` for the authoritative app set, `validate_uninstall`-style existence check (a renderer can't update an app that isn't installed), then in `spawn_blocking`: `build_update_command(source, &pkg)`, run via `SystemRunner`, map a `Backend` error through `crate::uninstall::classify_error` (polkit-cancel → `PermissionDenied`). (Reuse `validate_uninstall` for the existence check — rename to `validate_app_exists` if clearer, keeping the removable check only on the uninstall path.)
- [ ] **Step 2:** register. Add a `perform_update(source, pkg, runner)`-style testable helper + spy-runner test (unknown uid → NotFound + 0 runner calls; happy path → runner called with the right argv). `cargo test` green. Commit `feat(commands): update_app with guard`.

### Task C3: `update_all` command
**Files:** Modify `commands.rs`, `lib.rs`.
- [ ] **Step 1:** `update_all(uids: Vec<String>)` — group uids by source, then ONE batched privileged call per source: apt `pkexec apt-get -y install --only-upgrade <pkg…>`, `flatpak update -y <id…>`, `pkexec snap refresh <name…>`. Continue past a per-source failure; return a `Vec<(uid, Result-ish)>` or `{updated: Vec<String>, errors: Vec<String>}`. Validate each uid exists first. Off-thread.
- [ ] **Step 2:** register; unit-test the per-source batching/argv with a spy runner. `cargo test` + `cargo clippy --all-targets` clean. Commit `feat(commands): update_all batched per source`.

---

## Phase D — Frontend

### Task D1: types + api
**Files:** `src/lib/types.ts`, `src/lib/api.ts`.
- [ ] `App.update_available: string | null`. `checkUpdates()→Promise<[string,string][]>`, `updateApp(uid)`, `updateAll(uids)`. `npm run check`. Commit `feat(fe): update api + type`.

### Task D2: stores
**Files:** `src/lib/stores.ts`.
- [ ] After `checkUpdates()`, merge `update_available` into the matching `apps` entries (by uid). Add derived `updatableApps` + `updatesCount`. A `checking` busy flag. `removeApp`/toast patterns reused. `npm run check`. Commit `feat(fe): updates store state`.

### Task D3: Header — check + update all
**Files:** `src/lib/components/Header.svelte`.
- [ ] Add a **"Check for updates"** icon/text button → `checkUpdates()` (busy spinner). When `$updatesCount > 0`, show a **"Update all (N)"** button → `updateAll($updatableApps.map(a=>a.uid))` with confirm + toasts. Keep existing controls. `npm run check`. Commit `feat(fe): check + update-all in header`.

### Task D4: AppCard badge + AppDetail Update button
**Files:** `src/lib/components/AppCard.svelte`, `AppDetail.svelte`.
- [ ] AppCard: a small accent "update" dot/badge when `app.update_available`. AppDetail: an **Update** button (beside Open/Uninstall) shown when `update_available`, labeled with the new version; on click `updateApp(uid)` → success toast + clear the flag (or re-check), error via `parseAppError`. `npm run check` + `npm run build`. Commit `feat(fe): update badge + button`.

### Task D5: tests
**Files:** `src/lib/*.test.ts`, component tests.
- [ ] Store merge of update_available; AppDetail Update click calls `updateApp` + toasts (mock api); badge renders when flagged. `npm test` green. Commit `test(fe): update flow`.

---

## Phase E — Verify
- [ ] `cargo test`, `cargo clippy --all-targets`, `npm test`, `npm run check`, `npm run build` all green; CI gate passes.
- [ ] Manual (human): "Check for updates" prompts for the apt password, flags apps with updates; per-app + Update all work; cancel → "Authentication cancelled" toast. (Do not run real updates in CI.)

---

## Self-Review
- Spec coverage: detect (B1–B5, C1) ✓; per-app apply (C2) ✓; update-all (C3) ✓; on-demand check w/ apt refresh via polkit (C1/B5) ✓; badges/count/button (D2–D4) ✓; typed errors via classify_error (C2) ✓; tests (B,C,D5,E) ✓.
- Placeholders: none — parsers/builders have full code; commands reference the existing `uninstall_app` structure to mirror (DRY in a known codebase).
- Type consistency: `build_update_command`/`parse_*`/`check_updates_with` signatures match their call sites; `update_available` named identically across model/types/stores.
