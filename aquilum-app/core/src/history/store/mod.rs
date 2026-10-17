mod legacy;
mod names;
mod retention;
#[cfg(test)]
mod tests;

use super::delta;
use super::milestone::Source;
use crate::files::document::write_content_atomic;
use crate::files::error::FileCommandError;
use crate::text::content::Content;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use legacy::adopt_quantum_history;
pub use names::{names, remove_version_and_names, write_name};
pub use retention::remove_expired;

pub const AQUILUM_FOLDER: &str = ".aquilum";
const HISTORY_FOLDER: &str = "history";
pub const VERSION_EXTENSION: &str = ".md";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionFile {
    pub id: String,
    pub at_ms: u64,
    pub device: String,
    pub source: Source,
    pub from_ms: Option<u64>,
    pub name: Option<String>,
    pub is_current: bool,
}

fn history_root(base: &Path) -> PathBuf {
    base.join(AQUILUM_FOLDER).join(HISTORY_FOLDER)
}

pub fn history_dir(base: &Path, relative: &Path) -> PathBuf {
    history_root(base).join(relative)
}

pub fn version_path(dir: &Path, device: &str, source: Source, at: SystemTime, extension: &str) -> PathBuf {
    let label = source.file_label();
    free_name(dir, millis(at), |at| format!("{at:013}_{device}_{label}{extension}"))
}

pub fn write_version(
    dir: &Path,
    device: &str,
    source: Source,
    at: SystemTime,
    text: &dyn Content,
) -> Result<PathBuf, FileCommandError> {
    fs::create_dir_all(dir)?;
    let path = version_path(dir, device, source, at, VERSION_EXTENSION);
    write_content_atomic(&path, text, None)?;
    Ok(path)
}

pub fn versions(dir: &Path) -> Vec<VersionFile> {
    let mut versions = files(dir)
        .filter_map(|(file_name, _)| parse_version(file_name))
        .collect::<Vec<_>>();
    versions.sort_by(|left, right| {
        (left.at_ms, &left.device, left.source != Source::Start, &left.id)
            .cmp(&(right.at_ms, &right.device, right.source != Source::Start, &right.id))
    });
    versions
}

pub fn version_at(id: &str) -> Option<u64> {
    parse_version(id.to_owned()).map(|version| version.at_ms)
}

pub fn system_time_from_ms(millis: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(millis)
}

fn files(dir: &Path) -> impl Iterator<Item = (String, PathBuf)> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| (entry.file_name().to_string_lossy().into_owned(), entry.path()))
}

fn parse_version(name: String) -> Option<VersionFile> {
    let (at_ms, device, source) = {
        let stem = name.strip_suffix(VERSION_EXTENSION).or_else(|| name.strip_suffix(delta::EXTENSION))?;
        let mut parts = stem.splitn(3, '_');
        let at_ms = parts.next()?.parse().ok()?;
        let device = parts.next()?.to_owned();
        (at_ms, device, Source::from_label(parts.next()?)?)
    };
    Some(VersionFile {
        id: name,
        at_ms,
        device,
        from_ms: source.origin_ms(),
        source,
        name: None,
        is_current: false,
    })
}

fn millis(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}

fn free_name(dir: &Path, at_ms: u64, name: impl Fn(u64) -> String) -> PathBuf {
    (at_ms..)
        .map(|at| dir.join(name(at)))
        .find(|candidate| !candidate.exists())
        .unwrap_or_else(|| dir.join(name(at_ms)))
}
