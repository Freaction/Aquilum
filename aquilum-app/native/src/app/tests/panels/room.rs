use super::*;
use crate::interface::Side;
use crate::ui::widgets::Pressed;
use masonry::core::WidgetId;

fn width_of(h: &Harness, id: WidgetId) -> f64 {
    h.root.get_widget(id).unwrap().ctx().bounding_box().width()
}

fn drag_to(h: &mut Harness, width: f64) {
    h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(width as u32, H)));
    h.app.window_resized(&mut h.root, Size::new(width, f64::from(H)));
    let _ = h.root.redraw();
}

fn shown(h: &Harness) -> (f64, f64, f64) {
    let chrome = h.app.chrome.as_ref().unwrap();
    let editor = h.app.note_ids.as_ref().unwrap().scroll.id();
    (width_of(h, chrome.sidebar.id()), width_of(h, chrome.panel.id()), width_of(h, editor))
}

#[test]
fn a_narrow_window_hides_the_right_panel_first_and_brings_both_back_while_dragging() {
    let mut h = Harness::new();
    h.open(h.write_note("Заметка.md", "текст"));
    h.app.interface.set_width(Side::Sidebar, Some(400.0));
    drag_to(&mut h, 1200.0);
    let (sidebar, panel, _) = shown(&h);
    assert!(sidebar > 0.0 && panel > 0.0, "на широком окне видны обе панели: {sidebar} {panel}");
    drag_to(&mut h, 900.0);
    let (sidebar, panel, editor) = shown(&h);
    assert!(sidebar > 0.0 && panel == 0.0 && editor >= 300.0, "сначала уходит правая: {sidebar} {panel} {editor}");
    drag_to(&mut h, 700.0);
    let (sidebar, panel, editor) = shown(&h);
    assert!(sidebar == 0.0 && panel == 0.0 && editor >= 300.0, "затем левая: {sidebar} {panel} {editor}");
    drag_to(&mut h, 500.0);
    h.anim(std::time::Duration::from_secs(1));
    drag_to(&mut h, 1200.0);
    let (sidebar, panel, _) = shown(&h);
    assert!(sidebar > 0.0 && panel > 0.0, "панели вернулись без кадров анимации: {sidebar} {panel}");
    assert!(h.app.interface.sidebar_open() && h.app.interface.right_open(), "сужение окна не меняет настройки");
}

#[test]
fn the_panel_button_makes_room_for_a_hidden_panel() {
    let mut h = Harness::new();
    drag_to(&mut h, 860.0);
    assert!(!h.app.panel_visible(), "правой панели не хватает места");
    let toggle = h.app.chrome.as_ref().unwrap().panel_toggle;
    h.act(toggle, Pressed);
    h.settle();
    assert!(h.app.panel_visible(), "кнопка показала правую панель");
    assert!(!h.app.interface.sidebar_open(), "левая уступила ей место");
}

#[test]
fn the_note_fills_the_window_at_every_width() {
    let mut h = Harness::new();
    h.open(h.write_note("Заметка.md", "текст"));
    for width in [411.0, 1000.0] {
        drag_to(&mut h, width);
        h.settle();
        let height = h.root.get_widget(h.app.note_ids.as_ref().unwrap().scroll.id()).unwrap().ctx().bounding_box().height();
        assert!(height > f64::from(H) / 2.0, "заметке не досталось высоты при ширине {width}: {height}");
    }
}
