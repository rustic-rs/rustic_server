use std::fmt::{Display, Formatter};

use serde::de;

pub mod constants {
    // TPE_LOCKS is is defined, but outside this types[] array.
    // This allow us to loop over the types[] when generating "routes"
    pub(crate) const TPE_DATA: &str = "data";
    pub(crate) const TPE_KEYS: &str = "keys";
    pub(crate) const TPE_LOCKS: &str = "locks";
    pub(crate) const TPE_SNAPSHOTS: &str = "snapshots";
    pub(crate) const TPE_INDEX: &str = "index";
    // FIXME: TPE_CONFIG is never used?
    pub(crate) const TPE_CONFIG: &str = "config";
    pub(crate) const TYPES: [&str; 5] = [TPE_DATA, TPE_KEYS, TPE_LOCKS, TPE_SNAPSHOTS, TPE_INDEX];
}

/// ArchivePathEnum hints what kind of path we received from the user.
///  - ArchivePathEnum::Repo points to the root of the repository.
///  - All other enum values point to data_type inside the repository
#[derive(Debug, PartialEq, Default)]
pub(crate) enum ArchivePathKind {
    Config,
    #[default]
    Data,
    Index,
    Keys,
    Locks,
    Repo,
    Snapshots,
}

pub(crate) struct ArchivePath {
    pub(crate) path_type: ArchivePathKind,
    pub(crate) tpe: String,
    pub(crate) path: String,
    pub(crate) name: String,
}

impl Display for ArchivePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[ArchivePath] path_type = {:?}, path: {}, tpe: {}, name: {:?}",
            self.path_type, self.path, self.tpe, self.name,
        )
    }
}
