use crate::model::{App, AppError};

/// Snap apps come fully-formed from snapd; desktop entries are only used
/// later for icon resolution (handled in the icons step), not enumeration.
pub fn list() -> Result<Vec<App>, AppError> {
    crate::snapd::list()
}
