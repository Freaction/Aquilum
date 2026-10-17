mod diff;
mod label;
mod rewrite;
#[cfg(test)]
mod tests;

use std::path::Path;

use aquilum_core::Core;
pub use aquilum_core::history::HistoryPage as Page;
pub use aquilum_core::history::store::VersionFile as Version;

pub use diff::{Change, diff, hunk_lines};
pub use label::{group_by_day, heading, label};
pub use rewrite::{restore, revert, view};

pub const PAGE_SIZE: usize = 50;

pub fn page(core: &Core, note: &Path, current: Option<&str>, limit: usize) -> Page {
    core.history.page(&|| core.known_roots(), note, current, 0, limit)
}
