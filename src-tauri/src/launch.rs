use crate::model::Source;

/// External command (program + args) to launch an app. Arg-array only (no shell).
///
/// Prefer launching via the .desktop entry when available (`gio launch <path>`),
/// which handles Exec field codes, Terminal=true, etc. Falls back to the package
/// manager's own run command when no desktop path is known.
pub fn build_launch_command(
    source: Source,
    desktop_path: Option<&str>,
    pkg_ref: &str,
) -> (&'static str, Vec<String>) {
    if let Some(path) = desktop_path {
        return ("gio", vec!["launch".into(), path.into()]);
    }
    match source {
        Source::Flatpak => ("flatpak", vec!["run".into(), pkg_ref.into()]),
        Source::Snap => ("snap", vec!["run".into(), pkg_ref.into()]),
        // Unreachable in practice (apt always has a desktop path); fails gracefully if hit.
        Source::Apt => ("gio", vec!["launch".into(), pkg_ref.into()]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_path_present_uses_gio_launch() {
        let (prog, args) =
            build_launch_command(Source::Apt, Some("/usr/share/applications/gedit.desktop"), "gedit");
        assert_eq!(prog, "gio");
        assert_eq!(args, vec!["launch", "/usr/share/applications/gedit.desktop"]);
    }

    #[test]
    fn desktop_path_overrides_source_for_flatpak() {
        let (prog, args) = build_launch_command(
            Source::Flatpak,
            Some("/var/lib/flatpak/exports/share/applications/org.gnome.Gedit.desktop"),
            "org.gnome.Gedit",
        );
        assert_eq!(prog, "gio");
        assert_eq!(
            args,
            vec!["launch", "/var/lib/flatpak/exports/share/applications/org.gnome.Gedit.desktop"]
        );
    }

    #[test]
    fn flatpak_without_desktop_path_uses_flatpak_run() {
        let (prog, args) = build_launch_command(Source::Flatpak, None, "com.github.wwmm.easyeffects");
        assert_eq!(prog, "flatpak");
        assert_eq!(args, vec!["run", "com.github.wwmm.easyeffects"]);
    }

    #[test]
    fn snap_without_desktop_path_uses_snap_run() {
        let (prog, args) = build_launch_command(Source::Snap, None, "firefox");
        assert_eq!(prog, "snap");
        assert_eq!(args, vec!["run", "firefox"]);
    }

    #[test]
    fn apt_without_desktop_path_falls_back_to_gio() {
        let (prog, args) = build_launch_command(Source::Apt, None, "gedit");
        assert_eq!(prog, "gio");
        assert_eq!(args, vec!["launch", "gedit"]);
    }

    #[test]
    fn shell_metacharacters_in_pkg_ref_stay_as_single_argv_element() {
        // Injection safety: a crafted pkg_ref with shell metacharacters must
        // arrive as a single uninterpreted element in the argv array.
        let malicious = "foo; rm -rf /";
        let (prog, args) = build_launch_command(Source::Flatpak, None, malicious);
        assert_eq!(prog, "flatpak");
        assert_eq!(args.len(), 2);
        assert_eq!(args[1], malicious, "metacharacters must not be split or interpreted");
    }

    #[test]
    fn shell_metacharacters_in_desktop_path_stay_as_single_argv_element() {
        let malicious = "/tmp/evil; rm -rf /";
        let (prog, args) = build_launch_command(Source::Apt, Some(malicious), "gedit");
        assert_eq!(prog, "gio");
        assert_eq!(args.len(), 2);
        assert_eq!(args[1], malicious, "path metacharacters must not be split or interpreted");
    }
}
