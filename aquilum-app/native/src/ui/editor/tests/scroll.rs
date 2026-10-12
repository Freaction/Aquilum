use masonry::app::RenderRoot;

use super::support::{scrolled, wheel, PAGE, TAG};
use crate::ui::editor::TextEditor;
use crate::ui::scroll::ScrollArea;

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
