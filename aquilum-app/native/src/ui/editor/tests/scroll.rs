use masonry::app::RenderRoot;
use masonry::core::keyboard::NamedKey;

use super::support::{drag, editor_id, named, paste, scrolled, typing, wheel, PAGE, TAG};
use crate::ui::editor::TextEditor;
use crate::ui::scroll::ScrollArea;

#[test]
fn typing_below_the_view_scrolls_after_the_caret() {
    let mut root = scrolled(TextEditor::new("Начало").markdown(), 400, 300);
    root.focus_on(Some(editor_id(&root)));
    named(&mut root, NamedKey::End);
    let caret = |root: &RenderRoot| {
        let w = root.get_widget_with_tag(TAG).unwrap();
        w.ctx().window_transform().transform_rect_bbox(w.inner().caret_rect())
    };
    for line in 0..30 {
        named(&mut root, NamedKey::Enter);
        typing(&mut root, &format!("строка {line} "));
        let at = caret(&root);
        assert!(at.y0 >= 0.0 && at.y1 <= 300.5, "каретка за краем окна после строки {line}: {at:?}");
    }
    typing(&mut root, &"слово ".repeat(80));
    let at = caret(&root);
    assert!(at.y0 >= 0.0 && at.y1 <= 300.5, "каретка за краем окна после переноса строки: {at:?}");
    paste(&mut root, &"\nвставка".repeat(40));
    let at = caret(&root);
    assert!(at.y0 >= 0.0 && at.y1 <= 300.5, "каретка за краем окна после вставки: {at:?}");
}

#[test]
fn scrolling_far_repaints_visible_paragraphs() {
    let body: String = (0..400).map(|i| format!("Абзац {i}\n")).collect();
    let mut root = scrolled(TextEditor::new(&body), 400, 300);
    wheel(&mut root, (200.0, 150.0), (0.0, -5000.0));
    let painted = root.get_widget_with_tag(TAG).expect("редактор").inner().painted;
    assert!(painted.0 <= 5000.0 && painted.1 >= 5300.0, "нарисовано {painted:?}, а видно 5000..5300");
}

#[test]
fn measuring_above_the_view_does_not_move_the_text() {
    let body: String = (0..3000).map(|i| if i % 7 == 0 { format!("Длинный абзац {i}: {}\n", "слово ".repeat(60)) } else { format!("Абзац {i}\n") }).collect();
    let mut root = scrolled(TextEditor::new(&body).markdown(), 600, 600);
    let scroll = root.get_widget_with_tag(PAGE).unwrap().id();
    let set = |root: &mut RenderRoot, offset: f64| {
        root.edit_widget(scroll, |mut w| ScrollArea::set_offset(&mut w.downcast::<ScrollArea<TextEditor>>(), offset));
        for _ in 0..3 {
            let _ = root.redraw();
        }
    };
    let offset = |root: &RenderRoot| root.get_widget_with_tag(PAGE).unwrap().inner().offset();
    let at = |root: &RenderRoot, y: f64| {
        let w = root.get_widget_with_tag(TAG).unwrap();
        w.inner().offset_at(w.ctx().window_transform().inverse(), masonry::kurbo::Point::new(40.0, y))
    };
    set(&mut root, 1.0e9);
    let mut shifted = false;
    for _ in 0..80 {
        let before = at(&root, 10.0);
        let from = offset(&root);
        set(&mut root, from - 100.0);
        shifted |= (offset(&root) - (from - 100.0)).abs() > 1.0;
        assert_eq!(at(&root, 110.0), before, "текст сдвинулся при прокрутке вверх с {from}");
    }
    assert!(shifted, "оценки выше видимой области ни разу не уточнились");
}

