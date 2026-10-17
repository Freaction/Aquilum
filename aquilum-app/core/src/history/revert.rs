use super::delta::{self, compose, starts, Region};
use super::rebuild::{is_delta, rebuild};
use super::store::VersionFile;
use crate::text::content::{self, find_byte, read_bytes, Content, Splice};
use std::io;
use std::ops::Range;
use std::path::Path;

pub type Map = Vec<(Range<usize>, Range<usize>)>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub previous: Range<usize>,
    pub version: Range<usize>,
    pub current: Range<usize>,
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

pub fn plan(dir: &Path, versions: &[VersionFile], index: usize, current: &dyn Content) -> io::Result<Vec<Window>> {
    let Some(previous) = index.checked_sub(1).map(|previous| versions[previous].id.as_str()) else {
        return Ok(Vec::new());
    };
    let own = changes(dir, previous, &versions[index].id)?;
    let mut later = Vec::new();
    for pair in versions[index..].windows(2) {
        later = carry(later, &changes(dir, &pair[0].id, &pair[1].id)?);
    }
    let latest = versions.last().map_or("", |version| version.id.as_str());
    let later = as_map(&carry(later, &between(&rebuild(dir, latest)?, current)?));
    let version = rebuild(dir, &versions[index].id)?;
    let mut windows = Vec::new();
    for (_, range) in &own {
        windows.push(grow(&version, range.clone(), &own, &later)?);
    }
    windows.dedup();
    Ok(windows
        .into_iter()
        .map(|range| Window {
            previous: shift(range.start, own.iter().map(|(p, v)| (v, p)), true)..shift(range.end, own.iter().map(|(p, v)| (v, p)), true),
            current: shift(range.start, later.iter().map(|(v, c)| (v, c)), true)..shift(range.end, later.iter().map(|(v, c)| (v, c)), false),
            version: range,
        })
        .collect())
}

fn carry(regions: Vec<Region>, step: &Map) -> Vec<Region> {
    step.iter().rev().fold(regions, |regions, (old, new)| compose(&regions, Splice { at: old.start, removed: old.len(), inserted: new.len() }))
}

fn as_map(regions: &[Region]) -> Map {
    regions.iter().zip(starts(regions)).map(|(region, at)| (region.from..region.to, at..at + region.len)).collect()
}

fn grow(version: &dyn Content, mut range: Range<usize>, own: &Map, later: &Map) -> io::Result<Range<usize>> {
    loop {
        let mut next = line_start(version, range.start)?..line_end(version, range.end)?;
        for region in own.iter().map(|(_, version)| version).chain(later.iter().map(|(version, _)| version)) {
            if crosses(region, &next) {
                next = next.start.min(region.start)..next.end.max(region.end);
            }
        }
        if next == range {
            return Ok(range);
        }
        range = next;
    }
}

fn crosses(region: &Range<usize>, window: &Range<usize>) -> bool {
    match region.is_empty() {
        true => window.start < region.start && region.start < window.end,
        false => region.start < window.end && window.start < region.end,
    }
}

pub fn shift<'a>(at: usize, pairs: impl IntoIterator<Item = (&'a Range<usize>, &'a Range<usize>)>, past_insertions: bool) -> usize {
    let growth: isize = pairs
        .into_iter()
        .filter(|(old, _)| old.end < at || old.end == at && (!old.is_empty() || past_insertions))
        .map(|(old, new)| new.len() as isize - old.len() as isize)
        .sum();
    (at as isize + growth) as usize
}

fn line_start(text: &dyn Content, at: usize) -> io::Result<usize> {
    Ok(find_byte(text, 0..at, true, |byte| byte == b'\n')?.map_or(0, |index| index + 1))
}

fn line_end(text: &dyn Content, at: usize) -> io::Result<usize> {
    if at > 0 && read_bytes(text, at - 1..at)? == b"\n" {
        return Ok(at);
    }
    Ok(find_byte(text, at..text.byte_len(), false, |byte| byte == b'\n')?.map_or(text.byte_len(), |index| index + 1))
}

#[cfg(test)]
#[path = "revert_tests.rs"]
mod tests;
