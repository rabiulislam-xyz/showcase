use crate::model::{AppError, Source};

/// The external command (program + args) that removes a package for a source.
/// Arg-array form only — never a shell string (injection-safe).
pub fn build_uninstall(source: Source, pkg_ref: &str) -> (&'static str, Vec<String>) {
    match source {
        // pkcon (PackageKit) raises the polkit dialog; -y auto-confirms PK's own prompt.
        Source::Apt => ("pkcon", vec!["-y".into(), "remove".into(), pkg_ref.into()]),
        Source::Flatpak => (
            "flatpak",
            vec![
                "uninstall".into(),
                "--app".into(),
                "-y".into(),
                pkg_ref.into(),
            ],
        ),
        Source::Snap => ("snap", vec!["remove".into(), pkg_ref.into()]),
    }
}

/// Refuse removal of system-critical packages. Returns Some(reason) if protected.
pub fn protected_reason(source: Source, pkg_ref: &str) -> Option<String> {
    match source {
        Source::Snap
            if matches!(
                pkg_ref,
                "core"
                    | "core18"
                    | "core20"
                    | "core22"
                    | "core24"
                    | "snapd"
                    | "bare"
                    | "snapd-desktop-integration"
            ) =>
        {
            Some(format!("{pkg_ref} is a base/system snap"))
        }
        _ => None,
    }
}

/// Map a failed removal's stderr/exit into a typed error.
pub fn classify_error(stderr: &str) -> AppError {
    let s = stderr.to_lowercase();
    if s.contains("not authorized")
        || s.contains("authentication")
        || s.contains("cancel")
        || s.contains("dismiss")
    {
        AppError::PermissionDenied("authentication was cancelled or denied".into())
    } else {
        AppError::Backend(stderr.trim().to_string())
    }
}

/// Returns true iff dpkg considers this package essential.
/// Defense-in-depth check before any privileged apt removal.
pub fn apt_is_essential(runner: &dyn crate::runner::CommandRunner, pkg: &str) -> bool {
    match runner.run("dpkg-query", &["-W", &format!("-f=${{Essential}}"), pkg]) {
        Ok(out) => out.trim() == "yes",
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::FakeRunner;

    // ── build_uninstall ───────────────────────────────────────────────────────

    #[test]
    fn apt_maps_to_pkcon_remove() {
        let (prog, args) = build_uninstall(Source::Apt, "gimp");
        assert_eq!(prog, "pkcon");
        assert_eq!(args, vec!["-y", "remove", "gimp"]);
    }

    #[test]
    fn flatpak_maps_to_flatpak_uninstall() {
        let (prog, args) = build_uninstall(Source::Flatpak, "org.gimp.GIMP");
        assert_eq!(prog, "flatpak");
        assert_eq!(args, vec!["uninstall", "--app", "-y", "org.gimp.GIMP"]);
    }

    #[test]
    fn snap_maps_to_snap_remove() {
        let (prog, args) = build_uninstall(Source::Snap, "firefox");
        assert_eq!(prog, "snap");
        assert_eq!(args, vec!["remove", "firefox"]);
    }

    /// Shell-metacharacter pkg_ref must stay as a single verbatim argument — no injection.
    #[test]
    fn pkg_ref_with_shell_metacharacters_is_single_arg() {
        let evil = "a; rm -rf ~";
        let (prog, args) = build_uninstall(Source::Apt, evil);
        assert_eq!(prog, "pkcon");
        // The evil string must appear as exactly one element, unmodified.
        assert_eq!(args.len(), 3);
        assert_eq!(args[2], evil);
    }

    #[test]
    fn snap_injection_safety() {
        let evil = "$(reboot)";
        let (prog, args) = build_uninstall(Source::Snap, evil);
        assert_eq!(prog, "snap");
        assert_eq!(args.len(), 2);
        assert_eq!(args[1], evil);
    }

    // ── protected_reason ─────────────────────────────────────────────────────

    #[test]
    fn core22_is_protected() {
        assert!(protected_reason(Source::Snap, "core22").is_some());
    }

    #[test]
    fn snapd_is_protected() {
        assert!(protected_reason(Source::Snap, "snapd").is_some());
    }

    #[test]
    fn bare_is_protected() {
        assert!(protected_reason(Source::Snap, "bare").is_some());
    }

    #[test]
    fn firefox_snap_is_not_protected() {
        assert!(protected_reason(Source::Snap, "firefox").is_none());
    }

    #[test]
    fn apt_package_is_not_protected_by_snap_guard() {
        // The snap guard must not fire for apt packages named the same.
        assert!(protected_reason(Source::Apt, "core22").is_none());
    }

    // ── classify_error ───────────────────────────────────────────────────────

    #[test]
    fn not_authorized_maps_to_permission_denied() {
        let err = classify_error("Not authorized");
        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn authentication_maps_to_permission_denied() {
        let err = classify_error("authentication failed");
        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn cancel_maps_to_permission_denied() {
        let err = classify_error("Operation was cancelled by user");
        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn dismiss_maps_to_permission_denied() {
        let err = classify_error("Dialog was dismissed");
        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn other_stderr_maps_to_backend() {
        let err = classify_error("package not found in database");
        assert!(matches!(err, AppError::Backend(_)));
    }

    #[test]
    fn empty_stderr_maps_to_backend() {
        let err = classify_error("");
        assert!(matches!(err, AppError::Backend(_)));
    }

    // ── apt_is_essential ─────────────────────────────────────────────────────

    #[test]
    fn essential_yes_returns_true() {
        let runner = FakeRunner::new().with("dpkg-query", "yes");
        assert!(apt_is_essential(&runner, "base-files"));
    }

    #[test]
    fn essential_empty_returns_false() {
        let runner = FakeRunner::new().with("dpkg-query", "");
        assert!(!apt_is_essential(&runner, "gimp"));
    }

    #[test]
    fn essential_no_returns_false() {
        let runner = FakeRunner::new().with("dpkg-query", "no");
        assert!(!apt_is_essential(&runner, "gimp"));
    }

    #[test]
    fn essential_error_returns_false() {
        // No fake registered → FakeRunner returns Err; must not panic.
        let runner = FakeRunner::new();
        assert!(!apt_is_essential(&runner, "gimp"));
    }
}
