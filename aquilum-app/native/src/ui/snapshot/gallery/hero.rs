use super::Scene;
use crate::ui::book::{BookView, HeroView, PageCover, Visual};
use crate::ui::note::{self, NoteView};
use crate::ui::snapshot::{H, open_parts, parts};

pub(super) fn snapshots(scene: &Scene) {
    for (name, pattern, book) in [("book", true, true), ("cover", true, false), ("photo", false, true)] {
        let mut p = parts("");
        let visual = if pattern { Visual::Pattern("tunnel") } else { Visual::Image(crate::images::default_book_cover()) };
        let (page, ids) = note::page(NoteView {
            file_name: "Дюна.md",
            title: "Дюна",
            body: "Фрэнк Герберт, 1965.",
            metadata: Some(false),
            can_back: true,
            can_forward: false,
            focus_mode: false,
            hero: Some(HeroView {
                cover: Some(visual),
                position: 30.0,
                book: book.then(|| BookView { cover: crate::images::default_book_cover(), has_file: false }),
                viewport: f64::from(H),
            }),
            resolver: None,
            smart: Default::default(),
        });
        p.note = page.erased();
        let (mut root, _) = open_parts(Some(scene.vault), scene.data, p);
        let _ = root.redraw();
        for _ in 0..500 {
            if crate::cover::take_finished() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if let Some(cover) = ids.hero.as_ref().and_then(|(_, hero)| hero.cover) {
            cover.edit(&mut root, |mut cover| PageCover::refresh(&mut cover));
        }
        scene.save(&mut root, &format!("hero-{name}"));
    }
}
