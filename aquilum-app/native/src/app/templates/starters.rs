use std::path::{Path, PathBuf};

use aquilum_core::Core;
use aquilum_core::files::document::read_text;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

use super::apply_placeholders;
use crate::frontmatter::{self, AUTHOR, BOOK_COVER, BOOK_FILE, BOOK_TYPE, COVER, CREATED, PAGE_COVER, PAGES, RATING, READ_PERCENT, READER_POSITION, STATUS, TAGS, TYPE};

const CREATED_PLACEHOLDER: &str = "{{date:DD-MM-YYYY}} | ({{time:HH:mm}})";
const NOTE_TAIL: &str = "\n\n\n\n***\n\nСсылки:\n- [[]]\n";
const BOOK_TAIL: &str = "\n\n\n***\n\nСсылки:\n- [[]]\n";

fn note_starter() -> String {
    let fields = [(CREATED, CREATED_PLACEHOLDER), (TAGS, "[]"), (AUTHOR, ""), ("source", "")];
    frontmatter::template(&fields) + NOTE_TAIL
}

fn book_starter() -> String {
    let fields = [
        (CREATED, CREATED_PLACEHOLDER),
        (COVER, "true"),
        (TYPE, BOOK_TYPE),
        (AUTHOR, ""),
        (STATUS, "to-read"),
        (PAGES, ""),
        (TAGS, "[книга, ]"),
        (RATING, "0"),
        (BOOK_COVER, ""),
        (PAGE_COVER, "pattern:tunnel"),
        (BOOK_FILE, ""),
        (READER_POSITION, ""),
        (READ_PERCENT, ""),
    ];
    frontmatter::template(&fields) + BOOK_TAIL
}

pub fn template_text(path: &Path) -> String {
    apply_placeholders(&read_text(path).unwrap_or_default())
}

pub fn write_starters(core: &Core, folder: &Path) -> Vec<PathBuf> {
    if let Err(error) = std::fs::create_dir_all(folder) {
        report!("папка шаблонов не создана: {error}");
        return Vec::new();
    }
    let mut created = Vec::new();
    for (name, content) in [("Note.md", note_starter()), ("Book.md", book_starter())] {
        let path = folder.join(name);
        if path.exists() {
            continue;
        }
        match gate::create(core, &path, &content, Source::Me, None) {
            Ok(_) => created.push(path),
            Err(error) => report!("{} не создан: {error:?}", path.display()),
        }
    }
    created
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starters_keep_their_text_and_have_valid_frontmatter() {
        assert_eq!(note_starter(), "---\ncreated: {{date:DD-MM-YYYY}} | ({{time:HH:mm}})\ntags: []\nauthor: \nsource: \n---\n\n\n\n\n***\n\nСсылки:\n- [[]]\n");
        let book = book_starter();
        assert_eq!(book, "---\ncreated: {{date:DD-MM-YYYY}} | ({{time:HH:mm}})\ncover: true\ntype: book\nauthor: \nstatus: to-read\npages: \ntags: [книга, ]\nrating: 0\nBook_cover: \nPage_cover: pattern:tunnel\nПуть к файлу: \nreader_position: \nread_percent: \n---\n\n\n\n***\n\nСсылки:\n- [[]]\n");
        assert!(frontmatter::end(&book).is_some_and(|end| book[end..].starts_with("\n\n\n\n***")));
        assert!(frontmatter::parse(&book).is_some_and(|fields| fields.is_book() && fields.has_page_cover()));
    }

    #[test]
    fn crlf_templates_reach_the_editor_without_carriage_returns() {
        let dir = crate::test_dir::tempdir();
        let path = dir.path().join("Книга.md");
        std::fs::write(&path, "---\r\ntype: book\r\n---\r\n\r\nСсылки:\r\n- [[]]\r\n").unwrap();
        let text = template_text(&path);
        assert!(!text.contains('\r'));
        assert!(frontmatter::parse(&text).is_some_and(|fields| fields.is_book()));
    }
}
