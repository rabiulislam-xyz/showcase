use crate::model::{App, Source};
use crate::runner::CommandRunner;

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

/// Command (program + args) that updates one package for a source. Arg-array only.
pub fn build_update_command(source: Source, pkg_ref: &str) -> (&'static str, Vec<String>) {
    match source {
        Source::Apt => ("pkexec", vec!["apt-get".into(), "-y".into(), "install".into(), "--only-upgrade".into(), pkg_ref.into()]),
        Source::Flatpak => ("flatpak", vec!["update".into(), "-y".into(), pkg_ref.into()]),
        Source::Snap => ("pkexec", vec!["snap".into(), "refresh".into(), pkg_ref.into()]),
        // AppImage self-update is out of scope; guarded upstream in perform_update.
        Source::AppImage => ("true", vec![]),
    }
}

/// Check ONLY this app's source for an available update, using CACHED metadata.
///
/// Unlike [`check_updates_with`], this performs NO privileged metadata refresh
/// (`pkexec apt-get update`): it reads whatever the package managers already know.
/// That keeps the per-app check instant and password-free.
///
/// Returns `Some(version)` when the package is listed as upgradable, otherwise
/// `None`. AppImage has no update path, and any source error degrades to `None`.
pub fn check_app_update_with(runner: &dyn CommandRunner, source: Source, pkg_ref: &str) -> Option<String> {
    let pairs = match source {
        // Unprivileged, cached: no `apt-get update` first.
        Source::Apt => parse_apt_upgradable(&runner.run("apt", &["list", "--upgradable"]).ok()?),
        Source::Flatpak => parse_flatpak_updates(
            &runner.run("flatpak", &["remote-ls", "--updates", "--app", "--columns=application,version"]).ok()?,
        ),
        Source::Snap => parse_snap_refresh_list(&runner.run("snap", &["refresh", "--list"]).ok()?),
        Source::AppImage => return None,
    };
    pairs.into_iter().find(|(p, _)| p == pkg_ref).map(|(_, v)| v)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AppError;
    use crate::runner::FakeRunner;
    use std::collections::HashMap;

    // ── parse_apt_upgradable ──────────────────────────────────────────────────

    #[test]
    fn apt_upgradable_parses_standard_output() {
        let input = "Listing...\nfirefox/jammy-updates 125.0 amd64 [upgradable from: 124.0]\nvim/jammy 2:9.0 amd64 [upgradable from: 2:8.2]\n";
        let result = parse_apt_upgradable(input);
        assert_eq!(result, vec![
            ("firefox".to_string(), "125.0".to_string()),
            ("vim".to_string(), "2:9.0".to_string()),
        ]);
    }

    #[test]
    fn apt_upgradable_empty_input_returns_empty() {
        assert_eq!(parse_apt_upgradable(""), vec![]);
    }

    #[test]
    fn apt_upgradable_listing_only_returns_empty() {
        assert_eq!(parse_apt_upgradable("Listing...\n"), vec![]);
    }

    // ── parse_flatpak_updates ─────────────────────────────────────────────────

    #[test]
    fn flatpak_updates_parses_tab_separated() {
        let input = "org.gimp.GIMP\t2.10.38\ncom.x.App\t1.1\n";
        let result = parse_flatpak_updates(input);
        assert_eq!(result, vec![
            ("org.gimp.GIMP".to_string(), "2.10.38".to_string()),
            ("com.x.App".to_string(), "1.1".to_string()),
        ]);
    }

    #[test]
    fn flatpak_updates_empty_input_returns_empty() {
        assert_eq!(parse_flatpak_updates(""), vec![]);
    }

    // ── parse_snap_refresh_list ───────────────────────────────────────────────

    #[test]
    fn snap_refresh_list_parses_standard_output() {
        let input = "Name    Version  Rev  Publisher  Notes\nfirefox 126.0    1234 mozilla    -\n";
        let result = parse_snap_refresh_list(input);
        assert_eq!(result, vec![("firefox".to_string(), "126.0".to_string())]);
    }

    #[test]
    fn snap_refresh_list_up_to_date_returns_empty() {
        assert_eq!(parse_snap_refresh_list("All snaps up to date.\n"), vec![]);
    }

    // ── build_update_command ──────────────────────────────────────────────────

    #[test]
    fn apt_update_maps_to_pkexec_apt_get_install_only_upgrade() {
        let (prog, args) = build_update_command(Source::Apt, "gimp");
        assert_eq!(prog, "pkexec");
        assert_eq!(args, vec!["apt-get", "-y", "install", "--only-upgrade", "gimp"]);
    }

    #[test]
    fn flatpak_update_maps_to_flatpak_update() {
        let (prog, args) = build_update_command(Source::Flatpak, "org.gimp.GIMP");
        assert_eq!(prog, "flatpak");
        assert_eq!(args, vec!["update", "-y", "org.gimp.GIMP"]);
    }

    #[test]
    fn snap_update_maps_to_pkexec_snap_refresh() {
        let (prog, args) = build_update_command(Source::Snap, "firefox");
        assert_eq!(prog, "pkexec");
        assert_eq!(args, vec!["snap", "refresh", "firefox"]);
    }

    /// Shell-metacharacter pkg_ref must stay as a single verbatim argv element — no injection.
    #[test]
    fn build_update_command_shell_metacharacters_are_single_arg() {
        let evil = "a; rm -rf ~";
        let (prog, args) = build_update_command(Source::Apt, evil);
        assert_eq!(prog, "pkexec");
        assert_eq!(args.len(), 5);
        assert_eq!(args.last().unwrap(), evil);
    }

    #[test]
    fn build_update_command_snap_injection_safety() {
        let evil = "$(reboot)";
        let (prog, args) = build_update_command(Source::Snap, evil);
        assert_eq!(prog, "pkexec");
        assert_eq!(args.len(), 3);
        assert_eq!(args.last().unwrap(), evil);
    }

    // ── check_updates_with ────────────────────────────────────────────────────

    #[test]
    fn check_updates_with_merges_all_sources() {
        let runner = FakeRunner::new()
            .with("pkexec", "") // apt-get update succeeds
            .with("apt", "Listing...\nfirefox/jammy-updates 125.0 amd64 [upgradable from: 124.0]\n")
            .with("flatpak", "org.gimp.GIMP\t2.10.38\n")
            .with("snap", "Name    Version  Rev  Publisher  Notes\ncodium  1.89.0   100 vscodium   -\n");

        let (pairs, warnings) = check_updates_with(&runner);
        assert!(warnings.is_empty(), "expected no warnings, got: {warnings:?}");
        assert!(pairs.contains(&("apt:firefox".to_string(), "125.0".to_string())), "missing apt entry");
        assert!(pairs.contains(&("flatpak:org.gimp.GIMP".to_string(), "2.10.38".to_string())), "missing flatpak entry");
        assert!(pairs.contains(&("snap:codium".to_string(), "1.89.0".to_string())), "missing snap entry");
        assert_eq!(pairs.len(), 3);
    }

    #[test]
    fn check_updates_with_failing_source_becomes_warning_others_survive() {
        // apt fails (pkexec not found), flatpak + snap succeed
        let mut responses: HashMap<String, Result<String, AppError>> = HashMap::new();
        responses.insert("pkexec".to_string(), Err(AppError::Backend("pkexec not found".into())));
        responses.insert("flatpak".to_string(), Ok("org.gimp.GIMP\t2.10.38\n".to_string()));
        responses.insert("snap".to_string(), Ok("Name    Version  Rev  Publisher  Notes\n".to_string()));
        let runner = FakeRunnerMulti { responses };

        let (pairs, warnings) = check_updates_with(&runner);
        assert_eq!(warnings.len(), 1, "expected exactly one warning");
        assert!(warnings[0].contains("apt"), "warning should mention 'apt'");
        // flatpak result survives
        assert!(pairs.contains(&("flatpak:org.gimp.GIMP".to_string(), "2.10.38".to_string())));
    }

    // ── check_app_update_with ─────────────────────────────────────────────────

    #[test]
    fn check_app_update_apt_found_returns_version() {
        let runner = FakeRunner::new()
            .with("apt", "Listing...\nfirefox/jammy-updates 125.0 amd64 [upgradable from: 124.0]\nvim/jammy 2:9.0 amd64 [upgradable from: 2:8.2]\n");
        assert_eq!(
            check_app_update_with(&runner, Source::Apt, "firefox"),
            Some("125.0".to_string())
        );
    }

    #[test]
    fn check_app_update_apt_not_listed_returns_none() {
        // Output lists firefox only; querying gimp must yield None.
        let runner = FakeRunner::new()
            .with("apt", "Listing...\nfirefox/jammy-updates 125.0 amd64 [upgradable from: 124.0]\n");
        assert_eq!(check_app_update_with(&runner, Source::Apt, "gimp"), None);
    }

    #[test]
    fn check_app_update_flatpak_found_returns_version() {
        let runner = FakeRunner::new().with("flatpak", "org.gimp.GIMP\t2.10.38\ncom.x.App\t1.1\n");
        assert_eq!(
            check_app_update_with(&runner, Source::Flatpak, "org.gimp.GIMP"),
            Some("2.10.38".to_string())
        );
    }

    #[test]
    fn check_app_update_flatpak_not_listed_returns_none() {
        let runner = FakeRunner::new().with("flatpak", "com.x.App\t1.1\n");
        assert_eq!(check_app_update_with(&runner, Source::Flatpak, "org.gimp.GIMP"), None);
    }

    #[test]
    fn check_app_update_snap_found_returns_version() {
        let runner = FakeRunner::new()
            .with("snap", "Name    Version  Rev  Publisher  Notes\nfirefox 126.0    1234 mozilla    -\n");
        assert_eq!(
            check_app_update_with(&runner, Source::Snap, "firefox"),
            Some("126.0".to_string())
        );
    }

    #[test]
    fn check_app_update_snap_not_listed_returns_none() {
        let runner = FakeRunner::new().with("snap", "All snaps up to date.\n");
        assert_eq!(check_app_update_with(&runner, Source::Snap, "firefox"), None);
    }

    #[test]
    fn check_app_update_appimage_is_always_none() {
        // Even with a (nonsensical) canned response, AppImage short-circuits to None.
        let runner = FakeRunner::new().with("apt", "Listing...\nfoo/x 1.0 amd64 [upgradable from: 0.9]\n");
        assert_eq!(check_app_update_with(&runner, Source::AppImage, "foo"), None);
    }

    #[test]
    fn check_app_update_source_error_is_none() {
        // No fake registered → runner errors → None (no panic), per source.
        let runner = FakeRunner::new();
        assert_eq!(check_app_update_with(&runner, Source::Apt, "firefox"), None);
        assert_eq!(check_app_update_with(&runner, Source::Flatpak, "org.x.App"), None);
        assert_eq!(check_app_update_with(&runner, Source::Snap, "firefox"), None);
    }

    /// A FakeRunner variant that returns different responses per program (FakeRunner already
    /// does this by key, but we need to inject an Err — use FakeRunnerMulti for that test).
    struct FakeRunnerMulti {
        responses: HashMap<String, Result<String, AppError>>,
    }

    impl CommandRunner for FakeRunnerMulti {
        fn run(&self, program: &str, _args: &[&str]) -> Result<String, AppError> {
            self.responses
                .get(program)
                .cloned()
                .unwrap_or_else(|| Err(AppError::Backend(format!("no fake for {program}"))))
        }
    }
}
