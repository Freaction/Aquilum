use std::path::Path;

use aquilum_core::Core;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate;
use aquilum_core::history::Source;
use aquilum_core::history::service::VersionView;

use super::Version;

pub fn view(core: &Core, note: &Path, id: &str) -> Option<VersionView> {
    core.history.view(&|| core.known_roots(), note, id)
}

pub fn restore(core: &Core, note: &Path, version: &Version) -> Result<bool, FileCommandError> {
    gate::restore_version(core, note, &version.id, Source::Restore(version.at_ms), None)
}

pub fn revert(core: &Core, note: &Path, version: &Version) -> Result<bool, FileCommandError> {
    gate::revert_version(core, note, &version.id, Source::Revert(version.at_ms), crate::dates::offset_seconds())
}
