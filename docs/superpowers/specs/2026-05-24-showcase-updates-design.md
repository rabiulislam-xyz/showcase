# Showcase — Update Apps — Design Spec

- **Date:** 2026-05-24
- **Status:** Approved design
- **Builds on:** shipped Showcase (apt/flatpak/snap browse + uninstall + launch)

## Goal
Let users see which installed apps have a newer version available and update them — per-app or all at once — using each source's native, polkit-backed mechanism. Same three sources: apt, Flatpak, Snap.

## Scope
**In:** on-demand update check (refreshes metadata + flags apps), per-app update, "Update all", update badges/count, typed errors + toasts.
**Out:** auto/scheduled checks, version pinning/downgrade, changelog display, AppImage updates (AppImage handled in its own spec; self-update unreliable).

## Data model
`App` gains `update_available: Option<String>` — the available newer version when known, else `None`. Serde: extra optional field (`null` in JSON).

## Backend (`src-tauri/src/updates.rs` — pure, injectable, tested)
Mirror the existing `CommandRunner`/`SnapSource` seam so logic is unit-tested with fixtures.

**Detect** (`check_updates` command, async + `spawn_blocking`):
- **apt** — `pkexec apt-get update` (refresh; the one privileged check) then `apt list --upgradable` → parse `pkg/… <newver> …` lines into `pkg → version`.
- **flatpak** — `flatpak remote-ls --updates --app --columns=application,version` → `app-id → version`.
- **snap** — `snap refresh --list` → `name → version` (parse columns; "All snaps up to date." → empty).
- Returns `Vec<(uid, available_version)>`; per-source failure isolated (warning, not fatal). The frontend merges these into the app list (`update_available`).

**Apply** (`update_app(uid)` — async + `spawn_blocking`, guards like `uninstall_app`: validate the app exists; reject unknown):
- apt → `pkexec apt-get -y install --only-upgrade <pkg>`
- flatpak → `flatpak update -y <app-id>` (user installs: no auth; system: flatpak's own polkit)
- snap → `pkexec snap refresh <name>`
`update_all()` batches the flagged packages **per source** (one privileged call/prompt each): apt `pkexec apt-get -y install --only-upgrade <pkg…>`, `flatpak update -y <id…>`, `pkexec snap refresh <name…>`.

`build_check_command` / `parse_*_updates` / `build_update_command` are pure functions with arg-array output (no shell — injection-safe), each unit-tested. Errors typed via `AppError` (`PermissionDenied`/`Cancelled` for polkit, `Backend` otherwise) reusing `classify_error`.

## Frontend
- Header **"Check for updates"** button → `checkUpdates()` → annotate apps, show **"Updates (N)"** chip + an **"Update all"** action when N>0.
- `AppCard`: a small "update available" badge when `update_available`.
- `AppDetail`: an **Update** button (beside Open/Uninstall) shown when `update_available`, with the new version; success → toast + clear the flag (or re-check); error → toast (PermissionDenied → "Authentication cancelled").
- Busy states during check/update; reuse `parseAppError`, toasts, `Dropdown` patterns.

## Testing
Rust: parsers for `apt list --upgradable`, `flatpak remote-ls --updates`, `snap refresh --list` (fixtures incl. "up to date"); `build_update_command` per source incl. injection-safety; `update_app` guard ordering via spy runner (no privileged exec on unknown uid). Frontend: store/check flow, AppDetail update button, badge.

## Errors / edge cases
- No network / apt refresh fails → that source's check warns, others still work.
- App updated externally between check and apply → apply is idempotent (no-op if already current).
- `update_all` continues past a per-source failure and reports per-source results.
