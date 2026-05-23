use crate::model::{AppError, Source};

/// The external command (program + args) that removes a package for a source.
/// Arg-array form only — never a shell string (injection-safe).
pub fn build_uninstall(source: Source, pkg_ref: &str) -> (&'static str, Vec<String>) {
    match source {
        // pkexec runs apt-get as root after the GUI polkit prompt. apt-get refuses
        // Essential packages and -y auto-confirms (may remove reverse-dependencies).
        Source::Apt => (
            "pkexec",
            vec![
                "apt-get".into(),
                "-y".into(),
                "remove".into(),
                pkg_ref.into(),
            ],
        ),
        // flatpak handles its own polkit (system installs) and needs no root for user installs.
        Source::Flatpak => (
            "flatpak",
            vec![
                "uninstall".into(),
                "--app".into(),
                "-y".into(),
                pkg_ref.into(),
            ],
        ),
        // pkexec runs snap as root (snapd then needs no polkit).
        Source::Snap => (
            "pkexec",
            vec!["snap".into(), "remove".into(), pkg_ref.into()],
        ),
        // Placeholder — Phase D replaces this with a delete-file path.
        Source::AppImage => ("true", vec![]),
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

/// Map a failed removal's stderr into a typed error. Anchored to the phrases
/// polkit/pkcon emit when the user cancels or auth fails, to avoid mislabeling
/// unrelated backend failures that merely contain "cancel".
pub fn classify_error(stderr: &str) -> AppError {
    let s = stderr.to_lowercase();
    const AUTH_PHRASES: [&str; 6] = [
        "not authorized",
        "authentication failed",
        "authentication is required",
        "authentication required",
        "request dismissed",
        "operation was cancelled",
    ];
    if AUTH_PHRASES.iter().any(|p| s.contains(p)) {
        AppError::PermissionDenied("authentication was cancelled or denied".into())
    } else {
        AppError::Backend(stderr.trim().to_string())
    }
}

/// Returns true iff dpkg considers this package essential.
/// Defense-in-depth check before any privileged apt removal.
pub fn apt_is_essential(runner: &dyn crate::runner::CommandRunner, pkg: &str) -> bool {
    match runner.run("dpkg-query", &["-W", "-f=${Essential}", pkg]) {
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
    fn apt_maps_to_pkexec_apt_get_remove() {
        let (prog, args) = build_uninstall(Source::Apt, "gimp");
        assert_eq!(prog, "pkexec");
        assert_eq!(args, vec!["apt-get", "-y", "remove", "gimp"]);
    }

    #[test]
    fn flatpak_maps_to_flatpak_uninstall() {
        let (prog, args) = build_uninstall(Source::Flatpak, "org.gimp.GIMP");
        assert_eq!(prog, "flatpak");
        assert_eq!(args, vec!["uninstall", "--app", "-y", "org.gimp.GIMP"]);
    }

    #[test]
    fn snap_maps_to_pkexec_snap_remove() {
        let (prog, args) = build_uninstall(Source::Snap, "firefox");
        assert_eq!(prog, "pkexec");
        assert_eq!(args, vec!["snap", "remove", "firefox"]);
    }

    /// Shell-metacharacter pkg_ref must stay as a single verbatim argument — no injection.
    #[test]
    fn pkg_ref_with_shell_metacharacters_is_single_arg() {
        let evil = "a; rm -rf ~";
        let (prog, args) = build_uninstall(Source::Apt, evil);
        assert_eq!(prog, "pkexec");
        // The evil string must appear as exactly one element at the end, unmodified.
        assert_eq!(args.len(), 4);
        assert_eq!(args.last().unwrap(), evil);
    }

    #[test]
    fn snap_injection_safety() {
        let evil = "$(reboot)";
        let (prog, args) = build_uninstall(Source::Snap, evil);
        assert_eq!(prog, "pkexec");
        assert_eq!(args.len(), 3);
        assert_eq!(args.last().unwrap(), evil);
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
        // Real polkit output: "Not authorized to perform operation"
        let err = classify_error("Not authorized to perform operation");
        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn authentication_failed_maps_to_permission_denied() {
        // pkcon/polkit: "Authentication failed"
        let err = classify_error("Authentication failed");
        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn operation_was_cancelled_maps_to_permission_denied() {
        // polkit dialog closed: "GDBus.Error:org.freedesktop.PolicyKit1.Error.Cancelled: Operation was cancelled"
        let err = classify_error(
            "GDBus.Error:org.freedesktop.PolicyKit1.Error.Cancelled: Operation was cancelled",
        );
        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn request_dismissed_maps_to_permission_denied() {
        // polkit: "polkit: Request dismissed"
        let err = classify_error("polkit: Request dismissed");
        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn apt_package_not_found_maps_to_backend() {
        let err = classify_error("E: Unable to locate package foo");
        assert!(matches!(err, AppError::Backend(_)));
    }

    /// Regression: a backend error that contains the word "cancelled" in a different
    /// context must NOT be mislabeled as PermissionDenied.
    #[test]
    fn transaction_cancelled_due_to_dep_conflict_maps_to_backend() {
        let err = classify_error(
            "Error: transaction cancelled due to dependency conflict with libssl",
        );
        assert!(matches!(err, AppError::Backend(_)));
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
