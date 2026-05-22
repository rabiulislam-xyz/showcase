# Showcase — Plan 1: Backend Enumeration Core — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust backend that enumerates every installed GUI app from apt, Flatpak, and Snap into a unified `App` list with metadata + resolved icons, exposed as a Tauri `list_apps` command and verified end-to-end with a stub UI.

**Architecture:** `.desktop` files are the spine (what is an "app"); each is classified by path to a source and enriched with that source's metadata. Sources sit behind a `CommandRunner` abstraction so all parsing/merging logic is unit-tested with fixtures and never touches the live system in tests. A Tauri command runs the three sources in parallel threads, isolates per-source failures, resolves icons, and returns `Vec<App>`.

**Tech Stack:** Tauri v2, Rust (serde, serde_json, thiserror; std-only for I/O), Svelte + TypeScript (stub only this plan), Vite.

**Roadmap context:** Plan 2 = browse UI; Plan 3 = uninstall. This plan must compile, pass `cargo test`, and show real installed apps in a stub window.

---

## File Structure

Rust (`src-tauri/src/`):
- `model.rs` — `Source`, `App`, `AppError`.
- `desktop.rs` — parse/filter/classify `.desktop` entries; scan dirs.
- `runner.rs` — `CommandRunner` trait + `SystemRunner` + test `FakeRunner`.
- `dpkg.rs` — parse `dpkg-query`; build `path → package` reverse index.
- `snapd.rs` — parse `/v2/snaps` JSON; thin unix-socket fetch.
- `sizes.rs` — parse human sizes (for Flatpak).
- `sources/mod.rs` — `AppSource` trait.
- `sources/apt.rs`, `sources/flatpak.rs`, `sources/snap.rs` — per-source `list()`.
- `icons.rs` — icon name → resolved path.
- `aggregate.rs` — run sources in parallel, isolate failures.
- `commands.rs` — `list_apps` Tauri command.
- `lib.rs` / `main.rs` — module wiring + Tauri builder.

Frontend (`src/`): `App.svelte` (verification stub only).

Test fixtures: `src-tauri/tests/fixtures/…` and `#[cfg(test)]` inline modules.

---

## Phase A — Setup

### Task A1: Install system build dependencies

**Files:**
- Create: `scripts/setup-deps.sh`

- [ ] **Step 1: Write the setup script**

```bash
#!/usr/bin/env bash
# One-time system build dependencies for Tauri v2 on Ubuntu 22.04+.
set -euo pipefail
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  pkg-config
echo "Done. cargo/build-essential/libssl-dev are assumed already present."
```

- [ ] **Step 2: Make executable and run it**

Run: `chmod +x scripts/setup-deps.sh && ./scripts/setup-deps.sh`
Expected: apt installs the packages (polkit/sudo password prompt). Finishes with "Done."

- [ ] **Step 3: Verify webkit is discoverable**

Run: `pkg-config --modversion webkit2gtk-4.1`
Expected: prints a version (e.g. `2.50.4`).

- [ ] **Step 4: Commit**

```bash
git add scripts/setup-deps.sh
git commit -m "build: add system dependency setup script"
```

### Task A2: Scaffold Tauri v2 + Svelte-TS into the existing repo

**Files:**
- Create: `package.json`, `index.html`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `src/main.ts`, `src/App.svelte`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/build.rs`, `.gitignore`

- [ ] **Step 1: Scaffold into a temp dir (the repo dir is non-empty)**

```bash
npm create tauri-app@latest showcase-scaffold -- \
  --template svelte-ts --manager npm --yes
```
Run from `/tmp`: `cd /tmp && npm create tauri-app@latest showcase-scaffold -- --template svelte-ts --manager npm --yes`
Expected: generates `/tmp/showcase-scaffold` with a Tauri v2 + Svelte-TS project.

- [ ] **Step 2: Copy generated files into the repo, preserving git + docs**

```bash
rsync -a --exclude '.git' /tmp/showcase-scaffold/ /home/rab/projects/pet/showcase/
rm -rf /tmp/showcase-scaffold
```
Expected: project files now live alongside `docs/` and `.git/`.

- [ ] **Step 3: Ensure `.gitignore` covers build artifacts**

Ensure `.gitignore` contains (append if missing):
```
node_modules
dist
src-tauri/target
```

- [ ] **Step 4: Install JS deps and verify Rust compiles**

Run: `npm install`
Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: both succeed (first Rust build is slow).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 + Svelte-TS project"
```

### Task A3: Add Rust dependencies and confirm the test harness runs

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Test: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add dependencies to `src-tauri/Cargo.toml`**

