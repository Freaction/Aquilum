use super::*;
use crate::ui::motion;
use crate::ui::tokens::size;
use masonry::core::WidgetId;
use masonry::kurbo::{Point, Rect};

fn rect(h: &Harness, id: WidgetId) -> Rect {
    h.root.get_widget(id).unwrap().ctx().bounding_box()
}

fn wide_window() -> Harness {
    let mut h = Harness::new();
    h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(2400, 800)));
    h.app.window_resized(&mut h.root, Size::new(2400.0, 800.0));
    let mut config = h.config();
    config.editor.max_width_ch = 40;
    h.app.workspace.core.settings.update_config(config).unwrap();
    h.app.rebuild(&mut h.root);
    let note = h.write_note("Заметка.md", "текст");
    h.open(note);
    h.settle();
    h
}

fn column(h: &Harness) -> Rect {
    rect(h, h.app.note_ids.as_ref().unwrap().body.id())
}

fn page(h: &Harness) -> Rect {
    rect(h, h.app.note_ids.as_ref().unwrap().scroll.id())
}

#[test]
fn full_width_keeps_margins_on_tokens_and_goes_back() {
    let mut h = wide_window();
    let narrow = column(&h);
    assert!(narrow.width() < page(&h).width() - 2.0 * size::SPACE_32 - 100.0, "в окне нет места для широкого режима");
    h.app.toggle_wide_mode(&mut h.root);
    h.settle();
    let wide = column(&h);
    assert_eq!(wide.x0 - page(&h).x0, size::SPACE_32, "левое поле не 32 px");
    assert_eq!(page(&h).x1 - wide.x1, size::SPACE_32 + size::SCROLLBAR_WIDTH, "правое поле не 32 px рядом с полосой прокрутки");
    assert!(h.app.interface.wide());
    h.app.toggle_wide_mode(&mut h.root);
    h.settle();
    assert_eq!(column(&h).width(), narrow.width());
    assert!(!h.app.interface.wide());
}

#[test]
fn width_change_is_animated_and_instant_without_animations() {
    let mut h = wide_window();
    let narrow = column(&h).width();
    h.app.toggle_wide_mode(&mut h.root);
    h.anim(std::time::Duration::from_millis(30));
    let _ = h.root.redraw();
    let during = column(&h).width();
    assert!(during > narrow, "ширина не начала расти");
    h.settle();
    let wide = column(&h).width();
    assert!(during < wide, "ширина дошла до конца за один кадр");
    motion::set(false);
    h.app.toggle_wide_mode(&mut h.root);
    h.anim(std::time::Duration::from_millis(1));
    let _ = h.root.redraw();
    motion::set(true);
    assert_eq!(column(&h).width(), narrow, "без анимаций ширина вернулась не сразу");
}

#[test]
fn note_actions_menu_toggles_full_width() {
    let mut h = wide_window();
    h.app.open_note_actions_menu(&mut h.root, Point::new(500.0, 40.0));
    let Some(Popup::NoteActionsMenu { layer }) = &h.app.popup else { panic!("меню действий не открыто") };
    let layer = *layer;
    h.act(layer, crate::ui::menu::MenuChoice::Item(0));
    assert!(h.app.interface.wide(), "первый пункт меню — «Во всю ширину»");
}

#[test]
#[ignore]
fn width_tween_frame_cost() {
    for paragraphs in [20, 2000] {
        let mut h = Harness::new();
        h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(2400, 1200)));
        h.app.window_resized(&mut h.root, Size::new(2400.0, 1200.0));
        let mut config = h.config();
        config.editor.max_width_ch = 65;
        h.app.workspace.core.settings.update_config(config).unwrap();
        h.app.rebuild(&mut h.root);
        let body: String = (0..paragraphs).map(|i| format!("Абзац {i}: обычный текст заметки, который переносится на несколько строк в колонке редактора и не содержит разметки, но достаточно длинный для проверки.\n\n")).collect();
        let note = h.write_note("Большая.md", &body);
        h.open(note);
        h.settle();
        let started = std::time::Instant::now();
        let _ = h.root.redraw();
        let base = started.elapsed();
        h.app.toggle_wide_mode(&mut h.root);
        let mut worst = std::time::Duration::ZERO;
        let mut total = std::time::Duration::ZERO;
        for _ in 0..12 {
            let started = std::time::Instant::now();
            h.anim(std::time::Duration::from_millis(16));
            let _ = h.root.redraw();
            let spent = started.elapsed();
            total += spent;
            worst = worst.max(spent);
        }
        eprintln!("ЗАМЕР абзацев {paragraphs} ({} КБ): кадр без изменений {base:?}, средний кадр перехода {:?}, худший {worst:?}", body.len() / 1024, total / 12);
    }
}
