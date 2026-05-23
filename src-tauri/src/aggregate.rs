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
    // Total order: name (case-insensitive) with uid as a tiebreaker so apps
    // sharing a name keep a deterministic, stable position across runs.
    agg.apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.uid.cmp(&b.uid))
    });
    agg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Source;

    fn app(name: &str) -> App {
        app_src(Source::Apt, name)
    }

    fn app_src(source: Source, name: &str) -> App {
        App {
            uid: App::make_uid(source, name),
            source,
            name: name.to_string(),
            summary: None, description: None, version: None, icon_path: None,
            size_bytes: None, install_date: None, publisher: None,
            categories: vec![], exec: None, desktop_path: None,
            pkg_ref: name.to_string(),
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

    #[test]
    fn equal_names_break_ties_by_uid_deterministically() {
        // Same display name from different sources → uids "flatpak:Code",
        // "snap:Code". The uid tiebreaker must order them deterministically
        // regardless of input order.
        let snap = app_src(Source::Snap, "Code"); // uid "snap:Code"
        let flat = app_src(Source::Flatpak, "Code"); // uid "flatpak:Code"

        let forward = merge(vec![("a", Ok(vec![snap.clone(), flat.clone()]))]);
        let reversed = merge(vec![("a", Ok(vec![flat, snap]))]);

        // "flatpak:Code" < "snap:Code", so flatpak sorts first either way.
        assert_eq!(forward.apps[0].uid, "flatpak:Code");
        assert_eq!(forward.apps[1].uid, "snap:Code");
        let f: Vec<_> = forward.apps.iter().map(|a| a.uid.clone()).collect();
        let r: Vec<_> = reversed.apps.iter().map(|a| a.uid.clone()).collect();
        assert_eq!(f, r, "tie ordering must be input-order independent");
    }
}
