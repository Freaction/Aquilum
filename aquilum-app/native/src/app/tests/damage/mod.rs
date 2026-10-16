use super::*;
use crate::render::Renderer;
use masonry::core::WidgetId;
use masonry::kurbo::Rect;

mod checks;
mod frames;
mod reports;
mod sash;
mod tree;

use frames::{Frames, Report, summary};

#[derive(Clone, Copy)]
struct Spec {
    scale: f64,
    width: u32,
    height: u32,
}

const FIXED: Spec = Spec { scale: 1.75, width: 1750, height: 1051 };

fn size() -> Spec {
    let (width, height) = bench_size().unwrap_or((2880, 1716));
    Spec { scale: bench_scale(1.75), width, height }
}

fn setup(spec: Spec, body: &str) -> Harness {
    let mut h = Harness::with(None, spec.scale);
    h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(spec.width, spec.height)));
    h.app.window_resized(&mut h.root, Size::new(f64::from(spec.width) / spec.scale, f64::from(spec.height) / spec.scale));
    let note = h.write_note("Заметка.md", body);
    h.app.open(&mut h.root, note);
    for _ in 0..3 {
        h.settle();
    }
    h
}

fn rect_of(h: &Harness, id: WidgetId, scale: f64) -> Rect {
    h.root.get_widget(id).unwrap().ctx().bounding_box().scale_from_origin(scale)
}

fn pointer(h: &mut Harness, x: f64, y: f64, scale: f64, pressed: bool) -> PointerState {
    let mut state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: scale, ..PointerState::default() };
    if pressed {
        state.buttons.insert(PointerButton::Primary);
    }
    h.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state.clone(), coalesced: vec![], predicted: vec![] }));
    state
}

fn press(h: &mut Harness, x: f64, y: f64, scale: f64) {
    let state = pointer(h, x, y, scale, true);
    h.root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state }));
}

fn release(h: &mut Harness, x: f64, y: f64, scale: f64) {
    let state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: scale, ..PointerState::default() };
    h.root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state }));
}

fn click_at(h: &mut Harness, (x, y): (f64, f64), scale: f64) {
    press(h, x, y, scale);
    release(h, x, y, scale);
    h.deliver();
}

fn plain_body(paragraphs: usize) -> String {
    (0..paragraphs).map(|i| format!("Абзац {i}: обычный текст заметки, который переносится на несколько строк в колонке редактора и не содержит разметки.\n\n")).collect()
}

fn table_body(above: usize) -> String {
    let before: String = (0..above).map(|i| format!("Абзац {i} над таблицей с обычным текстом.\n\n")).collect();
    let after: String = (0..30).map(|i| format!("Абзац {i} под таблицей с обычным текстом.\n\n")).collect();
    format!("{before}| Имя | Значение |\n| --- | --- |\n| один | 1 |\n| два | 2 |\n\n{after}")
}

fn scroll_frames(h: &mut Harness, frames: &mut Frames, steps: u32) -> Vec<Report> {
    let (width, height) = (f64::from(frames.width), f64::from(frames.height));
    let scale = frames.scale;
    (0..steps)
        .map(|i| {
            let dy = if i < steps * 2 / 3 { -31.0 } else { 19.0 };
            h.scroll((width * 0.55, height * 0.5), scale, (0.0, dy));
            h.anim(std::time::Duration::from_millis(16));
            frames.draw(h)
        })
        .collect()
}

fn line_point(h: &Harness, paragraph: usize, scale: f64) -> (f64, f64) {
    let ids = h.app.note_ids.as_ref().unwrap();
    let body = rect_of(h, ids.body.id(), 1.0);
    let editor = ids.body.get(&h.root).unwrap();
    let text = editor.inner().text();
    let offset = text.match_indices("\n\n").nth(paragraph.saturating_sub(1)).map_or(0, |(i, _)| if paragraph == 0 { 0 } else { i + 2 });
    let top = editor.inner().top_of(offset).unwrap();
    ((body.x0 + 60.0) * scale, (body.y0 + top + 10.0) * scale)
}

fn table_border(h: &Harness, scale: f64) -> (f64, f64) {
    let ids = h.app.note_ids.as_ref().unwrap();
    let body = rect_of(h, ids.body.id(), 1.0);
    let editor = ids.body.get(&h.root).unwrap();
    let offset = editor.inner().text().find("| Имя").unwrap();
    let top = editor.inner().top_of(offset).unwrap();
    ((body.x0 + 120.0) * scale, (body.y0 + top + 38.0 + 40.0) * scale)
}

fn drag_border(h: &mut Harness, frames: &mut Frames, steps: u32) -> (Vec<Report>, f64) {
    let scale = frames.scale;
    let (x, y) = table_border(h, scale);
    pointer(h, x, y, scale, false);
    let _ = frames.draw(h);
    press(h, x, y, scale);
    h.deliver();
    let _ = frames.draw(h);
    let reports = (1..=steps)
        .map(|i| {
            pointer(h, x, y + f64::from(i) * 3.0 * scale, scale, true);
            h.deliver();
            frames.draw(h)
        })
        .collect();
    release(h, x, y + f64::from(steps) * 3.0 * scale, scale);
    h.deliver();
    (reports, y)
}
