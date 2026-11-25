use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use masonry::app::{RenderRoot, RenderRootSignal};
use masonry::core::{LayerType, NewWidget, PointerEvent, WidgetId, WidgetTag, WindowEvent};
use masonry::kurbo::Rect;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::widgets::Flex;

use super::TipLayer;
use crate::test_dir::{MOUSE, TOUCH, moved, pointer_at, primary};
use crate::ui::keymap::Command;
use crate::ui::keymap::gesture::TIP_DELAY;
use crate::ui::widgets::IconButton;

const BUTTON: WidgetTag<IconButton> = WidgetTag::named("кнопка");
type Signals = Rc<RefCell<Vec<RenderRootSignal>>>;

fn corner() -> (RenderRoot, Signals, Rect) {
    let button = NewWidget::new(IconButton::new(crate::ui::icons::SEARCH, "Поиск").keys(Command::QuickOpen)).with_tag(BUTTON);
    let column = Flex::column().main_axis_alignment(MainAxisAlignment::End).cross_axis_alignment(CrossAxisAlignment::End).with_fixed(button);
    let signals: Signals = Rc::default();
    let sink = Rc::clone(&signals);
    let mut root = RenderRoot::new(NewWidget::new(column).erased(), move |signal| sink.borrow_mut().push(signal), crate::test_dir::options(400, 300, 1.0));
    crate::ui::theme::register_fonts(&mut root);
    let _ = root.redraw();
    let rect = root.get_widget_with_tag(BUTTON).unwrap().ctx().bounding_box();
    (root, signals, rect)
}

fn frame(root: &mut RenderRoot, signals: &Signals) -> (Option<(String, WidgetId)>, bool) {
    crate::ui::timer::fire(root, Instant::now());
    root.handle_window_event(WindowEvent::AnimFrame(Duration::from_millis(16)));
    let (mut shown, mut removed) = (None, false);
    let taken = std::mem::take(&mut *signals.borrow_mut());
    for signal in taken {
        match signal {
            RenderRootSignal::NewLayer(LayerType::Tooltip(text), widget, at) => {
                shown = Some((text, widget.id()));
                root.add_layer(widget, at);
            }
            RenderRootSignal::RemoveLayer(id) => {
                root.remove_layer(id);
                removed = true;
            }
            _ => {}
        }
    }
    let _ = root.redraw();
    (shown, removed)
}

#[test]
fn a_mouse_hover_shows_the_label_with_keys_inside_the_window_and_a_press_hides_it() {
    let (mut root, signals, button) = corner();
    let at = pointer_at(button.center().x, button.center().y, 1.0);
    root.handle_pointer_event(moved(MOUSE, at.clone()));
    assert!(frame(&mut root, &signals).0.is_none(), "подсказка без задержки");
    std::thread::sleep(TIP_DELAY);
    let (shown, _) = frame(&mut root, &signals);
    let (text, layer) = shown.expect("подсказка не появилась");
    assert_eq!(text, "Поиск");
    let tip = root.get_widget(layer).unwrap();
    assert_eq!(tip.children()[0].children().len(), 2, "в подсказке нет сочетания клавиш");
    let bubble = tip.downcast::<TipLayer>().unwrap().inner().bubble;
    assert!(Rect::new(0.0, 0.0, 400.0, 300.0).contains(bubble.origin()) && bubble.x1 <= 400.0 && bubble.y1 <= 300.0, "подсказка вышла за окно: {bubble:?}");
    assert!(bubble.y1 <= button.y0, "у нижнего края подсказка должна встать над кнопкой: {bubble:?} {button:?}");

    root.handle_pointer_event(PointerEvent::Down(primary(at)));
    assert!(frame(&mut root, &signals).1, "нажатие не убрало подсказку");
}

#[test]
fn a_finger_never_starts_a_tooltip() {
    let (mut root, _, button) = corner();
    root.handle_pointer_event(moved(TOUCH, pointer_at(button.center().x, button.center().y, 1.0)));
    assert!(crate::ui::timer::next().is_none());
}

