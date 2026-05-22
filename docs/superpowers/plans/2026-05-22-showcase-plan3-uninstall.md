# Showcase — Plan 3: Uninstall — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. This plan touches a DESTRUCTIVE, privileged operation — guards, confirmation, and injection-safety are non-negotiable. Tests must NEVER perform a real removal (use injected fake command runners). Real end-to-end verification is done by the human authorizing (or cancelling) one polkit prompt.

**Goal:** Let the user completely uninstall an installed app from the detail drawer — with a confirmation dialog (space freed), graphical polkit authentication, protected-package guards, progress/busy feedback, and the card removed on success.

**Architecture:** Each source uninstalls via its native, polkit-backed tool, shelled through the existing `CommandRunner` seam (no shell strings → injection-safe): **apt → `pkcon remove`** (PackageKit, the spec's choice; pkcon raises the polkit dialog), **flatpak → `flatpak uninstall`**, **snap → `snap remove`**. A pure `build_uninstall` decides program+args and a `protected_reason` guard refuses essential/base packages. The Tauri `uninstall_app(uid)` command runs the removal off the UI thread and maps failures to typed errors (incl. polkit cancellation). The frontend confirms, shows busy, then removes the app from the store and toasts the result.

**Tech Stack:** Tauri v2 (Rust, `CommandRunner`), SvelteKit + Svelte 5.

**Why pkcon (not zbus PackageKit):** pkcon is the PackageKit CLI (already installed), triggers the same polkit auth, and keeps the logic shell-free and unit-testable via the runner seam — far less risk than hand-rolling async D-Bus transactions. Honors the spec's "PackageKit + polkit for apt".

---

## File Structure
Backend: `src-tauri/src/uninstall.rs` (new — pure command/guard logic + tests), `commands.rs` (async `uninstall_app`), `lib.rs` (register), `runner.rs` (add a small helper if needed for exit-code/stderr classification).
Frontend: `src/lib/api.ts` (`uninstallApp`), `src/lib/stores.ts` (`removeApp`, `toast`), `src/lib/components/{ConfirmDialog,Toast}.svelte`, `AppDetail.svelte` (enable Uninstall), `+page.svelte` (toast region).

---

## Phase A — Backend

### Task A1: pure `build_uninstall` + protected guard (TDD)
**Files:** create `src-tauri/src/uninstall.rs`; add `pub mod uninstall;` to `lib.rs`.

- [ ] **Step 1 (test first):** define and test:
```rust
use crate::model::Source;

/// The external command (program + args) that removes a package for a source.
/// Arg-array form only — never a shell string (injection-safe).
pub fn build_uninstall(source: Source, pkg_ref: &str) -> (&'static str, Vec<String>) {
    match source {
        // pkcon (PackageKit) raises the polkit dialog; -y auto-confirms PK's own prompt.
        Source::Apt => ("pkcon", vec!["-y".into(), "remove".into(), pkg_ref.into()]),
        Source::Flatpak => ("flatpak", vec!["uninstall".into(), "--app".into(), "-y".into(), pkg_ref.into()]),
        Source::Snap => ("snap", vec!["remove".into(), pkg_ref.into()]),
    }
}

/// Refuse removal of system-critical packages. Returns Some(reason) if protected.
pub fn protected_reason(source: Source, pkg_ref: &str) -> Option<String> {
    match source {
        Source::Snap if matches!(pkg_ref, "core"|"core18"|"core20"|"core22"|"core24"|"snapd"|"bare"|"snapd-desktop-integration") =>
            Some(format!("{pkg_ref} is a base/system snap")),
        _ => None,
    }
}
```
Tests: each source maps to the right program+args; pkg_ref is passed as a single arg verbatim (e.g. `"a; rm -rf"` stays one arg — injection-safe); `protected_reason` flags `core22`/`snapd` but not `firefox`.

- [ ] **Step 2:** run `cargo test --manifest-path src-tauri/Cargo.toml uninstall`; green. Commit `feat: pure uninstall command builder + protected guard`.

### Task A2: classify removal errors (TDD)
**Files:** modify `src-tauri/src/uninstall.rs`.

- [ ] **Step 1 (test first):**
```rust
use crate::model::AppError;
/// Map a failed removal's stderr/exit into a typed error.
pub fn classify_error(stderr: &str) -> AppError {
    let s = stderr.to_lowercase();
    if s.contains("not authorized") || s.contains("authentication") || s.contains("cancel") || s.contains("dismiss") {
        AppError::PermissionDenied("authentication was cancelled or denied".into())
    } else {
        AppError::Backend(stderr.trim().to_string())
    }
}
```
Tests: polkit cancel/"Not authorized" → PermissionDenied; other stderr → Backend.
- [ ] **Step 2:** green; commit `feat: classify uninstall errors (polkit cancel vs backend)`.

### Task A3: `uninstall_app` command
**Files:** modify `src-tauri/src/commands.rs`, `lib.rs`.

- [ ] **Step 1:** add an apt-essential re-check helper (defense in depth): for apt, query `dpkg-query -W -f='${Essential}' <pkg>` via SystemRunner; "yes" → protected.
- [ ] **Step 2:** add:
```rust
#[tauri::command]
pub async fn uninstall_app(uid: String) -> Result<(), crate::model::AppError> {
    use crate::model::{AppError, Source};
    let (src, pkg) = uid.split_once(':').ok_or_else(|| AppError::NotFound(uid.clone()))?;
    let source = match src { "apt"=>Source::Apt, "flatpak"=>Source::Flatpak, "snap"=>Source::Snap, _=>return Err(AppError::NotFound(uid.clone())) };
    let pkg = pkg.to_string();
    // Guards (run before any privileged call).
    if let Some(r) = crate::uninstall::protected_reason(source, &pkg) { return Err(AppError::Protected(r)); }
    // Blocking exec off the async thread.
    tauri::async_runtime::spawn_blocking(move || {
        let runner = crate::runner::SystemRunner;
        if source == Source::Apt {
            if crate::uninstall::apt_is_essential(&runner, &pkg) { return Err(AppError::Protected(format!("{pkg} is an essential package"))); }
        }
        let (prog, args) = crate::uninstall::build_uninstall(source, &pkg);
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        crate::runner::CommandRunner::run(&runner, prog, &argv)
            .map(|_| ())
            .map_err(|e| match e { AppError::Backend(msg) => crate::uninstall::classify_error(&msg), other => other })
    }).await.map_err(|e| AppError::Backend(format!("join: {e}")))?
}
```
Add `apt_is_essential(runner, pkg) -> bool` in uninstall.rs (parses dpkg-query "yes"); unit-test it with FakeRunner.
- [ ] **Step 3:** register `uninstall_app` in the handler. `cargo test` green, `cargo clippy --all-targets` clean. Commit `feat: uninstall_app command (pkcon/flatpak/snap, guarded, polkit)`.

---

## Phase B — Frontend

### Task B1: api + store helpers
**Files:** `src/lib/api.ts`, `src/lib/stores.ts`.
- [ ] `export const uninstallApp = (uid: string) => invoke<void>("uninstall_app", { uid });`
- [ ] stores: `removeApp(uid)` (filters `apps`), and a minimal toast store: `toasts` writable array + `pushToast(kind: "success"|"error", msg)` / auto-dismiss.
- [ ] `npm run check`. Commit `feat: uninstallApp api + store helpers (removeApp, toasts)`.

### Task B2: ConfirmDialog + Toast components
**Files:** `src/lib/components/ConfirmDialog.svelte`, `Toast.svelte`.
- [ ] **ConfirmDialog:** props `{ open, title, message, confirmLabel, destructive, busy }`, events/callbacks `onconfirm`/`oncancel`. Modal, role="dialog", Esc/backdrop cancel, focus the confirm button, destructive styling (red). Shows a spinner + disables buttons while `busy`.
- [ ] **Toast:** renders `toasts`, success (green) / error (red), dismissible, stacked bottom-right, ARIA live.
- [ ] `npm run check`. Commit `feat: confirm dialog + toast components`.

### Task B3: wire uninstall into AppDetail
**Files:** `src/lib/components/AppDetail.svelte`, `src/routes/+page.svelte`.
- [ ] Enable the Uninstall button only when `app.removable`; otherwise keep disabled with `title={app.protected_reason ?? "Protected"}`.
- [ ] Click → open ConfirmDialog: title "Uninstall {name}?", message includes the source and "Frees ~{humanSize(size_bytes)}" plus "You may be asked for your password." On confirm: set busy, call `uninstallApp(uid)`; on success → `removeApp(uid)`, close drawer, `pushToast("success", "{name} uninstalled")`; on error → keep drawer, `pushToast("error", message)` (PermissionDenied → "Authentication cancelled"). Always clear busy.
- [ ] Mount `<Toast/>` in `+page.svelte`.
- [ ] `npm run check`, `npm run build`. Commit `feat: uninstall flow in detail drawer with confirm + toast`.

---

## Phase C — Verify

### Task C1: automated
- [ ] `cargo test` (all green, incl. new uninstall unit tests), `cargo clippy --all-targets` (clean), `npm run check` (0 errors), `npm test` (green), `npm run build` (ok).

### Task C2: real-system verification (human-authorized — DESTRUCTIVE)
- [ ] Controller launches the app; opens an app's drawer; the human picks whether to (a) proceed to actually remove a throwaway app they choose, or (b) trigger uninstall and **Cancel** at the polkit prompt to confirm the flow + error handling without removing anything. The controller MUST NOT pick and remove a package on its own. Confirm: protected apps show the disabled button; confirm dialog shows freed space; cancel → "Authentication cancelled" toast, card stays; success (if chosen) → card disappears + success toast.

---

## Self-Review
- Destructive-op safety: protected guard (snap base/core + apt Essential re-check) before any privileged call; injection-safe arg arrays; confirmation dialog; typed polkit-cancel handling; tests never remove anything (FakeRunner); real removal only on explicit human authorization.
- Spec coverage: apt=PackageKit(pkcon)+polkit, flatpak/snap native+polkit, guards, confirm + freed space, progress (busy state), card removal, error toasts.
- Deferred (acceptable): live line-by-line progress streaming (busy spinner used instead); CSP tightening.
