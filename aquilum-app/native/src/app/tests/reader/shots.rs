use super::*;

#[test]
#[ignore]
fn reader_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let book = std::path::PathBuf::from(std::env::var("AQ_BOOK").unwrap());
    let mut h = Harness::new();
    let note = h.vault.path().join("a.md");
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    open_book(&mut h, &book, None, &book.file_stem().unwrap().to_string_lossy());
    let size = h.root.size();
    let (w, hh) = (size.width as u16, size.height as u16);
    for i in 0..4 {
        let started = std::time::Instant::now();
        let shot = h.frame(w, hh, 1.0);
        std::fs::write(out.join(format!("reader-{i}.png")), shot.png()).unwrap();
        eprintln!("страница {i}: {:?}", started.elapsed());
        turn_pages(&mut h, 3);
    }
    h.drag((120.0, 160.0), (420.0, 220.0));
    let _ = h.root.redraw();
    h.deliver();
    std::fs::write(out.join("reader-selection.png"), h.frame(w, hh, 1.0).png()).unwrap();
    h.click(936.0, 30.0);
    let _ = h.root.redraw();
    h.deliver();
    std::fs::write(out.join("reader-settings.png"), h.frame(w, hh, 1.0).png()).unwrap();
}

#[test]
#[ignore]
fn quote_links_open_reader_at_quote() {
    let source_note = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    let source_vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let doc = std::fs::read_to_string(&source_note).unwrap();
    let hrefs = || doc.lines().filter_map(|l| l.rsplit_once("](")).map(|(_, href)| href.trim_end().trim_end_matches(')').to_owned());
    let book = crate::frontmatter::parse(&doc)
        .and_then(|f| f.text(crate::frontmatter::BOOK_FILE).map(str::to_owned))
        .or_else(|| hrefs().find_map(|href| crate::links::reader::parse_href(&href).and_then(|(_, book)| book)))
        .unwrap();
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join(&book).parent().unwrap()).unwrap();
    std::fs::copy(source_vault.join(&book), vault.join(&book)).unwrap();
    let note = h.write_note(&source_note.file_name().unwrap().to_string_lossy(), &doc);
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    for (n, line) in doc.lines().filter(|l| crate::links::reader::is_reader(l.rsplit_once("](").map_or("", |(_, href)| href))).enumerate() {
        let href = line.rsplit_once("](").unwrap().1.trim_end().trim_end_matches(')');
        assert!(h.app.open_reader_link(&mut h.root, href), "ссылка не открыла читалку: {href}");
        h.deliver();
        assert!(h.reader(), "читалка не открылась");
        h.settle();
        h.settle();
        std::fs::write(out.join(format!("quote-{n}.png")), h.frame(W as u16, H as u16, 1.0).png()).unwrap();        h.escape();
    }
}
