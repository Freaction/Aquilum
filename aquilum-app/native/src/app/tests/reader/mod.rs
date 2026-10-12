use super::*;

mod pages;
mod quotes;
mod shots;

fn sample_fb2() -> String {
    let paragraphs: String = (1..=400).map(|i| format!("<p>Абзац номер {i}. Съешь же ещё этих мягких французских булок да выпей чаю, повторяя это снова и снова.</p>")).collect();
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\"><body><section><title><p>Глава первая</p></title>{paragraphs}</section><section><title><p>Глава вторая</p></title>{paragraphs}</section></body></FictionBook>")
}

impl Harness {
    pub(super) fn reader(&self) -> bool {
        matches!(&self.app.popup, Some(Popup::Reader(_)))
    }
}

fn book_note(h: &Harness, tail: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    std::fs::create_dir_all(h.vault.path().join("Files")).unwrap();
    let file = h.vault.path().join("Files").join("книга.fb2");
    std::fs::write(&file, sample_fb2()).unwrap();
    let page = h.write_note("📖 Книга.md", &format!("---\nПуть к файлу: Files/книга.fb2\npages:\n---\n{tail}"));
    (file, page)
}

fn open_book(h: &mut Harness, file: &std::path::Path, page: Option<&std::path::Path>, title: &str) {
    let opening = crate::app::reader::Opening { file: file.to_path_buf(), page: page.map(std::path::Path::to_path_buf), title: title.into(), initial: None, peek: false };
    h.app.open_reader(&mut h.root, opening);
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
}

fn turn_pages(h: &mut Harness, count: usize) {
    for _ in 0..count {
        h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
        let _ = h.root.redraw();
        h.deliver();
    }
}
