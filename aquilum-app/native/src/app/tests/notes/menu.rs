use masonry::core::WidgetId;
use masonry::kurbo::{Point, Rect};

use super::*;
use crate::app::editor_requests::EditorCommand::{self, *};
use crate::ui::editor::TextEditor;
use crate::ui::keymap::Command;
use crate::ui::tokens::size;

const NOTE: &str = "первое слово\n\nвторая строка\n\n";

fn caret_at(h: &mut Harness, offset: usize) -> Rect {
    let body = h.app.note_ids.as_ref().unwrap().body;
    body.edit(&mut h.root, |mut editor| TextEditor::select_byte_range(&mut editor, offset, offset));
    let _ = h.root.redraw();
    let editor = body.get(&h.root).unwrap();
    editor.ctx().window_transform().transform_rect_bbox(editor.inner().caret_rect())
}

pub(super) fn press(h: &mut Harness, at: Point, button: PointerButton, count: u8) {
    let pointer = if button == PointerButton::Secondary { MOUSE } else { TOUCH };
    h.press(PointerState { count, ..pointer_at(at.x, at.y, 1.0) }, button, pointer);
}

pub(super) fn menu(h: &Harness) -> Option<(Rect, Vec<EditorCommand>)> {
    let (layer, _, commands) = h.app.editor_menu?;
    Some((h.root.get_widget(layer)?.ctx().bounding_box(), commands.to_vec()))
}

pub(super) fn choose(h: &mut Harness, item: usize) {
    let (layer, ..) = h.app.editor_menu.expect("меню открыто");
    let mut list = h.root.get_widget(layer).unwrap();
    while list.children().len() == 1 {
        list = list.children()[0];
    }
    let at = list.children()[item].ctx().bounding_box().center();
    press(h, at, PointerButton::Primary, 1);
}

fn scrolled(h: &mut Harness, at: Point, dy: f64) {
    h.scroll((at.x, at.y), 1.0, (0.0, dy));
    h.settle();
    h.deliver();
    std::thread::sleep(crate::ui::keymap::gesture::MENU_SETTLE * 2);
    crate::ui::timer::fire(&mut h.root, std::time::Instant::now());
    h.anim(std::time::Duration::from_millis(16));
    h.deliver();
}

fn opened(text: &str) -> (Harness, WidgetId) {
    let mut h = Harness::new();
    h.open(h.write_note("Меню.md", text));
    let editor = h.focus_editor();
    (h, editor)
}

#[test]
fn a_touch_selection_brings_up_the_edit_menu_under_it_and_its_items_keep_the_editor_focused() {
    let (mut h, editor) = opened(NOTE);
    let (start, word) = (caret_at(&mut h, 0), caret_at(&mut h, 3));
    h.press(PointerState { count: 2, ..pointer_at(word.center().x, word.center().y, 1.0) }, PointerButton::Primary, MOUSE);
    assert!(menu(&h).is_none(), "на ПК выделение мышью не открывает меню, только правый щелчок");
    press(&mut h, word.center(), PointerButton::Primary, 2);
    let (frame, commands) = menu(&h).expect("двойной тап по слову не открыл меню");
    assert_eq!(commands, [Cut, Copy, Paste, SelectAll]);
    assert!(frame.y0 >= word.y1 && frame.y0 <= word.y1 + size::SPACE_8, "меню {frame:?} не под строкой {word:?}");
    assert!((frame.x0 - start.x0).abs() <= size::CARET_WIDTH, "меню {frame:?} не от начала выделения {start:?}");
    choose(&mut h, 1);
    assert_eq!(h.copied.as_deref(), Some("первое"));
    assert!(menu(&h).is_none(), "меню осталось после выбора");
    assert_eq!(h.root.focused_widget(), Some(editor), "нажатие на пункт меню увело фокус из редактора");

    let from = NOTE.find("вторая").unwrap();
    let (a, b) = (caret_at(&mut h, from), caret_at(&mut h, from + "вторая".len()));
    h.drag_with((a.x0 + 1.0, a.center().y), (b.x0, b.center().y), TOUCH);
    assert!(menu(&h).is_some(), "протяжка пальцем не открыла меню");
    choose(&mut h, 0);
    assert_eq!(h.copied.as_deref(), Some("вторая"));
    assert_eq!(h.body_text(), "первое слово\n\n строка\n\n");
}

#[test]
fn right_click_and_a_double_tap_on_empty_space_offer_paste_and_inserts() {
    let (mut h, _) = opened(NOTE);
    let caret = [Paste, SelectAll, InsertBook, InsertTable, InsertFrontmatter];
    let (empty, word, second) = (caret_at(&mut h, NOTE.len()), caret_at(&mut h, 3), caret_at(&mut h, NOTE.find("вторая").unwrap() + 4));
    press(&mut h, empty.center(), PointerButton::Secondary, 1);
    let (frame, commands) = menu(&h).expect("правый щелчок не открыл меню");
    assert_eq!(commands, caret);
    assert!(frame.y0 >= empty.center().y && frame.y0 <= empty.center().y + size::SPACE_8, "меню {frame:?} не у точки щелчка {empty:?}");
    choose(&mut h, 1);
    let body = h.app.note_ids.as_ref().unwrap().body;
    assert_eq!(body.get(&h.root).unwrap().inner().selection(), 0..NOTE.len(), "«Выделить всё» не выделило заметку");

    press(&mut h, second.center(), PointerButton::Primary, 1);
    press(&mut h, second.center(), PointerButton::Primary, 2);
    assert_eq!(menu(&h).map(|(_, commands)| commands.len()), Some(4), "двойной тап по слову не открыл меню выделения");
    press(&mut h, word.center(), PointerButton::Secondary, 1);
    assert_eq!(menu(&h).map(|(_, commands)| commands), Some(caret.to_vec()), "правый щелчок вне выделения не снял его");
    press(&mut h, empty.center(), PointerButton::Primary, 1);
    assert!(menu(&h).is_none(), "тап не закрыл меню");
    press(&mut h, empty.center(), PointerButton::Primary, 2);
    assert_eq!(menu(&h).map(|(_, commands)| commands), Some(caret.to_vec()), "долгое нажатие на пустом месте не предложило вставку");
}

