use super::*;
use crate::ui::titlebar::TitlebarAction;
use crate::ui::tokens::size;
use masonry::core::{Widget, WidgetRef};
use masonry::kurbo::Rect;

fn tabs(widget: WidgetRef<'_, dyn Widget>, out: &mut Vec<Rect>) {
    if widget.short_type_name() == "TabButton" {
        out.push(widget.ctx().bounding_box());
    }
    for child in widget.children() {
        tabs(child, out);
    }
}

fn press(h: &mut Harness, x: f64, y: f64) -> (bool, bool) {
    let mut state = pointer_at(x, y, 1.0);
    h.root.handle_pointer_event(moved(MOUSE, state.clone()));
    state.buttons.insert(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state: state.clone() }));
    let drag = h.signals.borrow().iter().any(|s| matches!(s, RenderRootSignal::Action(a, _) if a.downcast_ref::<TitlebarAction>() == Some(&TitlebarAction::Drag)));
    let captured = h.root.pointer_capture_target().is_some();
    state.buttons.remove(PointerButton::Primary);
    h.root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state }));
    h.signals.borrow_mut().clear();
    (drag, captured)
}

#[test]
fn every_free_spot_of_the_header_drags_the_window() {
    let mut h = Harness::new();
    for name in ["б.md", "в.md"] {
        let path = h.vault.path().join(name);
        std::fs::write(&path, "текст").unwrap();
        h.app.open_in(&mut h.root, path, true);
    }
    h.settle();
    h.signals.borrow_mut().clear();
    let mut dead = Vec::new();
    for y in [1.0, size::TABS_HEIGHT / 2.0, size::TABS_HEIGHT - 2.0] {
        for x in (0..W).step_by(2).map(f64::from) {
            let (drag, captured) = press(&mut h, x, y);
            if drag == captured {
                dead.push((x, y, drag));
            }
        }
    }
    assert!(dead.is_empty(), "шапка в этих точках и не тащит окно, и не отдаёт нажатие элементу: {dead:?}");
    let mut boxes = Vec::new();
    tabs(h.root.get_layer_root(0), &mut boxes);
    boxes.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    let gap = (boxes[0].x1 + boxes[1].x0) / 2.0;
    assert_eq!(press(&mut h, gap, size::TABS_HEIGHT / 2.0), (true, false), "между вкладками окно не тащится");
}
