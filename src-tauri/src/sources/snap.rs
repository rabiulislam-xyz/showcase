use crate::model::{App, AppError};

/// Snap apps come fully-formed from snapd; desktop entries are only used
/// later for icon resolution (handled in the icons step), not enumeration.
pub fn list() -> Result<Vec<App>, AppError> {
    crate::snapd::list()
}

/// Seam over snapd so enumeration/details can be exercised without the live
/// `/run/snapd.socket`. Implemented for real by [`SnapdSocket`].
pub trait SnapSource: Sync {
    /// List installed snap apps.
    fn list(&self) -> Result<Vec<App>, AppError>;
    /// Long description for a single snap; None on any error or when absent.
    fn description(&self, name: &str) -> Option<String>;
}

/// Production implementation: talks to the live snapd unix socket.
pub struct SnapdSocket;

impl SnapSource for SnapdSocket {
    fn list(&self) -> Result<Vec<App>, AppError> {
        crate::snapd::list()
    }

    fn description(&self, name: &str) -> Option<String> {
        crate::snapd::get_snap_description(name)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::collections::HashMap;

    /// Test double holding canned snap apps + per-name descriptions.
    pub struct FakeSnap {
        pub apps: Result<Vec<App>, AppError>,
        pub descriptions: HashMap<String, String>,
    }

    impl FakeSnap {
        pub fn new() -> Self {
            Self { apps: Ok(Vec::new()), descriptions: HashMap::new() }
        }
        pub fn with_apps(mut self, apps: Vec<App>) -> Self {
            self.apps = Ok(apps);
            self
        }
        pub fn failing(err: AppError) -> Self {
            Self { apps: Err(err), descriptions: HashMap::new() }
        }
        pub fn with_description(mut self, name: &str, desc: &str) -> Self {
            self.descriptions.insert(name.to_string(), desc.to_string());
            self
        }
    }

    impl SnapSource for FakeSnap {
        fn list(&self) -> Result<Vec<App>, AppError> {
            self.apps.clone()
        }
        fn description(&self, name: &str) -> Option<String> {
            self.descriptions.get(name).cloned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::FakeSnap;
    use super::*;
    use crate::model::Source;

    fn snap_app(name: &str) -> App {
        App {
            uid: App::make_uid(Source::Snap, name),
            source: Source::Snap,
            name: name.to_string(),
            summary: None,
            description: None,
            version: None,
            icon_path: None,
            size_bytes: None,
            install_date: None,
            publisher: None,
            categories: vec![],
            exec: None,
            pkg_ref: name.to_string(),
            removable: true,
            protected_reason: None,
        }
    }

    #[test]
    fn fake_snap_returns_canned_apps_and_descriptions() {
        let fake = FakeSnap::new()
            .with_apps(vec![snap_app("firefox")])
            .with_description("firefox", "A web browser");
        assert_eq!(fake.list().unwrap().len(), 1);
        assert_eq!(fake.description("firefox").as_deref(), Some("A web browser"));
        assert_eq!(fake.description("unknown"), None);
    }

    #[test]
    fn fake_snap_can_simulate_failure() {
        let fake = FakeSnap::failing(AppError::SourceUnavailable("down".into()));
        assert!(fake.list().is_err());
    }
}
