use std::path::PathBuf;

use super::media::locate;
use super::{Inner, indexed, settle};
use crate::frontmatter;
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
    let title = target.trim_end_matches(".md").to_owned();
    let Some(path) = path else {
        return Fetch::Ready(Book { title, linked: false, author: String::new(), cover: None, pages: None, file: None, page: None });
    };
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let fields = frontmatter::parse(&text).unwrap_or_default();
    let cover = fields.text(frontmatter::BOOK_COVER).and_then(|c| locate(inner, c)).and_then(|p| crate::images::load(&p));
    let pages = fields.text("pages").and_then(|p| p.split_once('/')).and_then(|(a, b)| Some((a.trim().parse().ok()?, b.trim().parse().ok()?)));
    Fetch::Ready(Book {
        title,
        linked: true,
        author: fields.text("author").unwrap_or_default().to_owned(),
        cover,
        pages,
        file: fields.text(frontmatter::BOOK_FILE).and_then(|f| locate(inner, f)),
        page: Some(path),
    })
}
