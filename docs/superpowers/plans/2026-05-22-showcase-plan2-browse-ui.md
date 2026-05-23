# Showcase — Plan 2: Browse UI — Implementation Plan

> **Historical record — implemented.**

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox tracking. Frontend polish should follow superpowers:frontend-design (distinctive, production-grade, Adwaita-aligned — not generic AI aesthetics).

**Goal:** Replace the stub list with a first-class, Adwaita-styled browsing UI: responsive app grid with icons, search, source filter, sort, an app-count header, a detail drawer with full metadata, and proper loading/empty/error+warning states.

**Architecture:** Backend `list_apps` returns `{apps, warnings}` (surfacing per-source failures); a new `get_app_details(uid)` lazily loads the long description. Icons render via Tauri's asset protocol (`convertFileSrc`). Frontend is a small typed layer (`types`/`api`/`stores`) with pure, unit-tested filter/sort logic, plus presentational Svelte 5 components themed with Adwaita CSS tokens (light/dark).

**Tech Stack:** Tauri v2 (Rust), SvelteKit + TypeScript (Svelte 5 runes), Vitest for logic tests.

**Builds on Plan 1 (merged to main).** Addresses Plan-2 carry-forwards: surface warnings; refine snap/flatpak icon matching.

---

## File Structure

Backend (`src-tauri/src/`):
- `model.rs` — add `AppList { apps, warnings }`.
- `commands.rs` — `list_apps` returns `AppList`; add `get_app_details(uid)`; improve icon matching for snap/flatpak.
- `details.rs` (new) — per-source long-description fetch.
- `tauri.conf.json` / capabilities — enable asset protocol for icon dirs.

Frontend (`src/`):
- `lib/types.ts` — `Source`, `App`, `AppList`.
- `lib/api.ts` — typed `invoke` wrappers + `iconSrc()`.
- `lib/filter.ts` — pure `filterAndSort(apps, query, source, sort)` (unit-tested).
- `lib/stores.ts` — Svelte stores: `apps`, `warnings`, `query`, `sourceFilter`, `sortKey`, `selected`, `status`; derived `visibleApps`.
- `lib/theme.css` — Adwaita tokens (light/dark).
- `lib/components/{AppCard,AppGrid,AppDetail,Header,StatusBanner,Spinner}.svelte`.
- `routes/+page.svelte` — compose header + grid + detail.
- `lib/filter.test.ts` — Vitest.

---

## Phase A — Backend additions

### Task A1: `AppList` and `list_apps` returns warnings

**Files:** Modify `src-tauri/src/model.rs`, `src-tauri/src/commands.rs`.