#[test]
fn a_long_code_line_scrolls_sideways_instead_of_wrapping() {
    let code = format!("```\n{}\n```\nпосле", "x".repeat(300));
    let mut root = scrolled(TextEditor::new(&code).markdown(), 600, 400);
    let at = |root: &RenderRoot| {
        let w = root.get_widget_with_tag(TAG).unwrap();
        w.inner().offset_at(w.ctx().window_transform().inverse(), masonry::kurbo::Point::new(300.0, 66.0))
    };
    let before = at(&root);
    wheel(&mut root, (300.0, 66.0), (-200.0, 0.0));
    let wheeled = at(&root);
    assert!(wheeled > before + 10, "строка кода не прокрутилась вбок: {before} → {wheeled}");
    let track = {
        let w = root.get_widget_with_tag(TAG).unwrap();
        let bottom = w.inner().top_of(code.find("после").unwrap()).unwrap();
        (w.ctx().window_transform() * masonry::kurbo::Point::new(0.0, bottom - 4.0)).y
    };
    drag(&mut root, (120.0, track), (360.0, track));
    assert!(at(&root) > wheeled + 10, "ползунок полосы прокрутки не тянется: {wheeled} → {}", at(&root));
}

#[test]
fn dragging_a_selection_past_the_edge_scrolls_smoothly() {
    use masonry::core::{PointerButton, PointerEvent, WindowEvent};
    use masonry::dpi::PhysicalPosition;
    use crate::test_dir::{moved, pointer_at, primary};
    let body: String = (0..200).map(|i| format!("Абзац {i}\n")).collect();
    let mut root = scrolled(TextEditor::new(&body), 400, 300);
    let page = root.get_widget_with_tag(PAGE).unwrap().id();
    root.edit_widget(page, |mut w| ScrollArea::set_offset(&mut w.downcast::<ScrollArea<TextEditor>>(), 2000.0));
    let _ = root.redraw();
    let offset = |root: &RenderRoot| root.get_widget_with_tag(PAGE).unwrap().inner().offset();
    let mut state = pointer_at(100.0, 150.0, 1.0);
    root.handle_pointer_event(moved(state.clone()));
    state.buttons.insert(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Down(primary(state.clone())));
    state.position = PhysicalPosition::new(100.0, -40.0);
    root.handle_pointer_event(moved(state));
    let _ = root.redraw();
    let mut last = offset(&root);
    for _ in 0..10 {
        root.handle_window_event(WindowEvent::AnimFrame(std::time::Duration::from_millis(16)));
        let _ = root.redraw();
        let step = last - offset(&root);
        assert!(step > 1.0 && step < 20.0, "шаг прокрутки за кадр {step}");
        last = offset(&root);
    }
    let caret = root.get_widget_with_tag(TAG).unwrap();
    let top = caret.ctx().window_transform().transform_rect_bbox(caret.inner().caret_rect()).y0;
    assert!(top < 0.0 && top > -80.0, "выделение не идёт за указателем: каретка на {top}");
}

#[test]
fn pressing_the_scrollbar_keeps_the_text_focused() {
    let body: String = (0..200).map(|i| format!("Абзац {i}\n")).collect();
    let mut root = scrolled(TextEditor::new(&body), 400, 300);
    let editor = editor_id(&root);
    root.focus_on(Some(editor));
    let _ = root.redraw();
    super::support::click(&mut root, 397.0, 20.0);
    assert_eq!(root.focused_widget(), Some(editor), "полоса прокрутки сняла фокус с текста");
}

#[test]
fn the_keyboard_keeps_a_tapped_table_cell_in_view() {
    let body: String = (0..150).map(|i| format!("Абзац {i}\n")).collect::<String>() + "| имя | срок |\n|---|---|\n| отчёт | май |\n";
    let mut root = scrolled(TextEditor::new(&body).markdown(), 400, 300);
    let page = root.get_widget_with_tag(PAGE).unwrap().id();
    root.edit_widget(page, |mut w| ScrollArea::set_offset(&mut w.downcast::<ScrollArea<TextEditor>>(), 1.0e9));
    let _ = root.redraw();
    let offset = |root: &RenderRoot| root.get_widget_with_tag(PAGE).unwrap().inner().offset();
    let table = {
        let w = root.get_widget_with_tag(TAG).unwrap();
        let top = w.inner().top_of(body.find('|').unwrap()).unwrap();
        w.ctx().window_transform() * masonry::kurbo::Point::new(40.0, top + 45.0)
    };
    super::support::click(&mut root, table.x, table.y);
    let before = offset(&root);
    let editor = editor_id(&root);
    root.edit_widget(editor, |mut w| TextEditor::keep_caret(&mut w.downcast::<TextEditor>()));
    let _ = root.redraw();
    assert!((offset(&root) - before).abs() < 1.0, "окно уехало с ячейки: {before} → {}", offset(&root));
}
