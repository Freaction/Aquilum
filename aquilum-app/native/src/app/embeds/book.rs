use std::path::PathBuf;

use super::media::locate;
use super::{Inner, indexed};
use crate::frontmatter;
use crate::ui::editor::resolve::{Book, Fetch};

pub(super) fn load(inner: &Inner, target: &str) -> Fetch<Book> {
    let (workspace, document) = (inner.workspace(), inner.source());
    let path = indexed(|| inner.core.search.resolve_wiki_links(&workspace, &document, &[target.to_owned()]))
        .ok()
        .and_then(|r| r.paths.into_iter().next().flatten())
        .map(PathBuf::from);
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
