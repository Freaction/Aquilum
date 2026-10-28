mod windows;
#[cfg(test)]
pub(crate) mod tests;

use super::delta::{self, compose, starts, Region};
use super::rebuild::{is_delta, rebuild};
use super::store::VersionFile;
use crate::documents::merge::{apply_edits, merge_external_change};
use crate::text::content::{self, read_string, Content, Splice};
use crate::text::pieces::Pieces;
use std::io;
use std::ops::Range;
use std::path::Path;

use windows::{windows, Window};

pub type Map = Vec<(Range<usize>, Range<usize>)>;

pub struct Reverted {
    pub text: Pieces<Box<dyn Content>>,
    pub regions: Vec<Region>,
    pub displaced: Vec<String>,
}

pub fn changes(dir: &Path, previous: &str, id: &str) -> io::Result<Map> {
    if is_delta(id) {
        let own = delta::read(&dir.join(id))?;
        if own.base.as_deref() == Some(previous) {
            return Ok(as_map(&own.regions));
        }
    }
    between(&rebuild(dir, previous)?, &rebuild(dir, id)?)
}

pub fn between(old: &dyn Content, new: &dyn Content) -> io::Result<Map> {
    let splice = content::splice(old, new)?;
    Ok(match splice.removed + splice.inserted {
        0 => Vec::new(),
        _ => vec![(splice.at..splice.at + splice.removed, splice.at..splice.at + splice.inserted)],
    })
}

pub fn revert(dir: &Path, versions: &[VersionFile], index: usize, current: Box<dyn Content>) -> io::Result<Reverted> {
    let previous = &versions[index - 1].id;
    let own = changes(dir, previous, &versions[index].id)?;
    let mut later = Vec::new();
    for pair in versions[index..].windows(2) {
        later = carry(later, &changes(dir, &pair[0].id, &pair[1].id)?);
    }
    let latest = rebuild(dir, &versions[versions.len() - 1].id)?;
    let later = as_map(&carry(later, &between(&latest, &*current)?));
    let version = rebuild(dir, &versions[index].id)?;
    let windows = windows(&version, &own, &later)?;
    apply(&windows, &rebuild(dir, previous)?, &version, current)
}

fn apply(windows: &[Window], previous: &dyn Content, version: &dyn Content, current: Box<dyn Content>) -> io::Result<Reverted> {
    let mut merged = Vec::new();
    let mut displaced = Vec::new();
    for window in windows {
        let mine = read_string(&*current, window.current.clone())?;
        let found = merge_external_change(&read_string(version, window.version.clone())?, &read_string(previous, window.previous.clone())?, &mine);
        displaced.extend(found.displaced);
        if !found.edits.is_empty() {
            merged.push((window.current.clone(), apply_edits(&mine, &found.edits)));
        }
    }
    let len = current.byte_len();
    let mut text = Pieces::new(vec![current]);
    let mut regions = Vec::new();
    let mut cursor = 0;
    for (range, result) in merged {
        text.push(0, cursor..range.start);
        regions.push(Region { from: range.start, to: range.end, len: result.len() });
        let source = text.source(Box::new(result));
        text.push(source, 0..regions[regions.len() - 1].len);
        cursor = range.end;
    }
    text.push(0, cursor..len);
    Ok(Reverted { text, regions, displaced })
}

fn carry(regions: Vec<Region>, step: &Map) -> Vec<Region> {
    step.iter().rev().fold(regions, |regions, (old, new)| compose(&regions, Splice { at: old.start, removed: old.len(), inserted: new.len() }))
}

fn as_map(regions: &[Region]) -> Map {
    regions.iter().zip(starts(regions)).map(|(region, at)| (region.from..region.to, at..at + region.len)).collect()
}