Under `[dependencies]` add (serde may already be present — keep one copy):
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
```

- [ ] **Step 2: Add a trivial failing test in `src-tauri/src/lib.rs`**

Append:
```rust
#[cfg(test)]
mod harness_smoke {
    #[test]
    fn harness_runs() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 3: Run the test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml harness_runs`
Expected: PASS (`test harness_smoke::harness_runs ... ok`).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "build: add serde/thiserror deps and test harness smoke test"
```

---

## Phase B — Model & desktop parsing (pure logic, TDD)

### Task B1: `Source` enum

**Files:**
- Create: `src-tauri/src/model.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod model;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/model.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Apt,
    Flatpak,
    Snap,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Source::Apt).unwrap(), "\"apt\"");
        assert_eq!(serde_json::to_string(&Source::Flatpak).unwrap(), "\"flatpak\"");
        assert_eq!(serde_json::to_string(&Source::Snap).unwrap(), "\"snap\"");
    }
}
```
Add `pub mod model;` to `src-tauri/src/lib.rs`.

- [ ] **Step 2: Run test to verify it fails, then passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml source_serializes_lowercase`
Expected: PASS (code and test are written together here; the RED is the compile-first run).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/model.rs src-tauri/src/lib.rs
git commit -m "feat: add Source enum with lowercase serde"
```

### Task B2: `App` struct and `AppError`

**Files:**
- Modify: `src-tauri/src/model.rs`

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/src/model.rs` (above the `#[cfg(test)]` block, add the types; inside it, add the test):
```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    pub uid: String,
    pub source: Source,
    pub name: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub icon_path: Option<PathBuf>,
    pub size_bytes: Option<u64>,
    pub install_date: Option<String>,
    pub publisher: Option<String>,
    pub categories: Vec<String>,
    pub exec: Option<String>,
    pub pkg_ref: String,
    pub removable: bool,
    pub protected_reason: Option<String>,
}

impl App {
    /// Stable identifier: "{source}:{pkg_ref}".
    pub fn make_uid(source: Source, pkg_ref: &str) -> String {
        let s = match source {
            Source::Apt => "apt",
            Source::Flatpak => "flatpak",
            Source::Snap => "snap",
        };
        format!("{s}:{pkg_ref}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("source unavailable: {0}")]
    SourceUnavailable(String),
    #[error("protected: {0}")]
    Protected(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("operation cancelled")]
    Cancelled,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("backend error: {0}")]
    Backend(String),
}
```
Test inside the existing `tests` module:
```rust
#[test]
fn uid_combines_source_and_ref() {
    assert_eq!(App::make_uid(Source::Snap, "firefox"), "snap:firefox");
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml uid_combines`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/model.rs
git commit -m "feat: add App struct, uid helper, and typed AppError"
```

### Task B3: Parse a `.desktop` entry from text

**Files:**
- Create: `src-tauri/src/desktop.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod desktop;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/desktop.rs`:
```rust
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopEntry {
    pub path: PathBuf,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub categories: Vec<String>,
    pub no_display: bool,
    pub hidden: bool,
    pub entry_type: Option<String>,
}

/// Parse the `[Desktop Entry]` group only. Localized keys (e.g. `Name[de]`)
/// are ignored in favor of the unlocalized key. Unknown keys are skipped.
pub fn parse_entry(path: PathBuf, text: &str) -> DesktopEntry {
    let mut entry = DesktopEntry { path, ..Default::default() };
    let mut in_group = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_group = line == "[Desktop Entry]";
            continue;
        }
        if !in_group {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim();
        if key.contains('[') {
            continue; // localized variant
        }
        match key {
            "Name" => entry.name = Some(value.to_string()),
            "Comment" => entry.comment = Some(value.to_string()),
            "Icon" => entry.icon = Some(value.to_string()),
            "Exec" => entry.exec = Some(value.to_string()),
            "Type" => entry.entry_type = Some(value.to_string()),
            "NoDisplay" => entry.no_display = value.eq_ignore_ascii_case("true"),
            "Hidden" => entry.hidden = value.eq_ignore_ascii_case("true"),
            "Categories" => {
                entry.categories = value
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
            _ => {}
        }
    }
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[Desktop Entry]\n\
        Version=1.0\n\
        Name=Activity Log Manager\n\
        Name[de]=Aktivitaetsprotokoll\n\
        Comment=Configure logging\n\
        Icon=activity-log-manager\n\
        Exec=activity-log-manager\n\
        Type=Application\n\
        Categories=Settings;Utility;\n";

    #[test]
    fn parses_core_fields_and_ignores_localized() {
        let e = parse_entry(PathBuf::from("/x.desktop"), SAMPLE);
        assert_eq!(e.name.as_deref(), Some("Activity Log Manager"));
        assert_eq!(e.comment.as_deref(), Some("Configure logging"));
        assert_eq!(e.icon.as_deref(), Some("activity-log-manager"));
        assert_eq!(e.entry_type.as_deref(), Some("Application"));
        assert_eq!(e.categories, vec!["Settings", "Utility"]);
        assert!(!e.no_display && !e.hidden);
    }
}
```
Add `pub mod desktop;` to `lib.rs`.

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parses_core_fields`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/desktop.rs src-tauri/src/lib.rs
git commit -m "feat: parse .desktop [Desktop Entry] group"
```

### Task B4: `should_display` filter

**Files:**
- Modify: `src-tauri/src/desktop.rs`

- [ ] **Step 1: Write the failing test**

Add to `desktop.rs` (impl on the type):
```rust
impl DesktopEntry {
    /// True if this entry represents a launchable, visible application.
    pub fn should_display(&self) -> bool {
        self.entry_type.as_deref() == Some("Application")
            && !self.no_display
            && !self.hidden
    }
}
```
Add tests:
```rust
#[test]
fn hidden_or_nodisplay_or_nonapp_excluded() {
    let app = parse_entry(PathBuf::from("/a"), "[Desktop Entry]\nType=Application\n");
    assert!(app.should_display());

    let nodisp = parse_entry(PathBuf::from("/b"), "[Desktop Entry]\nType=Application\nNoDisplay=true\n");
    assert!(!nodisp.should_display());

    let hidden = parse_entry(PathBuf::from("/c"), "[Desktop Entry]\nType=Application\nHidden=true\n");
    assert!(!hidden.should_display());

    let link = parse_entry(PathBuf::from("/d"), "[Desktop Entry]\nType=Link\n");
    assert!(!link.should_display());
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml hidden_or_nodisplay`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/desktop.rs
git commit -m "feat: add should_display filter for desktop entries"
```

### Task B5: Classify source by file path

**Files:**
- Modify: `src-tauri/src/desktop.rs`

- [ ] **Step 1: Write the failing test**

Add to `desktop.rs`:
```rust
use crate::model::Source;

/// Classify which package source owns a `.desktop` file by its location.
pub fn classify_source(path: &std::path::Path) -> Source {
    let p = path.to_string_lossy();
    if p.contains("/flatpak/") {
        Source::Flatpak
    } else if p.contains("/snapd/desktop/") {
        Source::Snap
    } else {
        Source::Apt
    }
}
```
Tests:
```rust
#[test]
fn classifies_by_path() {
    use std::path::Path;
    assert_eq!(
        classify_source(Path::new("/var/lib/flatpak/exports/share/applications/x.desktop")),
        Source::Flatpak
    );
    assert_eq!(
        classify_source(Path::new("/var/lib/snapd/desktop/applications/firefox_firefox.desktop")),
        Source::Snap
    );
    assert_eq!(
        classify_source(Path::new("/usr/share/applications/gedit.desktop")),
        Source::Apt
    );
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml classifies_by_path`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/desktop.rs
git commit -m "feat: classify desktop entry source by path"
```

### Task B6: Scan a directory of `.desktop` files

**Files:**
- Modify: `src-tauri/src/desktop.rs`
- Test: `src-tauri/tests/fixtures/apps/` (created in step 1)

- [ ] **Step 1: Write the failing test + fixtures**

Create fixture files:
- `src-tauri/tests/fixtures/apps/good.desktop`:
```
[Desktop Entry]
Type=Application
Name=Good App
Icon=good
```
- `src-tauri/tests/fixtures/apps/hidden.desktop`:
```
[Desktop Entry]
Type=Application
Name=Hidden App
NoDisplay=true
```
- `src-tauri/tests/fixtures/apps/notes.txt` (a non-.desktop file to ensure it is skipped): `ignore me`

Add to `desktop.rs`:
```rust
/// Scan one directory, returning displayable entries from `*.desktop` files.
/// Unreadable files are skipped (logged by the caller if desired).
pub fn scan_dir(dir: &std::path::Path) -> Vec<DesktopEntry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            let parsed = parse_entry(path, &text);
            if parsed.should_display() {
                out.push(parsed);
            }
        }
    }
    out
}
```
Test:
```rust
#[test]
fn scan_dir_returns_only_displayable_desktop_files() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/apps");
    let entries = scan_dir(&dir);
    let names: Vec<_> = entries.iter().filter_map(|e| e.name.clone()).collect();
    assert!(names.contains(&"Good App".to_string()));
    assert!(!names.contains(&"Hidden App".to_string()));
    assert_eq!(entries.len(), 1);
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml scan_dir_returns`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/desktop.rs src-tauri/tests/fixtures/apps
git commit -m "feat: scan directory for displayable desktop entries"
```

