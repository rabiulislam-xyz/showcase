# Showcase — Ubuntu App Manager — Design Spec

- **Date:** 2026-05-22
- **Status:** Approved (design); pending implementation plan
- **Target:** Ubuntu 22.04 LTS (jammy) and later, GNOME desktop

## 1. Overview

Showcase is a native-feeling desktop application for Ubuntu that lets a user
visually browse every installed **GUI application** across package sources
(apt/dpkg, Flatpak, Snap), inspect each app's metadata, and completely
uninstall apps with a single, guided action.

Installing or updating apps is **out of scope for v1** but the architecture
must leave room to add it later through the same source abstraction.

## 2. Scope

**In scope (v1)**
- Enumerate installed GUI apps from apt, Flatpak, and Snap.
- Show each app visually: icon, name, source, version.
- Search, filter by source, sort (name / size / recently installed).
- Detail view with full metadata.
- Complete uninstall with confirmation, privilege escalation, and progress.

**Out of scope (v1)**
- Installing or updating apps.
- AppImage (no central registry; revisit later).
- Browsing an app store / catalog.
- Editing per-app permissions (Flatpak/Snap interfaces).

## 3. Target environment & feasibility (verified on dev machine)

- Ubuntu 22.04.5, GNOME. `apt`, `dpkg`, `flatpak`, `snap` all present.
- **Rust** `cargo 1.93.1` already installed.
- **WebKitGTK 4.1** available in repos (`2.50.4-0ubuntu0.22.04.1`) → **Tauri v2** is viable.
- PackageKit present with the **aptcc** backend; its D-Bus system service is installed.
- `snapd` socket live at `/run/snapd.socket`.
- `.desktop` → package mapping works via `dpkg -S`; `dpkg-query` exposes version/size/essential.
- One-time build dependencies still required (see §13).

## 4. Tech stack

- **Shell:** Tauri v2 (Rust core, WebKitGTK 4.1 webview).
- **Frontend:** Svelte + TypeScript + Vite.
- **Styling:** Custom CSS aligned to GNOME/Adwaita (Cantarell font, Adwaita
  color tokens, light/dark via `prefers-color-scheme`).
- **Rust crates (intended):** `tauri` v2, `zbus` (PackageKit D-Bus), `serde`/`serde_json`,
  a unix-socket HTTP client for snapd (e.g. `hyperlocal`/`hyper`), a freedesktop
  desktop-entry parser, and an icon-theme lookup crate. Exact crate choices are
  finalized in the implementation plan.

## 5. Data model

```
App {
  uid:            String          // stable: "{source}:{pkg_ref}"
  source:         Apt | Flatpak | Snap
  name:           String          // .desktop Name=, fallback pkg name
  summary:        Option<String>  // short comment
  description:    Option<String>  // long, lazy-loaded in detail
  version:        Option<String>
  icon_path:      Option<PathBuf> // resolved absolute path
  size_bytes:     Option<u64>     // installed size
  install_date:   Option<DateTime>
  publisher:      Option<String>
  categories:     Vec<String>
  exec:           Option<String>
  pkg_ref:        String          // apt pkg name / flatpak app-id / snap name
  removable:      bool
  protected_reason: Option<String> // why uninstall is blocked, if so
}
```

## 6. Enumeration — hybrid (.desktop spine + per-manager enrichment)

The `.desktop` files are the source of truth for "what is an app" (exactly
what appears in the app menu) and give name + icon + comment uniformly.

1. Scan XDG application dirs:
   `/usr/share/applications`, `/usr/local/share/applications`,
   `~/.local/share/applications`, `/var/lib/snapd/desktop/applications`,
   `/var/lib/flatpak/exports/share/applications`,
   `~/.local/share/flatpak/exports/share/applications`.
2. Each entry with `Type=Application` and without `NoDisplay=true` / `Hidden=true`
   becomes one app. Capture Name, Comment, Icon, Exec, Categories.
3. Classify source by file location: under flatpak exports → Flatpak; under
   snapd desktop dir → Snap; otherwise → Apt.
4. Build a `path → package` reverse index **once** by reading
   `/var/lib/dpkg/info/*.list` (avoids spawning `dpkg -S` per file).
5. Enrich per source:
   - **Apt:** batched `dpkg-query -W` → version, installed-size, `Essential`.
     Install date approximated from the package `.list` file mtime.
   - **Flatpak:** machine-readable `flatpak list --app --columns=...`
     (or libflatpak) → version, size, origin, install date.
   - **Snap:** snapd REST `GET /v2/snaps` → version, installed-size,
     install-date, **type**. Keep `type=app`; drop `base/core/kernel/gadget/snapd`.
6. Sources are enumerated **concurrently**; a failure in one source is isolated
   (e.g. snapd not running → skip snaps with a warning, never crash).
7. The same real app installed from two sources (e.g. apt + flatpak) appears as
   two entries, each labeled by source.

**Why hybrid and not all-PackageKit:** Ubuntu's aptcc backend is apt-only;
Flatpak and Snap are not behind PackageKit here. So each source is handled by
its native mechanism.

## 7. Icon resolution

