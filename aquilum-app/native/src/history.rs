use std::path::Path;

use aquilum_core::Core;
use aquilum_core::documents::merge::line_diff::{line_diff, split_lines};
use aquilum_core::documents::merge::merge_external_change;
use aquilum_core::files::document::read_file_snapshot_impl;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate;
use aquilum_core::history::Source;
use aquilum_core::history::store::VersionFile;
use chrono::Datelike;

use crate::dates;
use crate::i18n::{t, t_with};

pub const PAGE_SIZE: usize = 50;
const WRITE_RETRIES: usize = 3;

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

fn source_key(source: &Source) -> &'static str {
    match source {
        Source::Me => "me",
        Source::Agent => "agent",
        Source::External => "external",
        Source::Links => "links",
        Source::Start | Source::Reading => "start",
        Source::Restore(_) => "restore",
        Source::Revert(_) => "revert",
    }
}

pub fn label(version: &Version) -> String {
    version.name.clone().unwrap_or_else(|| t(&format!("history.sources.{}", source_key(&version.source))))
}

pub fn title(version: &Version) -> String {
    match version.from_ms {
        None => t(&format!("history.sources.{}", source_key(&version.source))),
        Some(from) => {
            let key = if matches!(version.source, Source::Revert(_)) { "history.revertedFrom" } else { "history.restoredFrom" };
            t_with(key, &[("date", &dates::date_time(from))])
        }
    }
}

pub fn heading(version: &Version) -> String {
    let named = version.name.as_ref().map(|n| format!("{n} · ")).unwrap_or_default();
    format!("{named}{} · {}", title(version), dates::date_time(version.at_ms))
}

pub fn day_label(at_ms: u64, now_ms: u64) -> String {
    let (Some(at), Some(now)) = (dates::local(at_ms), dates::local(now_ms)) else { return String::new() };
    let (day, today) = (at.date_naive(), now.date_naive());
    if day == today {
        t("history.today")
    } else if today.pred_opt() == Some(day) {
        t("history.yesterday")
    } else {
        dates::day(at_ms, at.year() != now.year())
    }
}

pub fn group_by_day(versions: &[Version], now_ms: u64) -> Vec<(String, Vec<&Version>)> {
    let mut groups: Vec<(chrono::NaiveDate, String, Vec<&Version>)> = Vec::new();
    for version in versions {
        let Some(at) = dates::local(version.at_ms) else { continue };
        let date = at.date_naive();
        match groups.last_mut() {
            Some((day, _, list)) if *day == date => list.push(version),
            _ => groups.push((date, day_label(version.at_ms, now_ms), vec![version])),
        }
    }
    groups.into_iter().map(|(_, label, list)| (label, list)).collect()
}

pub struct Texts {
    pub text: String,
    pub previous: Option<String>,
}

pub fn texts(core: &Core, note: &Path, id: &str) -> Option<Texts> {
    let texts = core.history.version(&|| core.known_roots(), note, Some(id))?;
    Some(Texts { text: texts.text, previous: texts.previous })
}

fn rewrite(core: &Core, note: &Path, source: Source, next: impl Fn(&str) -> String) -> Result<bool, FileCommandError> {
    let mut conflicts = 0;
    loop {
        let snapshot = read_file_snapshot_impl(note)?;
        let content = next(&snapshot.content);
        if content == snapshot.content {
            return Ok(false);
        }
        match gate::write(core, note, &content, Some(&snapshot.hash), source, None) {
            Ok(_) => return Ok(true),
            Err(FileCommandError::Conflict { .. }) if conflicts < WRITE_RETRIES => conflicts += 1,
            Err(error) => return Err(error),
        }
    }
}

pub fn restore(core: &Core, note: &Path, version: &Version) -> Result<bool, FileCommandError> {
    let Some(texts) = texts(core, note, &version.id) else { return Ok(false) };
    rewrite(core, note, Source::Restore(version.at_ms), |_| texts.text.clone())
}