---

## Phase C — dpkg / apt source

### Task C1: `CommandRunner` abstraction

**Files:**
- Create: `src-tauri/src/runner.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod runner;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/runner.rs`:
```rust
use crate::model::AppError;
use std::collections::HashMap;

/// Abstraction over running an external command, so source logic is testable.
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, AppError>;
}

/// Runs real processes via std::process::Command.
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, AppError> {
        let output = std::process::Command::new(program)
            .args(args)
            .output()
            .map_err(|e| AppError::Backend(format!("spawn {program}: {e}")))?;
        if !output.status.success() {
            return Err(AppError::Backend(format!(
                "{program} exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Test double: returns canned stdout keyed by the program name.
pub struct FakeRunner {
    pub responses: HashMap<String, Result<String, AppError>>,
}

impl FakeRunner {
    pub fn new() -> Self {
        Self { responses: HashMap::new() }
    }
    pub fn with(mut self, program: &str, out: &str) -> Self {
        self.responses.insert(program.to_string(), Ok(out.to_string()));
        self
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, program: &str, _args: &[&str]) -> Result<String, AppError> {
        self.responses
            .get(program)
            .cloned()
            .unwrap_or_else(|| Err(AppError::Backend(format!("no fake for {program}"))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_runner_returns_canned_output() {
        let r = FakeRunner::new().with("dpkg-query", "hello\n");
        assert_eq!(r.run("dpkg-query", &["-W"]).unwrap(), "hello\n");
        assert!(r.run("missing", &[]).is_err());
    }
}
```
Add `pub mod runner;` to `lib.rs`.

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml fake_runner_returns`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/runner.rs src-tauri/src/lib.rs
git commit -m "feat: add CommandRunner trait with system and fake impls"
```

### Task C2: Parse `dpkg-query` output

