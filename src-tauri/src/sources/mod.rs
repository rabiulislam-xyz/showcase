use crate::model::{App, AppError};

pub mod apt;
pub mod flatpak;
pub mod snap;

/// A package source that can enumerate its installed GUI apps.
/// (Uninstall is added in Plan 3.)
pub trait AppSource {
    fn list(&self) -> Result<Vec<App>, AppError>;
}