- [ ] **Step 1 (test, model.rs):** add struct + serde test.
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppList {
    pub apps: Vec<App>,
    pub warnings: Vec<String>,
}
```
Test: `serde_json::to_string` of an `AppList{apps:vec![],warnings:vec!["x".into()]}` contains `"warnings"`.

- [ ] **Step 2:** change `list_apps` in commands.rs:
```rust
#[tauri::command]
pub fn list_apps() -> crate::model::AppList {
    let agg = enumerate();
    crate::model::AppList { apps: agg.apps, warnings: agg.warnings }
}
```
- [ ] **Step 3:** `cargo test` green. Commit `feat: list_apps returns apps + warnings`.

### Task A2: Improve icon matching (carry-forward #2) + index by desktop-id

**Files:** Modify `src-tauri/src/commands.rs`.

Snap/flatpak `App.name` rarely equals a `.desktop` `Name=`. Match more robustly: when scanning entries, also remember each entry's icon keyed by BOTH its `Name=` and its source+pkg heuristics. Minimal, tested-by-integration approach:

- [ ] **Step 1:** Build a `Vec<(Source, /*key*/ String, /*icon*/ String)>` from entries where key = lowercased `Name`. In `resolve_icons`, for each app try, in order: exact name match, case-insensitive name match, and for flatpak the app-id last path segment, for snap the `pkg_ref`. First hit → resolve via the prebuilt index. Keep it O(apps + entries) using a `HashMap<(Source,String),String>`.
- [ ] **Step 2:** `cargo build`; run the gated integration test `-- --ignored` and confirm icon-resolved count rises vs Plan 1 (note the number). Commit `feat: more robust icon matching for snap/flatpak`.

### Task A3: `get_app_details(uid)` — lazy long description

**Files:** Create `src-tauri/src/details.rs`; modify `commands.rs`, `lib.rs`.

- [ ] **Step 1 (details.rs):** pure parsing helpers with tests:
```rust
/// Extract the long description from `apt-cache show <pkg>` output (the
/// indented lines under "Description[-xx]:"), de-indenting and dropping " ." lines.
pub fn parse_apt_description(show_output: &str) -> Option<String> { /* ... */ }
```
Test with a sample `apt-cache show` block.
- [ ] **Step 2 (commands.rs):** `get_app_details(uid)` parses `source:pkg_ref`, then per source:
  - apt → `apt-cache show <pkg>` via SystemRunner → `parse_apt_description`.
  - flatpak → `flatpak info <app-id>` → description line (or None).
  - snap → already have `summary`; fetch `/v2/snaps/<name>` `description` field if present.
  Returns `Option<String>` (the long description). Errors → None (non-fatal).
- [ ] **Step 3:** register `get_app_details` in the handler. `cargo test` green; commit `feat: get_app_details lazy long description`.

### Task A4: Enable asset protocol for icons

**Files:** Modify `src-tauri/tauri.conf.json` and `src-tauri/capabilities/default.json`.

- [ ] **Step 1:** In `tauri.conf.json` `app.security`, add:
```json
"assetProtocol": { "enable": true, "scope": [
  "/usr/share/icons/**", "/usr/share/pixmaps/**",
  "/var/lib/flatpak/exports/share/icons/**",
  "/var/lib/snapd/**",
  "$HOME/.local/share/icons/**", "$HOME/.local/share/flatpak/exports/share/icons/**"
] }
```
- [ ] **Step 2:** Ensure `capabilities/default.json` permits `core:default` (asset protocol is gated by the scope above; no extra permission needed in v2 for reading scoped assets via convertFileSrc). Verify `cargo build`.
- [ ] **Step 3:** Commit `feat: enable asset protocol for icon files`.

---

## Phase B — Frontend foundation

### Task B1: Types + API layer

**Files:** Create `src/lib/types.ts`, `src/lib/api.ts`.

- [ ] **Step 1 (types.ts):**
```ts
export type Source = "apt" | "flatpak" | "snap";
export interface App {
  uid: string; source: Source; name: string;
  summary: string | null; description: string | null; version: string | null;
  icon_path: string | null; size_bytes: number | null; install_date: string | null;
  publisher: string | null; categories: string[]; exec: string | null;
  pkg_ref: string; removable: boolean; protected_reason: string | null;
}
export interface AppList { apps: App[]; warnings: string[]; }
```
- [ ] **Step 2 (api.ts):**
```ts
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { App, AppList } from "./types";

export const listApps = () => invoke<AppList>("list_apps");
export const getAppDetails = (uid: string) => invoke<string | null>("get_app_details", { uid });
export const iconSrc = (app: App): string | null =>
  app.icon_path ? convertFileSrc(app.icon_path) : null;
```
- [ ] **Step 3:** `npm run check` passes. Commit `feat: frontend types + api layer`.

### Task B2: Pure filter/sort + Vitest

**Files:** Create `src/lib/filter.ts`, `src/lib/filter.test.ts`; add vitest.

- [ ] **Step 1:** add dev deps + script: `npm i -D vitest @testing-library/svelte jsdom` and `"test": "vitest run"` in package.json.
- [ ] **Step 2 (filter.ts):**
```ts
import type { App, Source } from "./types";
export type SortKey = "name" | "size" | "recent";
export function filterAndSort(
  apps: App[], query: string, source: Source | "all", sort: SortKey
): App[] {
  const q = query.trim().toLowerCase();
  let out = apps.filter(a =>
    (source === "all" || a.source === source) &&
    (q === "" || a.name.toLowerCase().includes(q) ||
     (a.summary?.toLowerCase().includes(q) ?? false)));
  const by = {
    name: (a: App, b: App) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
    size: (a: App, b: App) => (b.size_bytes ?? 0) - (a.size_bytes ?? 0),
    recent: (a: App, b: App) => (b.install_date ?? "").localeCompare(a.install_date ?? ""),
  }[sort];
  return [...out].sort(by);
}
```
- [ ] **Step 3 (filter.test.ts):** tests — filter by source, by query (name + summary), sort by name/size/recent. Sample apps inline.
- [ ] **Step 4:** `npm test` green. Commit `feat: pure filter/sort with vitest coverage`.

### Task B3: Stores

**Files:** Create `src/lib/stores.ts`.

- [ ] **Step 1:** writable stores `query`, `sourceFilter` (default "all"), `sortKey` (default "name"), `apps`, `warnings`, `selected` (App|null), `status` ("loading"|"ready"|"error"), `errorMsg`. A `derived` `visibleApps = filterAndSort(apps, query, sourceFilter, sortKey)`. An async `loadApps()` that sets loading, calls `listApps()`, fills apps+warnings, sets ready/error.
- [ ] **Step 2:** `npm run check`. Commit `feat: svelte stores for app browsing state`.

---

## Phase C — Components + Adwaita theming

> Follow superpowers:frontend-design. Adwaita reference: accent `#3584e4`, destructive `#e01b24`, light bg `#fafafb`/cards `#ffffff`, dark bg `#1e1e1e`/cards `#303030`, text `#2e3436`/dark `#deddda`, radius 12px cards / 8px controls, system font (Cantarell/Inter/system-ui). Subtle shadows, generous spacing, smooth 150ms transitions, focus-visible rings. Crisp icons (48px grid, image-rendering auto). Distinctive but native-feeling.

