mod diff;
mod label;
mod rewrite;
#[cfg(test)]
mod tests;

use std::path::Path;

use aquilum_core::Core;
use aquilum_core::files::gate;
use aquilum_core::history::Source;
use aquilum_core::history::store::VersionFile;

pub use diff::{Change, diff};
pub use label::{group_by_day, heading, label};
pub use rewrite::{restore, revert, texts};

pub const PAGE_SIZE: usize = 50;

#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    pub id: String,
    pub at_ms: u64,
    pub source: Source,
    pub from_ms: Option<u64>,
    pub name: Option<String>,
    pub is_current: bool,
}

impl From<VersionFile> for Version {
    fn from(v: VersionFile) -> Self {
        Version { id: v.id, at_ms: v.at_ms, source: v.source, from_ms: v.from_ms, name: v.name, is_current: v.is_current }
    }
}

pub struct Page {
    pub total: usize,
    pub versions: Vec<Version>,
}

pub fn page(core: &Core, note: &Path, limit: usize) -> Page {
    let page = core.history.page(&|| core.known_roots(), note, 0, limit);
    Page { total: page.total, versions: page.versions.into_iter().map(Version::from).collect() }
}

pub fn rename(core: &Core, note: &Path, version: &Version, name: &str) {
    gate::name_version(core, note, Some(&version.id), name.trim());
}