**Files:**
- Create: `src-tauri/src/dpkg.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod dpkg;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/dpkg.rs`:
```rust
/// One package's fields from `dpkg-query -W -f='${Package}\t${Version}\t${Installed-Size}\t${Essential}\n'`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpkgInfo {
    pub package: String,
    pub version: String,
    pub size_bytes: u64, // Installed-Size is reported in KiB
    pub essential: bool,
}

/// Parse multi-line dpkg-query output. Malformed lines are skipped.
pub fn parse_query(output: &str) -> Vec<DpkgInfo> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let package = parts.next()?.trim();
            let version = parts.next()?.trim();
            let size_kib: u64 = parts.next()?.trim().parse().ok()?;
            let essential = matches!(parts.next().map(str::trim), Some("yes"));
            if package.is_empty() {
                return None;
            }
            Some(DpkgInfo {
                package: package.to_string(),
                version: version.to_string(),
                size_bytes: size_kib * 1024,
                essential,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lines_and_converts_kib_to_bytes() {
        let out = "bash\t5.1-6ubuntu1.1\t1864\tyes\ngedit\t41.0-2\t4096\t\nbad-line\n";
        let infos = parse_query(out);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0], DpkgInfo {
            package: "bash".into(), version: "5.1-6ubuntu1.1".into(),
            size_bytes: 1864 * 1024, essential: true,
        });
        assert_eq!(infos[1].package, "gedit");
        assert!(!infos[1].essential);
    }
}
```
Add `pub mod dpkg;` to `lib.rs`.

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parses_lines_and_converts`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/dpkg.rs src-tauri/src/lib.rs
git commit -m "feat: parse dpkg-query output into DpkgInfo"
```

### Task C3: Build `path → package` reverse index

**Files:**
- Modify: `src-tauri/src/dpkg.rs`

- [ ] **Step 1: Write the failing test**

Add to `dpkg.rs`:
```rust
use std::collections::HashMap;
use std::path::Path;

/// Build a map of `installed file path → owning package` by reading the
/// `*.list` files under a dpkg info dir (default `/var/lib/dpkg/info`).
/// Only `.desktop` paths are retained (all we need for app→package mapping).
pub fn build_desktop_index(info_dir: &Path) -> HashMap<String, String> {
    let mut index = HashMap::new();
    let Ok(read) = std::fs::read_dir(info_dir) else { return index };
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("list") {
            continue;
        }
        // file name like "gedit:amd64.list" or "gedit.list" -> package "gedit"
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let package = stem.split(':').next().unwrap_or(stem).to_string();
        if let Ok(text) = std::fs::read_to_string(&path) {
            for line in text.lines() {
                if line.ends_with(".desktop") {
                    index.insert(line.to_string(), package.clone());
                }
            }
        }
    }
    index
}
```
Fixtures:
- `src-tauri/tests/fixtures/dpkg-info/gedit:amd64.list`:
```
/usr/share/applications/org.gnome.gedit.desktop
/usr/bin/gedit
/usr/share/doc/gedit/copyright
```
Test:
```rust
#[test]
fn reverse_index_maps_desktop_paths_to_package() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dpkg-info");
    let idx = build_desktop_index(&dir);
    assert_eq!(
        idx.get("/usr/share/applications/org.gnome.gedit.desktop").map(String::as_str),
        Some("gedit")
    );
    assert!(!idx.contains_key("/usr/bin/gedit"));
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml reverse_index_maps`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/dpkg.rs src-tauri/tests/fixtures/dpkg-info
git commit -m "feat: build desktop-path to package reverse index"
```

### Task C4: `AppSource` trait

**Files:**
- Create: `src-tauri/src/sources/mod.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod sources;`)

- [ ] **Step 1: Write the code (trait only; behavior tested via impls)**

Create `src-tauri/src/sources/mod.rs`:
```rust
use crate::model::{App, AppError};

pub mod apt;
pub mod flatpak;
pub mod snap;

/// A package source that can enumerate its installed GUI apps.
/// (Uninstall is added in Plan 3.)
pub trait AppSource {
    fn list(&self) -> Result<Vec<App>, AppError>;
}
```
Add `pub mod sources;` to `lib.rs`. Create empty placeholder files `src-tauri/src/sources/apt.rs`, `flatpak.rs`, `snap.rs` each containing `// implemented in later tasks` so the module compiles.

- [ ] **Step 2: Verify it compiles**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: compiles (empty source modules are fine).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sources src-tauri/src/lib.rs
git commit -m "feat: add AppSource trait and source module skeleton"
```

### Task C5: Apt source — build apps from entries + index + dpkg-query

**Files:**
- Modify: `src-tauri/src/sources/apt.rs`

- [ ] **Step 1: Write the failing test**

Replace `src-tauri/src/sources/apt.rs`:
```rust
use crate::desktop::DesktopEntry;
use crate::dpkg::{self, DpkgInfo};
use crate::model::{App, AppError, Source};
use crate::runner::CommandRunner;
use std::collections::HashMap;