### Task C1: theme.css + global shell

**Files:** Create `src/lib/theme.css`; modify `src/routes/+layout.ts`/`+layout.svelte` (or `+page.svelte`) to import it.
- [ ] CSS custom properties for the tokens above under `:root` and `@media (prefers-color-scheme: dark)`. Reset, font, scrollbar styling. Commit `feat: Adwaita theme tokens (light/dark)`.

### Task C2: AppCard + AppGrid

**Files:** Create `src/lib/components/AppCard.svelte`, `AppGrid.svelte`.
- [ ] **AppCard:** icon (via `iconSrc`, fallback generic glyph), name (truncate 2 lines), source badge (colored pill: apt blue / flatpak teal / snap orange), version subtitle. Hover lift, click → `selected.set(app)`. Keyboard: focusable, Enter selects.
- [ ] **AppGrid:** responsive CSS grid (`repeat(auto-fill, minmax(160px,1fr))`), gap, renders `visibleApps` keyed by `uid`. Commit `feat: app card + responsive grid`.

### Task C3: Header (search/filter/sort/count/refresh)

**Files:** Create `src/lib/components/Header.svelte`.
- [ ] Sticky header: title, live search input (binds `query`), source filter segmented control (All/apt/flatpak/snap with counts), sort `<select>` (Name/Size/Recent), visible-count, refresh button (calls `loadApps`). Commit `feat: header with search, filter, sort, refresh`.

### Task C4: AppDetail drawer + StatusBanner + states

**Files:** Create `src/lib/components/AppDetail.svelte`, `StatusBanner.svelte`, `Spinner.svelte`.
- [ ] **AppDetail:** slide-in right drawer when `selected` set; shows large icon, name, source badge, version, size (human), install date, publisher, categories chips, exec; lazy-loads long description via `getAppDetails(uid)`; close on Esc/backdrop. Uninstall button present but DISABLED with title "coming in Plan 3".
- [ ] **StatusBanner:** if `warnings` non-empty, a dismissible amber banner listing them. Error state full-panel with retry. Loading → Spinner/skeleton. Empty (no visible apps) → friendly message.
- [ ] Commit `feat: detail drawer, status banner, loading/empty states`.

### Task C5: Compose +page.svelte
**Files:** Modify `src/routes/+page.svelte`.
- [ ] `onMount(loadApps)`; layout = Header + (Spinner | error | AppGrid) + AppDetail + StatusBanner + Toast region. Human size formatter util. Commit `feat: compose browse page`.

---

## Phase D — Verify

### Task D1: Build, typecheck, tests
- [ ] `npm run check` (0 errors), `npm test` (green), `cargo test` (green), `cargo clippy --all-targets` (clean).

### Task D2: GUI verification
- [ ] `npm run tauri dev`; controller captures a window screenshot (gnome-screenshot/import) and/or user eyeballs: grid renders with icons, search/filter/sort work, detail drawer opens with metadata + description, warnings banner if any. Fix issues. Final commit.

---

## Self-Review
- Spec coverage: browse visually (grid+icons C2), metadata detail (AppDetail C4 + get_app_details A3), search/filter/sort (filter.ts B2 + Header C3), warnings surfaced (A1 + StatusBanner C4), icon matching improved (A2), theming/states (C1/C4). Uninstall intentionally deferred to Plan 3 (button disabled).
- Asset protocol scope is the key technical risk → A4 + D2 confirm icons actually render in the webview.
- Carry-forwards from Plan 1 addressed: warnings (A1), icon matching (A2).
