use std::path::PathBuf;

use aquilum_core::files::document::read_text;
use aquilum_core::search::paths::strip_markdown_extension;

use super::media::locate;
use super::{Inner, indexed, settle};
use crate::frontmatter::{self, AUTHOR, BOOK_COVER, BOOK_FILE, PAGES};
use crate::ui::editor::resolve::{Book, Fetch};

pub(super) fn drain(inner: &Inner) {
    loop {
        let batch = {
            let mut queue = inner.book_queue.lock().unwrap_or_else(|e| e.into_inner());
            if queue.pending.is_empty() {
                queue.running = false;
                return;
            }
            std::mem::take(&mut queue.pending)
        };
        let targets: Vec<String> = batch.iter().map(|(target, _)| target.clone()).collect();
        let (workspace, document) = (inner.workspace(), inner.source());
        let mut paths: Vec<Option<PathBuf>> = indexed(|| inner.core.search.resolve_wiki_links(&workspace, &document, &targets))
            .map(|r| r.paths.into_iter().map(|p| p.map(PathBuf::from)).collect())
            .unwrap_or_default();
        paths.resize(targets.len(), None);
        let mut changed = false;
        for ((target, epoch), path) in batch.into_iter().zip(paths) {
            changed |= settle(&inner.books, &target, epoch, load(inner, &target, path));
        }
        if changed {
            inner.touch();
        }
    }
}

fn load(inner: &Inner, target: &str, path: Option<PathBuf>) -> Fetch<Book> {
    let title = strip_markdown_extension(target).to_owned();
    let Some(path) = path else {
        return Fetch::Ready(Book { title, linked: false, author: String::new(), cover: None, pages: None, file: None, page: None });
    };
    let text = read_text(&path).unwrap_or_default();
    let fields = frontmatter::parse(&text).unwrap_or_default();
    let cover = fields.text(BOOK_COVER).and_then(|c| locate(inner, c)).and_then(|p| crate::images::load(&p));
    Fetch::Ready(Book {
        title,
        linked: true,
        author: fields.text(AUTHOR).unwrap_or_default().to_owned(),
        cover,
        pages: fields.text(PAGES).and_then(frontmatter::parse_pages),
        file: fields.text(BOOK_FILE).and_then(|f| locate(inner, f)),
        page: Some(path),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aquilum_core::Core;

    use super::super::Embeds;
    use crate::ui::editor::resolve::Resolver;

    #[test]
    fn crlf_book_note_keeps_author_pages_cover_and_file() {
        let vault = crate::test_dir::tempdir();
        let data = crate::test_dir::tempdir();
        let mut png = Vec::new();
        image::RgbaImage::new(4, 3).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        std::fs::write(vault.path().join("cover.png"), png).unwrap();
        std::fs::write(vault.path().join("book.epub"), "").unwrap();
        std::fs::write(vault.path().join("📖 Книга.MD"), "---\r\nauthor: Автор\r\npages: 3/10\r\nBook_cover: cover.png\r\nПуть к файлу: book.epub\r\n---\r\nтекст\r\n").unwrap();
        let document = vault.path().join("Главная.md");
        std::fs::write(&document, "> [!book] [[📖 Книга]]").unwrap();
        let core = Core::open(data.path(), Arc::new(|_| {}));
        let workspace = vault.path().to_string_lossy().into_owned();
        core.search.prepare(&workspace).unwrap();
        for _ in 0..300 {
            let status = core.search.status(Some(&workspace));
            if status.state == aquilum_core::search::models::SearchIndexState::Ready && !status.updating {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let embeds = Embeds::new(Arc::clone(&core), vault.path().to_path_buf(), document, data.path().join("cache"), Arc::new(|| {}));
        let mut book = None;
        for _ in 0..300 {
            embeds.book("📖 Книга");
            embeds.take_changed();
            book = embeds.cached_book("📖 Книга");
            if book.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let book = book.expect("книга не загрузилась");
        assert!(book.linked);
        assert_eq!(book.author, "Автор");
        assert_eq!(book.pages, Some((3, 10)));
        assert!(book.cover.is_some());
        assert!(book.file.is_some_and(|file| file.ends_with("book.epub")));
        core.shutdown();
    }
}
