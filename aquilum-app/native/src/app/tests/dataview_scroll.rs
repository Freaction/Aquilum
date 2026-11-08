use masonry::core::{PointerScrollEvent, ScrollDelta};

use super::*;
use crate::ui::editor::TextEditor;
use crate::ui::scroll::bar::{Scrollbar, Span};

pub(super) fn bar(h: &Harness) -> Option<Span> {
    let editor = h.app.editor_id()?;
    h.root.get_widget(editor)?.downcast::<TextEditor>()?.inner().card_scrollbar()
}

pub(super) fn wheel(h: &mut Harness, delta: ScrollDelta, modifiers: Modifiers, scale: f64) {
    let area = h.root.get_widget(h.app.editor_id().unwrap()).unwrap().ctx().bounding_box();
    let mut state = pointer_at((area.x0 + 120.0) * scale, (area.y0 + 40.0) * scale, scale);
    state.modifiers = modifiers;
    h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta, state }));
    h.step();
}

pub(super) fn open_wide_table(mut h: Harness) -> Harness {
    let fields = |n: usize| ["a", "b", "c", "d", "e", "f", "g", "h"].map(|k| format!("{k}: значение поля {k} заметки номер {n}")).join("\n");
    for n in 0..3 {
        h.write_note(&format!("Строка {n}.md"), &format!("---\n{}\n---\nтекст", fields(n)));
    }
    let note = h.write_note("Главная.md", "```dataview\nTABLE a, b, c, d, e, f, g, h\n```\n\nниже\n");
    h.open(note);
    assert!(h.wait_for(|h| bar(h).is_some()), "широкая таблица датавью не получила прокрутку");
    h
}

#[test]
fn home_progress_table_fits_a_phone_column() {
    let scale = 2.625;
    let mut h = Harness::with(None, scale);
    let query = "TABLE WITHOUT ID item.name AS \"Период\", bar(item.span) AS \"Прогресс\", remaining(item.span) AS \"Осталось\"\nFROM once\nFLATTEN list(object(\"name\", \"До конца работы\", \"span\", day), object(\"name\", \"Неделя\", \"span\", week), object(\"name\", \"Год\", \"span\", year)) AS item";
    let note = h.write_note("Домашняя.md", &format!("```dataview\n{query}\n```\n"));
    h.open(note);
    assert!(h.wait_for(|h| h.app.embeds.as_ref().unwrap().cached_rows(query) == Some(3)), "таблица не загрузилась");
    for width in [360, 800, 360] {
        let pixels = (f64::from(width) * scale) as u16;
        h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(u32::from(pixels), 1600)));
        h.app.window_resized(&mut h.root, Size::new(f64::from(width), 1600.0 / scale));
        h.settle();
        assert!(bar(&h).is_none(), "прогресс вызвал горизонтальную прокрутку при ширине {width}: {:?}", bar(&h).map(|b| b.max()));
        if std::env::var_os("AQ_OUT").is_some() {
            dump(&format!("dataview-progress-{width}.png"), &h.frame(pixels, 1600, scale));
        }
    }
}

#[test]
fn the_scrollbar_and_the_wheel_move_a_dataview_table_in_the_real_window() {
    let mut h = open_wide_table(Harness::new());
    let (before, text) = (bar(&h).unwrap(), h.body_text());
    assert!(before.max() > 100.0, "прокручивать почти нечего");
    let thumb = Scrollbar::new(masonry::kurbo::Axis::Horizontal).thumb(&before);
    let origin = h.root.get_widget(h.app.editor_id().unwrap()).unwrap().ctx().bounding_box().origin();
    let from = (origin.x + thumb.center().x, origin.y + thumb.center().y);
    h.pointer_move(from.0, from.1);
    h.step();
    h.drag(from, (from.0 + 120.0, from.1 + 6.0));
    h.step();
    let dragged = bar(&h).unwrap();
    assert!(dragged.offset > 0.0, "полоса таблицы не тянется мышью");
    assert_eq!(dragged.track, before.track, "таблица сдвинула строки редактора");
    assert_eq!(h.body_text(), text, "перетаскивание полосы изменило текст заметки");
    h.pointer_move(from.0 + 300.0, from.1);
    assert_eq!(bar(&h).unwrap().offset, dragged.offset, "после отпускания полоса продолжила тянуться");
    wheel(&mut h, ScrollDelta::LineDelta(0.0, 1.0), Modifiers::SHIFT, 1.0);
    let left = bar(&h).unwrap().offset;
    assert!(left < dragged.offset, "Shift и колесо не сдвинули таблицу");
    wheel(&mut h, ScrollDelta::LineDelta(-1.0, 0.0), Modifiers::empty(), 1.0);
    assert!(bar(&h).unwrap().offset > left, "горизонтальное колесо не сдвинуло таблицу");
    wheel(&mut h, ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -60.0)), Modifiers::SHIFT, 1.0);
    assert!(bar(&h).unwrap().offset > left, "Shift и колесо тачпада не сдвинули таблицу");
}
