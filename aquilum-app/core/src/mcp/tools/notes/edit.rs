use super::*;
use crate::files::content::FileContent;
use crate::files::error::FileCommandError;
use crate::search::headings::find_spot;
use crate::text::cancel::Cancel;
use crate::text::content::Content;
use std::io;
use std::ops::Range;

pub(super) type Changes = Vec<(Range<usize>, String)>;

pub(super) fn update_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let edit = edit_from(arguments)?;
    let version_name = optional_text(arguments, "versionName");
    let changed = gate::retrying(&note, |hash| {
        let changes = changes(&FileContent::open(&note)?, &edit).map_err(|message| FileCommandError::Task { message })?;
        match changes.is_empty() {
            true => Ok(false),
            false => gate::edit(core, &note, &changes, Some(hash), Source::Agent, version_name.as_deref()).map(|_| true),
        }
    })
    .map_err(|error| error.to_string())?;
    if !changed {
        return Ok(json!({ "path": vault.relative(&note), "changed": false }));
    }
    open_after_write(core, &vault, &note, arguments);
    let chars = Pages::open(&note).and_then(|mut pages| read_chars(&mut pages, 0..0)).map_err(|error| error.to_string())?.1;
    Ok(json!({ "path": vault.relative(&note), "changed": true, "chars": chars }))
}

pub(super) struct Edit {
    pub(super) mode: String,
    pub(super) content: String,
    pub(super) find: Option<String>,
    pub(super) heading: Option<String>,
    pub(super) all: bool,
}

pub(super) fn edit_from(arguments: &Value) -> Result<Edit, String> {
    let mode = optional_text(arguments, "mode").unwrap_or_else(|| "append".to_owned());
    let content = match mode.as_str() {
        "findReplace" | "replaceSection" => content_or_empty(arguments, "content")?,
        _ => content(arguments, "content")?,
    };
    Ok(Edit {
        mode,
        content,
        find: optional_content(arguments, "find"),
        heading: optional_text(arguments, "heading"),
        all: flag(arguments, "all", false),
    })
}

struct Ends {
    len: usize,
    leading: usize,
    trailing: usize,
    blank: bool,
}

pub(super) fn changes(text: &dyn Content, edit: &Edit) -> Result<Changes, String> {
    let content = edit.content.as_str();
    let ends = || ends(text).map_err(|error| error.to_string());
    match edit.mode.as_str() {
        "replace" => Ok(vec![(0..ends()?.len, content.to_owned())]),
        "append" => {
            let ends = ends()?;
            Ok(vec![match ends.blank {
                true => (0..ends.len, content.to_owned()),
                false => (ends.len - ends.trailing..ends.len, format!("\n\n{}", content.trim_start_matches('\n'))),
            }])
        }
        "prepend" if content.trim().is_empty() => Ok(Vec::new()),
        "prepend" => Ok(vec![(0..ends()?.leading, format!("{}\n\n", content.trim_end_matches('\n')))]),
        "findReplace" => {
            let needle = edit.find.as_deref().filter(|needle| !needle.is_empty()).ok_or("Для режима findReplace нужен аргумент «find»")?;
            let found = occurrences(text, needle).map_err(|error| error.to_string())?;
            match found.len() {
                0 => Err(format!("Текст не найден в заметке: {}", shorten(needle, 80))),
                count if count > 1 && !edit.all => Err(format!(
                    "Текст встречается в заметке больше одного раза (всего: {count}) — уточните «find» или передайте all: true"
                )),
                _ => Ok(found.into_iter().map(|range| (range, content.to_owned())).collect()),
            }
        }
        "underHeading" | "replaceSection" => {
            let heading = edit.heading.as_deref().ok_or_else(|| format!("Для режима {} нужен аргумент «heading»", edit.mode))?;
            section(text, heading, content, edit.mode == "replaceSection")
        }
        other => Err(format!("Неизвестный режим правки: {other}")),
    }
}

fn ends(text: &dyn Content) -> io::Result<Ends> {
    let mut found = Ends { len: 0, leading: 0, trailing: 0, blank: true };
    let mut pages = Pages::of(text);
    while let Some(page) = pages.next()? {
        let kept = page.text.trim_end_matches('\n');
        if page.first {
            found.leading = page.text.len() - page.text.trim_start_matches('\n').len();
        }
        found.trailing = page.text.len() - kept.len() + if kept.is_empty() { found.trailing } else { 0 };
        found.blank &= page.text.trim().is_empty();
        found.len = page.start + page.text.len();
    }
    Ok(found)
}

fn occurrences(text: &dyn Content, needle: &str) -> io::Result<Vec<Range<usize>>> {
    let reach = needle.len() - 1;
    let (mut found, mut tail, mut pages) = (Vec::new(), String::new(), Pages::of(text));
    while let Some(page) = pages.next()? {
        let text = page.text.as_str();
        let head = text.ceil_char_boundary(reach);
        let seam = page.start - tail.len();
        tail.push_str(&text[..head]);
        collect(&tail, seam, page.start, needle, &mut found);
        collect(text, page.start, usize::MAX, needle, &mut found);
        let from = text.floor_char_boundary(text.len().saturating_sub(reach));
        tail = text[from..].to_owned();
    }
    Ok(found)
}

fn collect(text: &str, offset: usize, starts_before: usize, needle: &str, found: &mut Vec<Range<usize>>) {
    let skip = found.last().map_or(0, |last| last.end).saturating_sub(offset).min(text.len());
    let starts = text[skip..].match_indices(needle).map(|(at, _)| offset + skip + at).take_while(|&at| at < starts_before);
    found.extend(starts.map(|at| at..at + needle.len()));
}

fn section(text: &dyn Content, heading: &str, content: &str, replace: bool) -> Result<Changes, String> {
    let failed = |error: io::Error| error.to_string();
    let (outline, _) = headings::outline(&mut Pages::of(text)).map_err(failed)?;
    let section = single_heading(&outline, heading)?.content.clone();
    let kept = if replace { section.end } else { section.start };
    let mut pages = Pages::of(text);
    let spot = find_spot(&mut pages, &Cancel::never(), |page| (page.start..page.start + page.text.len()).contains(&kept).then(|| kept - page.start)).map_err(failed)?;
    let (newlines, more) = match spot {
        Some(spot) => {
            let rest = &spot.page.text[spot.at..];
            let tail = rest.trim_start_matches('\n');
            (rest.len() - tail.len(), !tail.is_empty() || !pages.finished())
        }
        None => (0, false),
    };
    let body = content.trim_matches('\n');
    let mut insert = if body.is_empty() { String::new() } else { format!("\n{body}\n") };
    if more {
        insert.push('\n');
    }
    Ok(vec![(section.start..kept + newlines, insert)])
}