#[test]
fn the_edit_menu_closes_on_tap_escape_back_focus_loss_selection_change_and_typing() {
    let (mut h, editor) = opened(NOTE);
    let (word, elsewhere) = (caret_at(&mut h, 3), caret_at(&mut h, NOTE.find("слово").unwrap() + 4));
    let closers: [(&str, &dyn Fn(&mut Harness)); 6] = [
        ("тап мимо", &|h| press(h, elsewhere.center(), PointerButton::Primary, 1)),
        ("Escape", &|h| _ = h.escape()),
        ("«Назад»", &|h| _ = h.app.command(&mut h.root, Command::Back)),
        ("потеря фокуса окном", &|h| _ = h.root.handle_text_event(TextEvent::WindowFocusChange(false))),
        ("смена выделения", &|h| _ = h.key(Key::Named(NamedKey::ArrowRight), Modifiers::SHIFT)),
        ("набор текста", &|h| h.type_text("Ж")),
    ];
    for (what, close) in closers {
        h.root.handle_text_event(TextEvent::WindowFocusChange(true));
        h.root.focus_on(Some(editor));
        press(&mut h, word.center(), PointerButton::Primary, 2);
        assert!(menu(&h).is_some(), "{what}: меню не открылось");
        let before = h.body_text();
        close(&mut h);
        h.deliver();
        assert!(menu(&h).is_none(), "{what}: меню не закрылось");
        if what != "набор текста" {
            assert_eq!(h.body_text(), before, "{what}: текст изменился");
        }
    }
    assert_eq!(h.body_text(), "Ж слово\n\nвторая строка\n\n");
}

#[test]
fn the_edit_menu_stays_with_the_selection_while_scrolling_and_works_in_the_title() {
    let (mut h, _) = opened(&format!("{NOTE}{}", "абзац\n\n".repeat(60)));
    let word = caret_at(&mut h, 3);
    press(&mut h, word.center(), PointerButton::Primary, 2);
    let (before, offset) = (menu(&h).expect("меню не открылось").0, note_offset(&h));
    let below = Point::new(word.x0, word.y1 + 200.0);
    scrolled(&mut h, below, -60.0);
    let moved = note_offset(&h) - offset;
    let (after, _) = menu(&h).expect("после прокрутки меню не вернулось к выделению");
    assert!(moved > 10.0, "страница не прокрутилась: {moved}");
    assert!((before.y0 - after.y0 - moved).abs() < 1.0, "меню {after:?} не сдвинулось вместе с выделением на {moved} от {before:?}");
    choose(&mut h, 1);
    scrolled(&mut h, below, 60.0);
    assert!(menu(&h).is_none(), "закрытое меню вернулось при прокрутке");

    let title = h.app.note_ids.as_ref().unwrap().title;
    let frame = title.get(&h.root).unwrap().ctx().bounding_box();
    press(&mut h, Point::new(frame.x0 + 12.0, frame.center().y), PointerButton::Primary, 2);
    assert_eq!(menu(&h).map(|(_, commands)| commands), Some(vec![Cut, Copy, Paste, SelectAll]), "двойной тап по заголовку не открыл меню");
    choose(&mut h, 1);
    assert_eq!(h.copied.as_deref(), Some("Меню"));
    assert_eq!(h.root.focused_widget(), Some(title.id()), "меню увело фокус из заголовка");
}

#[test]
fn near_the_bottom_right_the_edit_menu_opens_above_and_stays_on_screen() {
    let mut h = super::super::adaptive::at_width(360.0);
    let bars = masonry::kurbo::Insets::new(0.0, 24.0, 0.0, 48.0);
    h.app.fit_window(&mut h.root, h.app.window, bars);
    h.open(h.write_note("Длинная.md", &"слово ".repeat(3000)));
    let body = h.app.note_ids.as_ref().unwrap().body;
    h.root.focus_on(Some(body.id()));
    let right = body.get(&h.root).unwrap().ctx().bounding_box().x1;
    let y = f64::from(H) - bars.y1 - 30.0;
    press(&mut h, Point::new(right - 30.0, y), PointerButton::Primary, 2);
    let (frame, _) = menu(&h).expect("двойной тап у нижнего края не открыл меню");
    let editor = body.get(&h.root).unwrap();
    let line = editor.ctx().window_transform().transform_rect_bbox(editor.inner().caret_rect());
    assert!(frame.y1 <= line.y0, "меню {frame:?} не поднялось над выделением {line:?} у края");
    assert!(frame.x0 >= size::SPACE_8 && frame.x1 <= 360.0 - size::SPACE_8 + 0.5, "меню {frame:?} вышло за экран");
    assert!(frame.y0 >= bars.y0 + size::SPACE_8, "меню {frame:?} зашло под строку состояния");
}