Resolve the `.desktop` `Icon=` value (a name or absolute path) to a concrete
file via freedesktop icon-theme lookup (current theme → hicolor fallback,
preferring scalable/large sizes). The resolved path is surfaced to the webview
through Tauri's asset protocol. A bundled generic icon is the fallback.

## 8. Uninstall (destructive + privileged — guarded)

- **Apt:** PackageKit over D-Bus (`zbus`): `Resolve` the package, then
  `RemovePackages`. polkit shows the auth dialog automatically. Refuse packages
  marked `Essential`; surface reverse-dependencies before proceeding.
- **Flatpak:** libflatpak / `flatpak uninstall`. User installs need no auth;
  system installs trigger polkit via Flatpak itself.
- **Snap:** snapd REST `POST /v2/snaps/{name}` with `{action:"remove"}`; polkit
  handles auth. Refuse `base/core/snapd` snaps.

Common rules:
- Always show a confirmation dialog naming exactly what will be removed and the
  disk space that will be freed.
- Progress is streamed to the frontend via Tauri events.
- Errors are typed and surfaced as toasts (including polkit cancellation).
- **No shell string interpolation.** Use typed D-Bus calls and argument-array
  process execution only — package refs never reach a shell. Injection-safe.

## 9. Architecture & modules

**Rust** (`src-tauri/src/`)
- `model.rs` — `App`, `Source`, `AppError`.
- `desktop.rs` — scan/parse/classify `.desktop`; apply NoDisplay/Hidden filter.
- `icons.rs` — icon name → resolved path.
- `dpkg.rs` — build reverse index; parse `dpkg-query` output.
- `snapd.rs` — unix-socket REST client for snapd.
- `sources/apt.rs`, `sources/flatpak.rs`, `sources/snap.rs` — each implements
  a common trait `AppSource { list(); enrich(); uninstall(); }`.
- `commands.rs` — Tauri commands: `list_apps`, `get_app_details(uid)`,
  `uninstall_app(uid)` (emits progress events).
- `lib.rs` — wiring; Tauri capabilities locked to the above commands; CSP on.

Each source is independently testable behind the `AppSource` trait, with
command output / fixture directories injectable so tests never touch the real
system.

**Frontend** (`src/`)
- App shell: header (search field, source-filter chips, sort control, refresh,
  app count) + responsive grid.
- `AppGrid` / `AppCard` (icon, name, source badge, version).
- `AppDetail` drawer: full metadata, freed-space estimate, Uninstall + confirm.
- `lib/api.ts` — typed wrappers over `invoke()` + event listeners.
- Stores: `apps`, `filter`, `busy`.
- Reusable `Toast` and `Confirm` components.

## 10. Data flow

1. `list_apps` → enumerate all sources concurrently → merged `Vec<App>` (icons
   as resolved paths). Results stream to the grid as each source resolves, for
   fast first paint.
2. Click an app → `get_app_details(uid)` lazily loads heavier metadata → drawer.
3. Uninstall → confirm dialog → `uninstall_app(uid)` → polkit → progress events
   → on success, remove the card and update totals.

## 11. Error handling

Every command returns `Result<T, AppError>`. `AppError` variants:
`SourceUnavailable`, `Protected`, `PermissionDenied`, `Cancelled`, `NotFound`,
`Backend(message)`. Per-source enumeration failures are isolated; protected /
essential packages are caught in a pre-flight check before any privileged call.

## 12. Security

- The app runs unprivileged; only the specific uninstall operation escalates,
  via polkit, per action.
- Validate every `pkg_ref` before use; never interpolate into a shell.
- Tauri capabilities restricted to the three commands; CSP enabled; no remote
  content loaded into the webview.
- Guard against removing essential/base components that could break the system.

## 13. Testing strategy

- **Rust unit tests with fixtures:** `.desktop` parsing, path-based source
  classification, icon resolution, `dpkg-query` parsing, snapd JSON parsing,
  PackageKit package-id formatting, and the protected-package guard.
- Sources sit behind the `AppSource` trait with injectable command output /
  fixture dirs, so logic is tested without mutating the system.
- **Uninstall:** exercise a dry-run / simulate path (`apt-get -s`, `pkcon`
  simulate) in tests; never perform a real removal in CI.
- **Frontend:** a few component tests (card render, filter behavior).

## 14. UX / polish

Adwaita-matched theming with light/dark, keyboard navigation, search-as-you-type,
source filtering, sorting (name / size / recently installed), explicit
loading / empty / error states, crisp icons, freed-space display, and a
responsive grid.

## 15. One-time setup (requires sudo)

```
apt install libwebkit2gtk-4.1-dev libgtk-3-dev \
            libayatana-appindicator3-dev librsvg2-dev
```

`cargo`, `build-essential`, and `libssl-dev` are already present. This step is
scripted in the implementation plan.

## 16. Resolved decisions

- **Name:** Showcase.
- **Frontend framework:** Svelte + TypeScript.
- **Sources:** apt + Flatpak + Snap.
- **App definition:** GUI apps only (those with desktop entries).
- **Privilege model:** PackageKit + polkit for apt; native polkit paths for
  Flatpak and Snap.
