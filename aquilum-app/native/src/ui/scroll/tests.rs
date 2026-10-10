use std::sync::Arc;
use std::time::Duration;

use masonry::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
use masonry::core::keyboard::Modifiers;
use masonry::core::{NewWidget, PointerEvent, PointerId, PointerInfo, PointerScrollEvent, PointerState, PointerType, ScrollDelta, WidgetTag, WindowEvent};
use masonry::dpi::{PhysicalPosition, PhysicalSize};
use masonry::layout::Length;
use masonry::theme::default_property_set;
use masonry::widgets::SizedBox;

use super::ScrollArea;

const TAG: WidgetTag<ScrollArea<SizedBox>> = WidgetTag::named("прокрутка");
const MOUSE: PointerInfo = PointerInfo { pointer_id: Some(PointerId::PRIMARY), persistent_device_id: None, pointer_type: PointerType::Mouse };

fn root() -> RenderRoot {
    let content = NewWidget::new(SizedBox::empty().height(Length::px(5000.0)));
    let options = RenderRootOptions {
        default_properties: Arc::new(default_property_set()),
        use_system_fonts: false,
        size_policy: WindowSizePolicy::User,
        size: PhysicalSize::new(400, 300),
        scale_factor: 1.0,
        test_font: None,
    };
    let mut root = RenderRoot::new(NewWidget::new(ScrollArea::new(content)).with_tag(TAG).erased(), |_| {}, options);
    let _ = root.redraw();
    root
}

fn offset(root: &RenderRoot) -> f64 {
    root.get_widget_with_tag(TAG).expect("прокрутка").inner().offset
}

fn scroll(root: &mut RenderRoot, delta: ScrollDelta) {
    let state = PointerState { position: PhysicalPosition::new(200.0, 150.0), modifiers: Modifiers::empty(), count: 1, scale_factor: 1.0, ..PointerState::default() };
    root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta, state }));
}

fn frames(root: &mut RenderRoot, n: usize, trace: &mut Vec<f64>) {
    for _ in 0..n {
        root.handle_window_event(WindowEvent::AnimFrame(Duration::from_millis(8)));
        trace.push(offset(root));
    }
}

#[test]
fn touchpad_at_top_never_pushes_page_down() {
    let mut root = root();
    let mut trace = Vec::new();
    for y in [0.4, 0.7, 1.0, 0.2, 2.0, 0.05] {
        scroll(&mut root, ScrollDelta::LineDelta(0.0, y));
        trace.push(offset(&root));
        frames(&mut root, 3, &mut trace);
    }
    assert!(trace.iter().all(|&o| o == 0.0), "смещение у верхнего края: {trace:?}");
}

#[test]
fn reversing_direction_does_not_overshoot() {
    let mut root = root();
    let mut trace = Vec::new();
    for _ in 0..5 {
        scroll(&mut root, ScrollDelta::LineDelta(0.0, -1.0));
        frames(&mut root, 2, &mut trace);
    }
    let deepest = offset(&root);
    for y in [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0] {
        scroll(&mut root, ScrollDelta::LineDelta(0.0, y));
        frames(&mut root, 2, &mut trace);
    }
    frames(&mut root, 40, &mut trace);
    assert_eq!(offset(&root), 0.0);
    let after: Vec<f64> = trace.iter().copied().skip(10).collect();
    assert!(after.iter().all(|&o| o <= deepest), "после разворота страницу унесло ниже: {after:?}");
    assert!(after.windows(2).all(|w| w[1] <= w[0]), "движение вверх с рывками назад: {after:?}");
}

#[test]
fn wheel_glide_lands_exactly_and_stays_in_bounds() {
    let mut root = root();
    let mut trace = Vec::new();
    scroll(&mut root, ScrollDelta::LineDelta(0.0, -3.0));
    frames(&mut root, 40, &mut trace);
    assert_eq!(offset(&root), 3.0 * super::LINE);
    assert!(trace.windows(2).all(|w| w[1] >= w[0]), "доведение колесом пошло назад: {trace:?}");
    scroll(&mut root, ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 100_000.0)));
    assert_eq!(offset(&root), 0.0);
    scroll(&mut root, ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -100_000.0)));
    assert_eq!(offset(&root), 5000.0 - 300.0);
}