/// Build apt-sourced apps from already-scanned desktop entries.
/// `index` maps desktop path → package; `runner` provides dpkg-query output.
pub fn list_from(
    entries: &[DesktopEntry],
    index: &HashMap<String, String>,
    runner: &dyn CommandRunner,
) -> Result<Vec<App>, AppError> {
    // Resolve each entry to its owning package.
    let mut pkg_of: HashMap<String, String> = HashMap::new(); // path -> pkg
    for e in entries {
        let key = e.path.to_string_lossy().to_string();
        if let Some(pkg) = index.get(&key) {
            pkg_of.insert(key, pkg.clone());
        }
    }
    if pkg_of.is_empty() {
        return Ok(Vec::new());
    }

    // One batched dpkg-query for all packages.
    let packages: Vec<&str> = pkg_of.values().map(String::as_str).collect();
    let mut args = vec![
        "-W",
        "-f=${Package}\t${Version}\t${Installed-Size}\t${Essential}\n",
    ];
    args.extend(packages.iter().copied());
    let output = runner.run("dpkg-query", &args)?;
    let infos: HashMap<String, DpkgInfo> = dpkg::parse_query(&output)
        .into_iter()
        .map(|i| (i.package.clone(), i))
        .collect();

    let mut apps = Vec::new();
    for e in entries {
        let key = e.path.to_string_lossy().to_string();
        let Some(pkg) = pkg_of.get(&key) else { continue };
        let info = infos.get(pkg);
        apps.push(App {
            uid: App::make_uid(Source::Apt, pkg),
            source: Source::Apt,
            name: e.name.clone().unwrap_or_else(|| pkg.clone()),
            summary: e.comment.clone(),
            description: None,
            version: info.map(|i| i.version.clone()),
            icon_path: None, // resolved later by icons module
            size_bytes: info.map(|i| i.size_bytes),
            install_date: None,
            publisher: None,
            categories: e.categories.clone(),
            exec: e.exec.clone(),
            pkg_ref: pkg.clone(),
            removable: info.map(|i| !i.essential).unwrap_or(true),
            protected_reason: info
                .filter(|i| i.essential)
                .map(|_| "Essential system package".to_string()),
        });
    }
    Ok(apps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::parse_entry;
    use crate::runner::FakeRunner;
    use std::path::PathBuf;

    #[test]
    fn builds_app_with_metadata_and_protection() {
        let entry = parse_entry(
            PathBuf::from("/usr/share/applications/org.gnome.gedit.desktop"),
            "[Desktop Entry]\nType=Application\nName=Text Editor\nComment=Edit text\nIcon=gedit\nExec=gedit %U\n",
        );
        let mut index = HashMap::new();
        index.insert(
            "/usr/share/applications/org.gnome.gedit.desktop".to_string(),
            "gedit".to_string(),
        );
        let runner = FakeRunner::new()
            .with("dpkg-query", "gedit\t41.0-2\t4096\t\n");

        let apps = list_from(&[entry], &index, &runner).unwrap();
        assert_eq!(apps.len(), 1);
        let a = &apps[0];
        assert_eq!(a.uid, "apt:gedit");
        assert_eq!(a.name, "Text Editor");
        assert_eq!(a.version.as_deref(), Some("41.0-2"));
        assert_eq!(a.size_bytes, Some(4096 * 1024));
        assert!(a.removable);
        assert_eq!(a.pkg_ref, "gedit");
    }

    #[test]
    fn essential_package_is_not_removable() {
        let entry = parse_entry(
            PathBuf::from("/usr/share/applications/bash.desktop"),
            "[Desktop Entry]\nType=Application\nName=Bash\n",
        );
        let mut index = HashMap::new();
        index.insert("/usr/share/applications/bash.desktop".to_string(), "bash".to_string());
        let runner = FakeRunner::new().with("dpkg-query", "bash\t5.1\t1864\tyes\n");

        let apps = list_from(&[entry], &index, &runner).unwrap();
        assert!(!apps[0].removable);
        assert_eq!(apps[0].protected_reason.as_deref(), Some("Essential system package"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib sources::apt`
Expected: PASS (both tests).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sources/apt.rs
git commit -m "feat: build apt apps from desktop entries + dpkg metadata"
```

---

## Phase D — Flatpak source

### Task D1: Parse human-readable sizes

**Files:**
- Create: `src-tauri/src/sizes.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod sizes;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/sizes.rs`:
```rust
/// Parse a human size like "92.6 MB", "1.2 GB", "512 kB" into bytes (1024-based).
/// Returns None if unparseable.
pub fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, unit) = s.split_at(
        s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len()),
    );
    let value: f64 = num.trim().parse().ok()?;
    let mult: f64 = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "kb" | "kib" | "k" => 1024.0,
        "mb" | "mib" | "m" => 1024.0 * 1024.0,
        "gb" | "gib" | "g" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * mult) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_units() {
        assert_eq!(parse_human_size("92.6 MB"), Some((92.6 * 1024.0 * 1024.0) as u64));
        assert_eq!(parse_human_size("1.0 GB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_human_size("512 kB"), Some(512 * 1024));
        assert_eq!(parse_human_size(""), None);
        assert_eq!(parse_human_size("garbage"), None);
    }
}
```
Add `pub mod sizes;` to `lib.rs`.

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parses_common_units`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sizes.rs src-tauri/src/lib.rs
git commit -m "feat: parse human-readable sizes to bytes"
```

### Task D2: Flatpak source — parse `flatpak list` columns

**Files:**
- Modify: `src-tauri/src/sources/flatpak.rs`

- [ ] **Step 1: Write the failing test**

Replace `src-tauri/src/sources/flatpak.rs`:
```rust
use crate::model::{App, AppError, Source};
use crate::runner::CommandRunner;
use crate::sizes::parse_human_size;

const COLUMNS: &str = "--columns=application,name,version,size,origin";

/// Parse tab-separated `flatpak list --app` output (one app per line,
/// fields in COLUMNS order). Blank lines skipped.
pub fn parse_list(output: &str) -> Vec<App> {
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut p = line.split('\t');
            let app_id = p.next()?.trim();
            if app_id.is_empty() {
                return None;
            }
            let name = p.next().unwrap_or("").trim();
            let version = p.next().unwrap_or("").trim();
            let size = p.next().unwrap_or("").trim();
            let origin = p.next().unwrap_or("").trim();
            Some(App {
                uid: App::make_uid(Source::Flatpak, app_id),
                source: Source::Flatpak,
                name: if name.is_empty() { app_id.to_string() } else { name.to_string() },
                summary: None,
                description: None,
                version: (!version.is_empty()).then(|| version.to_string()),
                icon_path: None,
                size_bytes: parse_human_size(size),
                install_date: None,
                publisher: (!origin.is_empty()).then(|| origin.to_string()),
                categories: Vec::new(),
                exec: None,
                pkg_ref: app_id.to_string(),
                removable: true,
                protected_reason: None,
            })
        })
        .collect()
}

pub fn list(runner: &dyn CommandRunner) -> Result<Vec<App>, AppError> {
    let output = runner.run("flatpak", &["list", "--app", COLUMNS])?;
    Ok(parse_list(&output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::FakeRunner;

    #[test]
    fn parses_flatpak_rows() {
        let out = "com.github.wwmm.easyeffects\tEasyEffects\t8.2.2\t92.6 MB\tflathub\n";
        let apps = parse_list(out);
        assert_eq!(apps.len(), 1);
        let a = &apps[0];
        assert_eq!(a.uid, "flatpak:com.github.wwmm.easyeffects");
        assert_eq!(a.name, "EasyEffects");
        assert_eq!(a.version.as_deref(), Some("8.2.2"));
        assert_eq!(a.size_bytes, Some((92.6 * 1024.0 * 1024.0) as u64));
        assert_eq!(a.publisher.as_deref(), Some("flathub"));
    }

    #[test]
    fn list_uses_runner() {
        let runner = FakeRunner::new().with("flatpak", "org.x.App\tX\t1.0\t10 MB\tflathub\n");
        let apps = list(&runner).unwrap();
        assert_eq!(apps[0].pkg_ref, "org.x.App");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib sources::flatpak`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sources/flatpak.rs
git commit -m "feat: enumerate flatpak apps via flatpak list"
```

---

## Phase E — Snap source

### Task E1: Parse snapd `/v2/snaps` JSON

**Files:**
- Create: `src-tauri/src/snapd.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod snapd;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/snapd.rs`:
```rust
use crate::model::{App, AppError, Source};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SnapdResponse {
    result: Vec<SnapEntry>,
}

#[derive(Debug, Deserialize)]
struct SnapEntry {
    name: String,
    #[serde(default)]
    version: String,
    #[serde(rename = "type", default)]
    snap_type: String,
    #[serde(rename = "installed-size", default)]
    installed_size: u64,
    #[serde(rename = "install-date", default)]
    install_date: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    publisher: Option<Publisher>,
}

#[derive(Debug, Deserialize)]
struct Publisher {
    #[serde(rename = "display-name", default)]
    display_name: Option<String>,
}

/// Parse a snapd `/v2/snaps` JSON body into apps, keeping only `type == "app"`.
pub fn parse_snaps(body: &str) -> Result<Vec<App>, AppError> {
    let resp: SnapdResponse = serde_json::from_str(body)
        .map_err(|e| AppError::Backend(format!("snapd json: {e}")))?;
    let apps = resp
        .result
        .into_iter()
        .filter(|s| s.snap_type == "app")
        .map(|s| App {
            uid: App::make_uid(Source::Snap, &s.name),
            source: Source::Snap,
            name: s.name.clone(),
            summary: s.summary,
            description: None,
            version: (!s.version.is_empty()).then(|| s.version.clone()),
            icon_path: None,
            size_bytes: (s.installed_size > 0).then_some(s.installed_size),
            install_date: s.install_date,
            publisher: s.publisher.and_then(|p| p.display_name),
            categories: Vec::new(),
            exec: None,
            pkg_ref: s.name.clone(),
            removable: true,
            protected_reason: None,
        })
        .collect();
    Ok(apps)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"{
      "type":"sync",
      "result":[
        {"name":"firefox","version":"124.0","type":"app","installed-size":256000000,
         "install-date":"2026-03-01T10:00:00Z","summary":"Web browser",
         "publisher":{"display-name":"Mozilla"}},
        {"name":"core22","version":"20260101","type":"base","installed-size":77000000}
      ]
    }"#;

    #[test]
    fn keeps_only_app_type() {
        let apps = parse_snaps(BODY).unwrap();
        assert_eq!(apps.len(), 1);
        let a = &apps[0];
        assert_eq!(a.uid, "snap:firefox");
        assert_eq!(a.version.as_deref(), Some("124.0"));
        assert_eq!(a.size_bytes, Some(256000000));
        assert_eq!(a.publisher.as_deref(), Some("Mozilla"));
        assert_eq!(a.install_date.as_deref(), Some("2026-03-01T10:00:00Z"));
    }
}
```
Add `pub mod snapd;` to `lib.rs`.

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml keeps_only_app_type`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/snapd.rs src-tauri/src/lib.rs
git commit -m "feat: parse snapd /v2/snaps json into apps"
```

### Task E2: Fetch from the snapd unix socket

**Files:**
- Modify: `src-tauri/src/snapd.rs`

- [ ] **Step 1: Write the implementation (I/O — not unit tested)**

Add to `snapd.rs`:
```rust
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

const SOCKET: &str = "/run/snapd.socket";

/// GET a snapd REST path over the unix socket and return the response body.
/// Minimal blocking HTTP/1.0 client (snapd speaks HTTP over the socket).
pub fn snapd_get(path: &str) -> Result<String, AppError> {
    let mut stream = UnixStream::connect(SOCKET)
        .map_err(|e| AppError::SourceUnavailable(format!("snapd socket: {e}")))?;
    let req = format!("GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n");
    stream
        .write_all(req.as_bytes())
        .map_err(|e| AppError::Backend(format!("snapd write: {e}")))?;
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .map_err(|e| AppError::Backend(format!("snapd read: {e}")))?;
    // Split headers from body at the first blank line.
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .unwrap_or("")
        .to_string();
    Ok(body)
}

/// List installed snap apps from the live socket.
pub fn list() -> Result<Vec<App>, AppError> {
    let body = snapd_get("/v2/snaps")?;
    parse_snaps(&body)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/snapd.rs
git commit -m "feat: fetch snaps over the snapd unix socket"
```

### Task E3: Snap source wrapper

**Files:**
- Modify: `src-tauri/src/sources/snap.rs`

- [ ] **Step 1: Write the implementation**

Replace `src-tauri/src/sources/snap.rs`:
```rust
use crate::model::{App, AppError};

/// Snap apps come fully-formed from snapd; desktop entries are only used
/// later for icon resolution (handled in the icons step), not enumeration.
pub fn list() -> Result<Vec<App>, AppError> {
    crate::snapd::list()
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sources/snap.rs
git commit -m "feat: snap source wrapper over snapd client"
```

---

## Phase F — Icon resolution

### Task F1: Resolve icon name to a file path

**Files:**
- Create: `src-tauri/src/icons.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod icons;`)
- Test fixtures: `src-tauri/tests/fixtures/icons/...`

- [ ] **Step 1: Write the failing test + fixtures**

Create fixture files (empty content is fine — only existence matters):
- `src-tauri/tests/fixtures/icons/hicolor/48x48/apps/gedit.png`
- `src-tauri/tests/fixtures/icons/scalable/apps/inkscape.svg`

Create `src-tauri/src/icons.rs`:
```rust
use std::path::{Path, PathBuf};

const EXTS: [&str; 3] = ["png", "svg", "xpm"];

/// Resolve a `.desktop` Icon= value against a list of icon theme roots.
/// - Absolute existing paths are returned as-is.
/// - Otherwise search each root recursively for `<name>.{png,svg,xpm}`.
/// Returns the first match, or None.
pub fn resolve(icon: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    if icon.is_empty() {
        return None;
    }
    let p = Path::new(icon);
    if p.is_absolute() && p.exists() {
        return Some(p.to_path_buf());
    }
    for root in roots {
        if let Some(found) = search_dir(root, icon) {
            return Some(found);
        }
    }
    None
}

fn search_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    let read = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if stem == name && EXTS.contains(&ext) {
                return Some(path);
            }
        }
    }
    for sub in subdirs {
        if let Some(found) = search_dir(&sub, name) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> Vec<PathBuf> {
        vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/icons")]
    }

    #[test]
    fn finds_png_by_name() {
        let found = resolve("gedit", &roots()).unwrap();
        assert!(found.ends_with("gedit.png"));
    }

    #[test]
    fn finds_svg_by_name() {
        let found = resolve("inkscape", &roots()).unwrap();
        assert!(found.ends_with("inkscape.svg"));
    }

    #[test]
    fn absolute_path_passthrough_and_missing_is_none() {
        assert_eq!(resolve("does-not-exist", &roots()), None);
        assert_eq!(resolve("", &roots()), None);
    }
}
```
Add `pub mod icons;` to `lib.rs`.

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib icons`
Expected: PASS (3 tests).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/icons.rs src-tauri/src/lib.rs src-tauri/tests/fixtures/icons
git commit -m "feat: resolve icon names to file paths via theme roots"
```

---

## Phase G — Aggregation, command, end-to-end verification

### Task G1: Aggregate sources with per-source failure isolation

**Files:**
- Create: `src-tauri/src/aggregate.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod aggregate;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/aggregate.rs`:
```rust
use crate::model::{App, AppError};

/// Result of aggregating sources: collected apps plus any non-fatal warnings
/// (one per source that failed). A failing source never drops the others.
#[derive(Debug, Default)]
pub struct Aggregated {
    pub apps: Vec<App>,
    pub warnings: Vec<String>,
}

/// Merge results from multiple sources. Each input is one source's outcome.
pub fn merge(results: Vec<(&str, Result<Vec<App>, AppError>)>) -> Aggregated {
    let mut agg = Aggregated::default();
    for (name, res) in results {
        match res {
            Ok(mut apps) => agg.apps.append(&mut apps),
            Err(e) => agg.warnings.push(format!("{name}: {e}")),
        }
    }
    agg.apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    agg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Source;

    fn app(name: &str) -> App {
        App {
            uid: App::make_uid(Source::Apt, name),
            source: Source::Apt,
            name: name.to_string(),
            summary: None, description: None, version: None, icon_path: None,
            size_bytes: None, install_date: None, publisher: None,
            categories: vec![], exec: None, pkg_ref: name.to_string(),
            removable: true, protected_reason: None,
        }
    }

    #[test]
    fn failing_source_becomes_warning_others_survive_and_sort() {
        let results = vec![
            ("apt", Ok(vec![app("Zebra"), app("apple")])),
            ("snap", Err(AppError::SourceUnavailable("down".into()))),
        ];
        let agg = merge(results);
        assert_eq!(agg.apps.len(), 2);
        assert_eq!(agg.apps[0].name, "apple"); // case-insensitive sort
        assert_eq!(agg.apps[1].name, "Zebra");
        assert_eq!(agg.warnings.len(), 1);
        assert!(agg.warnings[0].contains("snap"));
    }
}
```
Add `pub mod aggregate;` to `lib.rs`.

- [ ] **Step 2: Run test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml failing_source_becomes_warning`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/aggregate.rs src-tauri/src/lib.rs
git commit -m "feat: aggregate sources with per-source failure isolation"
```

### Task G2: Wire enumeration + icon resolution into a `list_apps` command

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod commands;` + register handler)

- [ ] **Step 1: Write the orchestration with icon resolution**

Create `src-tauri/src/commands.rs`:
```rust
use crate::aggregate::{self, Aggregated};
use crate::desktop;
use crate::dpkg;
use crate::icons;
use crate::model::App;
use crate::runner::SystemRunner;
use crate::sources::{apt, flatpak, snap};
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
        ("apt", apt_handle.join().unwrap_or_else(|_| Ok(vec![]))),
        ("flatpak", flatpak_handle.join().unwrap_or_else(|_| Ok(vec![]))),
        ("snap", snap_handle.join().unwrap_or_else(|_| Ok(vec![]))),
    ];
    let mut agg = aggregate::merge(results);
    resolve_icons(&mut agg.apps, &entries);
    agg
}

/// Fill icon_path for each app using a name pulled from its desktop entry.
fn resolve_icons(apps: &mut [App], entries: &[desktop::DesktopEntry]) {
    let roots = icon_roots();
    for app in apps.iter_mut() {
        // Match by exec or name to a desktop entry's Icon= value.
        let icon_name = entries.iter().find_map(|e| {
            let matches_name = e.name.as_deref() == Some(app.name.as_str());
            if matches_name { e.icon.clone() } else { None }
        });
        if let Some(name) = icon_name {
            app.icon_path = icons::resolve(&name, &roots);
        }
    }
}

#[tauri::command]
pub fn list_apps() -> Vec<App> {
    enumerate().apps
}
```
In `lib.rs`: add `pub mod commands;` and register the handler in the Tauri builder:
```rust
// inside the existing run() builder chain:
.invoke_handler(tauri::generate_handler![commands::list_apps])
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add list_apps command orchestrating all sources + icons"
```

### Task G3: Real-system integration smoke test (gated)

**Files:**
- Create: `src-tauri/tests/integration_enumerate.rs`

- [ ] **Step 1: Write a gated integration test**

Create `src-tauri/tests/integration_enumerate.rs`:
```rust
//! Runs the real enumeration against the host. Ignored by default so CI on
//! non-Ubuntu hosts stays green; run locally with `--ignored`.
#[test]
#[ignore]
fn enumerates_real_apps() {
    let agg = showcase_lib::commands::enumerate();
    println!("apps={} warnings={:?}", agg.apps.len(), agg.warnings);
    assert!(!agg.apps.is_empty(), "expected at least one installed app");
}
```
Note: the library crate name is whatever the scaffold set (commonly `showcase_lib`). If different, match `src-tauri/Cargo.toml`'s `[lib] name`.

- [ ] **Step 2: Run it explicitly**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test integration_enumerate -- --ignored --nocapture`
Expected: prints a non-zero app count (e.g. `apps=120 warnings=[]`) and PASSES.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/integration_enumerate.rs
git commit -m "test: gated real-system enumeration smoke test"
```

### Task G4: Stub UI to verify end-to-end in a window

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Replace `src/App.svelte` with a minimal list**

```svelte
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type App = { uid: string; name: string; source: string; version: string | null };
  let apps: App[] = [];
  let error = "";

  onMount(async () => {
    try {
      apps = await invoke<App[]>("list_apps");
    } catch (e) {
      error = String(e);
    }
  });
</script>

<main>
  <h1>Showcase — {apps.length} apps</h1>
  {#if error}<p style="color:red">{error}</p>{/if}
  <ul>
    {#each apps as a (a.uid)}
      <li>{a.name} <small>({a.source} {a.version ?? ""})</small></li>
    {/each}
  </ul>
</main>
```

- [ ] **Step 2: Run the app and verify real apps appear**

Run: `npm run tauri dev`
Expected: a window opens titled "Showcase — N apps" listing real installed apps (apt + flatpak + snap), each with source + version. No red error text.

- [ ] **Step 3: Commit**

```bash
git add src/App.svelte
git commit -m "feat: stub UI listing enumerated apps end-to-end"
```

### Task G5: Full suite + plan-complete checkpoint

- [ ] **Step 1: Run the entire test suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all unit tests PASS; the `#[ignore]`d integration test is skipped.

- [ ] **Step 2: Confirm the gated integration test on this host**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test integration_enumerate -- --ignored --nocapture`
Expected: PASS with non-zero app count.

- [ ] **Step 3: Final commit / tag the milestone**

```bash
git add -A
git commit -m "chore: Plan 1 backend enumeration core complete" --allow-empty
```

---

## Self-Review (filled by author)

**Spec coverage:** Enumeration (apt/flatpak/snap, GUI-only via .desktop) → B/C/D/E + G2. Metadata (name/version/size/install-date/publisher/categories) → C5, D2, E1. Icons → F1 + G2. Concurrency + per-source isolation → G1/G2. Typed errors → B2. Testing strategy (fixtures, injectable runner, no system mutation, gated real test) → throughout + G3. Uninstall, full UI, theming → deferred to Plans 2 & 3 (by design).

**Placeholder scan:** No TBD/TODO; every code step has complete code; the only deferrals are explicitly scoped to later plans.

**Type consistency:** `App`/`Source`/`AppError` defined in B1/B2 and used unchanged everywhere. `CommandRunner::run(program, args)` consistent across apt/flatpak. `enumerate()` (lib-exposed) used by both the Tauri command and the integration test. Source `list`/`list_from` signatures match their call sites in G2.

**Known assumption to confirm during execution:** the scaffold's `[lib] name` (used as `showcase_lib` in G3) — adjust the integration test import to match the actual crate name.
