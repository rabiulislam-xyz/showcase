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
                desktop_path: None,
                pkg_ref: app_id.to_string(),
                removable: true,
                protected_reason: None,
                update_available: None,
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
