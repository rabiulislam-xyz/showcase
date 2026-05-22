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
