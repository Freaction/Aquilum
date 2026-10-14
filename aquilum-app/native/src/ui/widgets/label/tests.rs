use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use masonry::app::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
use masonry::core::keyboard::{Code, Key, KeyState, KeyboardEvent, Location};
use masonry::core::{NewWidget, PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerInfo, PointerState, PointerType, PointerUpdate, TextEvent};
use masonry::dpi::{PhysicalPosition, PhysicalSize};
use masonry::theme::default_property_set;

use super::Label;
use crate::test_dir::command;

const TEXT: &str = "Привет мир";
const WIDTH: u32 = 400;
const HEIGHT: u32 = 60;

type Signals = Rc<RefCell<Vec<RenderRootSignal>>>;

fn mount(label: Label) -> (RenderRoot, Signals) {
    let signals = Signals::default();
    let sink = signals.clone();
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: true,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(WIDTH, HEIGHT),
        scale_factor: 1.0,
        test_font: None,
    };
    let mut root = RenderRoot::new(NewWidget::new(label).erased(), move |signal| sink.borrow_mut().push(signal), options);
    let _ = root.redraw();
    (root, signals)
}

fn label() -> Label {
    Label::new(TEXT, |t| t.text_primary)
}

fn gesture(root: &mut RenderRoot, from: (f64, f64), to: (f64, f64), count: u8) {
    let mouse = PointerInfo { pointer_id: Some(PointerId::PRIMARY), persistent_device_id: None, pointer_type: PointerType::Mouse };
    let mut state = PointerState { position: PhysicalPosition::new(from.0, from.1), count, scale_factor: 1.0, ..PointerState::default() };
    state.buttons.insert(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: mouse, button: Some(PointerButton::Primary), state: state.clone() }));
    state.position = PhysicalPosition::new(to.0, to.1);
    root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: mouse, current: state.clone(), coalesced: vec![], predicted: vec![] }));
    state.buttons.remove(PointerButton::Primary);
    root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: mouse, button: Some(PointerButton::Primary), state }));
    let _ = root.redraw();
}

fn command_key(root: &mut RenderRoot, letter: &str) {
    let event = KeyboardEvent {
        state: KeyState::Down,
        key: Key::Character(letter.into()),
        code: Code::Unidentified,
        location: Location::Standard,
        modifiers: command(),
        repeat: false,
        is_composing: false,
    };
    root.handle_text_event(TextEvent::Keyboard(event));
    let _ = root.redraw();
}

fn copied(signals: &Signals) -> Vec<String> {
    signals
        .borrow()
        .iter()
        .filter_map(|signal| match signal {
            RenderRootSignal::ClipboardStore(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn pixels(root: &mut RenderRoot) -> Vec<u32> {
    let (layers, _) = root.redraw();
    let mut renderer = crate::render::Renderer::new(1);
    renderer.render(&layers, WIDTH as u16, HEIGHT as u16, 1.0, crate::ui::theme::background()).unwrap();
    renderer.frame_pixels().to_vec()
}

#[test]
fn dragging_over_a_selectable_label_and_copying_puts_its_text_on_the_clipboard() {
    let (mut root, signals) = mount(label().selectable());
    gesture(&mut root, (1.0, 8.0), (WIDTH as f64 - 2.0, 8.0), 1);
    command_key(&mut root, "c");
    assert_eq!(copied(&signals), [TEXT]);
}

#[test]
fn a_double_click_selects_one_word_and_select_all_selects_everything() {
    let (mut root, signals) = mount(label().selectable());
    gesture(&mut root, (6.0, 8.0), (6.0, 8.0), 2);
    command_key(&mut root, "c");
    assert_eq!(copied(&signals), ["Привет"]);
    command_key(&mut root, "a");
    command_key(&mut root, "c");
    assert_eq!(copied(&signals).last().map(String::as_str), Some(TEXT));
}

#[test]
fn a_plain_label_neither_selects_nor_copies() {
    let (mut root, signals) = mount(label());
    gesture(&mut root, (1.0, 8.0), (WIDTH as f64 - 2.0, 8.0), 1);
    command_key(&mut root, "c");
    assert!(copied(&signals).is_empty());
}

#[test]
fn the_selection_is_painted_and_goes_away_when_focus_is_lost() {
    let _theme = crate::ui::theme::test_lock();
    let (mut root, _) = mount(label().selectable());
    let before = pixels(&mut root);
    gesture(&mut root, (1.0, 8.0), (WIDTH as f64 - 2.0, 8.0), 1);
    let selected = pixels(&mut root);
    let changed = before.iter().zip(&selected).filter(|(a, b)| a != b).count();
    assert!(changed > 200, "выделение не нарисовано: изменилось {changed} точек");
    root.focus_on(None);
    let after = pixels(&mut root);
    assert_eq!(after, before, "после потери фокуса выделение осталось");
}
