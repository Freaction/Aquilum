use super::*;
use crate::links::{Mode, Target};
use crate::ui::widgets::{ItemAction, Pressed};

pub(super) fn mode_button(h: &Harness, mode: Mode) -> masonry::core::WidgetId {
    h.app.chrome.as_ref().unwrap().panel_parts.modes.iter().find(|(_, m)| *m == mode).unwrap().0.id()
}

#[test]
fn outline_jumps_to_headings_follows_edits_and_resets_with_the_note() {
    let mut h = Harness::new();
    let body = format!("# Первая\n{}## Вторая\nтекст\n", "строка\n".repeat(200));
    let note = h.write_note("a.md", &body);
    h.open(note);
    h.send(mode_button(&h, Mode::Outline), Pressed);
    let listing = h.wait_listing();
    let rows: Vec<_> = listing.items.iter().map(|item| (item.title.as_str(), item.depth)).collect();
    assert_eq!(rows, [("Первая", 0), ("Вторая", 1)]);

    let second = body.find("## Вторая").unwrap();
    let row = h.app.panel_body.as_ref().unwrap().items[1].0;
    crate::ui::motion::set(false);
    h.act(row, ItemAction::Open { new_tab: false });
    let flashed = h.app.note_ids.as_ref().unwrap().body.get(&h.root).unwrap().inner().flashed();
    crate::ui::motion::set(true);
    assert_eq!(caret(&h), second);
    assert_eq!(flashed, Some(second..second + "## Вторая".len()), "заголовок не подсвечен без анимаций");
    assert!(note_offset(&h) > 0.0, "заметка не прокрутилась к заголовку");

    let intro = "Вступление\n";
    h.replace_body(&format!("{intro}{body}"));
    let listing = h.wait_listing();
    assert!(matches!(&listing.items[1].target, Target::Spot(spot) if spot.start == intro.len() + second));
    assert_eq!(h.app.panel_body.as_ref().unwrap().items[1].0, row, "после правки строки оглавления перестроились");

    let other = h.write_note("b.md", "без заголовков");
    h.open(other);
    let listing = h.wait_listing();
    assert!(listing.items.is_empty() && listing.notice.is_some());
}