pub fn revert(core: &Core, note: &Path, version: &Version) -> Result<bool, FileCommandError> {
    let Some(Texts { text, previous: Some(previous) }) = texts(core, note, &version.id) else { return Ok(false) };
    rewrite(core, note, Source::Revert(version.at_ms), |current| {
        let merged = merge_external_change(&text, &previous, current);
        apply_utf16_edits(current, &merged.edits)
    })
}

fn byte_offset(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (index, ch) in text.char_indices() {
        if units >= utf16 {
            return index;
        }
        units += ch.len_utf16();
    }
    text.len()
}

fn apply_utf16_edits(text: &str, edits: &[aquilum_core::documents::merge::TextEdit]) -> String {
    let mut result = text.to_owned();
    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by(|a, b| b.from.cmp(&a.from));
    for edit in sorted {
        let from = byte_offset(&result, edit.from);
        let to = byte_offset(&result, edit.to);
        result.replace_range(from..to, &edit.insert);
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Same,
    Added,
    Removed,
}

pub fn diff(previous: Option<&str>, text: &str) -> Vec<(Change, String)> {
    let next = split_lines(text);
    let Some(previous) = previous else {
        return next.into_iter().map(|line| (Change::Same, line.to_owned())).collect();
    };
    let base = split_lines(previous);
    let hunks = line_diff(&base, &next).hunks;
    let mut lines = Vec::new();
    let mut at = 0;
    for hunk in hunks {
        lines.extend(base[at..hunk.from].iter().map(|line| (Change::Same, (*line).to_owned())));
        lines.extend(base[hunk.from..hunk.to].iter().map(|line| (Change::Removed, (*line).to_owned())));
        lines.extend(hunk.lines.into_iter().map(|line| (Change::Added, line)));
        at = hunk.to;
    }
    lines.extend(base[at..].iter().map(|line| (Change::Same, (*line).to_owned())));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_marks_added_and_removed_lines() {
        let lines = diff(Some("a\nb\nc"), "a\nB\nc\nd");
        let kinds: Vec<_> = lines.iter().map(|(k, l)| (*k, l.as_str())).collect();
        assert_eq!(
            kinds,
            [(Change::Same, "a"), (Change::Removed, "b"), (Change::Added, "B"), (Change::Same, "c"), (Change::Added, "d")]
        );
    }

    #[test]
    fn utf16_edits_apply_to_cyrillic() {
        let edits = [aquilum_core::documents::merge::TextEdit { from: 4, to: 6, insert: "ЁЖ".into() }];
        assert_eq!(apply_utf16_edits("абв гд е", &edits), "абв ЁЖ е");
    }

    #[test]
    fn restore_and_revert_rewrite_note() {
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let core = Core::open(data.path(), std::sync::Arc::new(|_| {}));
        core.ui_state.resolve_workspace(&vault.path().to_string_lossy(), 0).unwrap();
        let note = vault.path().join("a.md");
        std::fs::write(&note, "один\nдва\n").unwrap();
        for (text, source) in [("один\nдва\nтри\n", Source::Me), ("ноль\nодин\nдва\nтри\n", Source::Agent)] {
            let hash = read_file_snapshot_impl(&note).unwrap().hash;
            gate::write(&core, &note, text, Some(&hash), source, None).unwrap();
        }
        let versions = page(&core, &note, PAGE_SIZE).versions;
        let with_three = versions
            .iter()
            .find(|v| texts(&core, &note, &v.id).is_some_and(|t| t.text == "один\nдва\nтри\n"))
            .unwrap_or_else(|| panic!("версия с «три»; версий {}", versions.len()));
        assert!(revert(&core, &note, with_three).unwrap());
        assert_eq!(std::fs::read_to_string(&note).unwrap(), "ноль\nодин\nдва\n");
        assert!(restore(&core, &note, with_three).unwrap());
        assert_eq!(std::fs::read_to_string(&note).unwrap(), "один\nдва\nтри\n");
        core.shutdown();
    }
}
