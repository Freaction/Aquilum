use crate::search::paths::{relative_slash_path, strip_markdown_extension};
use std::ops::Range;
use std::path::{Path, PathBuf};

const ATTACHMENTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "avif", "ico", "tif", "tiff", "heic", "mp3",
    "wav", "m4a", "ogg", "oga", "flac", "opus", "aac", "mp4", "webm", "mkv", "mov", "avi", "m4v",
    "ogv", "pdf", "epub", "fb2", "mobi", "azw", "azw3", "prc", "djvu", "cbz", "cbr", "zip", "docx",
    "xlsx", "pptx",
];

pub struct Parts {
    pub note: Range<usize>,
    pub anchor: Option<Range<usize>>,
    pub label: Option<Range<usize>>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Target<'a> {
    pub note: &'a str,
    pub anchor: Option<&'a str>,
    pub label: Option<&'a str>,
}

fn trimmed(raw: &str, range: Range<usize>) -> Range<usize> {
    let part = &raw[range.clone()];
    let start = range.start + part.len() - part.trim_start().len();
    start..(range.end - (part.len() - part.trim_end().len())).max(start)
}

pub fn parts(raw: &str) -> Parts {
    let (head, label) = match raw.find('|') {
        Some(at) => {
            let end = if raw[..at].ends_with('\\') { at - 1 } else { at };
            (0..end, Some(trimmed(raw, at + 1..raw.len())))
        }
        None => (0..raw.len(), None),
    };
    let (note, anchor) = match raw[head.clone()].find('#') {
        Some(at) => (
            head.start..head.start + at,
            Some(trimmed(raw, head.start + at + 1..head.end)),
        ),
        None => (head, None),
    };
    Parts {
        note: trimmed(raw, note),
        anchor: anchor.filter(|range| !range.is_empty()),
        label: label.filter(|range| !range.is_empty()),
    }
}

pub fn split(raw: &str) -> Target<'_> {
    let parts = parts(raw);
    Target {
        note: &raw[parts.note],
        anchor: parts.anchor.map(|range| &raw[range]),
        label: parts.label.map(|range| &raw[range]),
    }
}

pub fn is_attachment(note: &str) -> bool {
    Path::new(note)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ATTACHMENTS
                .iter()
                .any(|attachment| extension.eq_ignore_ascii_case(attachment))
        })
}

pub fn target_key(target: &str) -> (String, &'static str) {
    let note = split(target).note.replace('\\', "/");
    let note = strip_markdown_extension(&note);
    (
        note.to_lowercase(),
        if note.contains('/') { "path" } else { "name" },
    )
}

pub fn relative_key(root: &Path, path: &Path) -> String {
    strip_markdown_extension(&relative_slash_path(root, path)).to_lowercase()
}

pub fn title_key(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}

pub fn relative_target(root: &Path, path: &Path, path_style: bool) -> String {
    if !path_style {
        return path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
    }
    strip_markdown_extension(&relative_slash_path(root, path)).to_owned()
}

pub fn select_candidate(root: &Path, source: &Path, paths: &[String]) -> Option<PathBuf> {
    let borrowed = paths.iter().map(String::as_str).collect::<Vec<_>>();
    pick_candidate(root, source, &borrowed).map(PathBuf::from)
}

pub fn pick_candidate<'a>(
    root: &Path,
    source: &Path,
    paths: &[&'a str],
) -> Option<&'a str> {
    let parent = source.parent();
    paths.iter().copied().min_by(|left, right| {
        candidate_rank(root, parent, left)
            .cmp(&candidate_rank(root, parent, right))
            .then_with(|| left.cmp(right))
    })
}

fn candidate_rank(root: &Path, parent: Option<&Path>, value: &str) -> (bool, usize) {
    let path = Path::new(value);
    (
        path.parent() != parent,
        path.strip_prefix(root).unwrap_or(path).components().count(),
    )
}

#[cfg(test)]
mod tests {
    use super::{is_attachment, split, target_key, Target};

    fn target<'a>(note: &'a str, anchor: Option<&'a str>, label: Option<&'a str>) -> Target<'a> {
        Target { note, anchor, label }
    }

    #[test]
    fn splits_note_anchor_and_label() {
        assert_eq!(split("Заметка"), target("Заметка", None, None));
        assert_eq!(split("  Заметка # Заголовок | подпись "), target("Заметка", Some("Заголовок"), Some("подпись")));
        assert_eq!(split("#Заголовок"), target("", Some("Заголовок"), None));
        assert_eq!(split("Заметка#^abc123"), target("Заметка", Some("^abc123"), None));
        assert_eq!(split("Папка/Заметка#А#Б"), target("Папка/Заметка", Some("А#Б"), None));
        assert_eq!(split("image.png|610"), target("image.png", None, Some("610")));
        assert_eq!(split("document.pdf#page=3"), target("document.pdf", Some("page=3"), None));
        assert_eq!(split("Заметка#|подпись"), target("Заметка", None, Some("подпись")));
        assert_eq!(split("Заметка|"), target("Заметка", None, None));
    }

    #[test]
    fn escaped_pipe_separates_the_label() {
        assert_eq!(split(r"Заметка\|подпись"), target("Заметка", None, Some("подпись")));
        assert_eq!(split(r"Заметка#Заголовок\|подпись"), target("Заметка", Some("Заголовок"), Some("подпись")));
    }

    #[test]
    fn keys_ignore_anchor_label_and_markdown_extension() {
        assert_eq!(target_key("Заметка#Заголовок"), ("заметка".to_owned(), "name"));
        assert_eq!(target_key(r"Папка\Заметка.MD#h|x"), ("папка/заметка".to_owned(), "path"));
        assert_eq!(target_key("#Заголовок"), (String::new(), "name"));
    }

    #[test]
    fn attachments_are_recognised_by_extension_only() {
        assert!(is_attachment("image.PNG"));
        assert!(is_attachment("Files/document.pdf"));
        assert!(!is_attachment("Research.v2"));
        assert!(!is_attachment("v1.2 план"));
        assert!(!is_attachment("Заметка"));
    }
}
