use super::*;
use masonry::core::{PointerScrollEvent, ScrollDelta, WidgetId};
use masonry::kurbo::Rect;

use crate::output::Area;

struct Frames {
    scale: f64,
    width: u16,
    height: u16,
    partial: crate::render::Renderer,
    check: bool,
}

struct Report {
    raster: u64,
    rects: Vec<(u16, u16, u16, u16)>,
    moved: Vec<(u16, u16, u16, u16, i32)>,
    shown: Vec<Area>,
    time: std::time::Duration,
    differ: usize,
}

impl Report {
    fn print(&self, label: &str) {
        let moved: u64 = self.moved.iter().map(|m| u64::from(m.2) * u64::from(m.3)).sum();
        let shown: u64 = self.shown.iter().map(|a| u64::from(a.w) * u64::from(a.h)).sum();
        eprintln!(
            "{label}: растр {} px {:?}; сдвиг {} px {:?}; вывод {} px {:?}; {:.2} мс",
            self.raster,
            self.rects,
            moved,
            self.moved,
            shown,
            self.shown.iter().map(|a| (a.x, a.y, a.w, a.h)).collect::<Vec<_>>(),
            self.time.as_secs_f64() * 1000.0
        );
    }

    fn above(&self, y: f64) -> bool {
        self.rects.iter().any(|r| f64::from(r.1) + 4.0 < y.floor()) || self.moved.iter().any(|m| f64::from(m.1) < y.floor())
    }
}

impl Frames {
    fn new(h: &mut Harness, scale: f64, width: u16, height: u16, check: bool) -> Self {
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get() as u16 - 1);
        let mut frames = Frames { scale, width, height, partial: crate::render::Renderer::new(threads), check };
        let (layers, _) = h.root.redraw();
        frames.partial.draw_frame(&layers, width, height, scale, theme::background(), true).unwrap();
        frames
    }

    fn draw(&mut self, h: &mut Harness) -> Report {
        let (layers, _) = h.root.redraw();
        let t = std::time::Instant::now();
        let mut shown = self.partial.draw_frame(&layers, self.width, self.height, self.scale, theme::background(), false).unwrap();
        crate::output::merge(&mut shown);
        let time = t.elapsed();
        let mut differ = 0;
        if self.check {
            let mut full = crate::render::Renderer::new(2);
            full.render(&layers, self.width, self.height, self.scale, theme::background()).unwrap();
            let channel = |p: u32, shift: u32| ((p >> shift) & 0xff) as i32;
            differ = self.partial.frame_pixels().iter().zip(full.frame_pixels()).filter(|&(&x, &y)| [0, 8, 16].iter().any(|&s| (channel(x, s) - channel(y, s)).abs() > 2)).count();
        }
        let rects = self.partial.rastered.clone();
        let raster = rects.iter().map(|r| u64::from(r.2) * u64::from(r.3)).sum();
        Report { raster, rects, moved: self.partial.moved.clone(), shown, time, differ }
    }
}

fn setup(scale: f64, width: u32, height: u32, body: &str) -> Harness {
    let mut h = Harness::with(None, scale);
    h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(width, height)));
    h.app.window_resized(&mut h.root, Size::new(f64::from(width) / scale, f64::from(height) / scale));
    let note = h.vault.path().join("Заметка.md");
    std::fs::write(&note, body).unwrap();
    h.app.open(&mut h.root, note);
    for _ in 0..3 {
        h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_secs(1)));
        let _ = h.root.redraw();
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

fn size() -> (f64, u32, u32) {
    let scale = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.75);
    let (w, h) = bench_size().unwrap_or((2880, 1716));
    (scale, w, h)
}

fn scroll_frames(h: &mut Harness, frames: &mut Frames, steps: u32) -> Vec<Report> {
    let (width, height) = (f64::from(frames.width), f64::from(frames.height));
    let scale = frames.scale;
    (0..steps)
        .map(|i| {
            let state = PointerState { position: PhysicalPosition::new(width * 0.55, height * 0.5), count: 1, scale_factor: scale, ..PointerState::default() };
            let dy = if i < steps * 2 / 3 { -31.0 } else { 19.0 };
            h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
            h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_millis(16)));
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

fn summary(label: &str, reports: &[Report]) {
    let n = reports.len().max(1) as f64;
    let raster: f64 = reports.iter().map(|r| r.raster as f64).sum::<f64>() / n;
    let moved: f64 = reports.iter().map(|r| r.moved.iter().map(|m| f64::from(m.2) * f64::from(m.3)).sum::<f64>()).sum::<f64>() / n;
    let shown: f64 = reports.iter().flat_map(|r| r.shown.iter()).map(|a| f64::from(a.w) * f64::from(a.h)).sum::<f64>() / n;
    let time: f64 = reports.iter().map(|r| r.time.as_secs_f64() * 1000.0).sum::<f64>() / n;
    eprintln!("{label}: в среднем растр {raster:.0} px, сдвиг {moved:.0} px, вывод {shown:.0} px, {time:.2} мс на кадр");
}

#[test]
#[ignore]
fn scroll_damage_report() {
    let (scale, width, height) = size();
    let (w, hh) = (width as u16, height as u16);
    let mut h = setup(scale, width, height, &plain_body(120));
    let ids = h.app.note_ids.as_ref().unwrap();
    let (viewport, column) = (rect_of(&h, ids.scroll.id(), scale), rect_of(&h, ids.body.id(), scale));
    eprintln!("окно {width}x{height}@{scale}; область прокрутки {viewport:?}; колонка текста x {:.0}..{:.0}", column.x0, column.x1);
    let mut frames = Frames::new(&mut h, scale, w, hh, false);
    let reports = scroll_frames(&mut h, &mut frames, 30);
    for (i, r) in reports.iter().enumerate().take(4) {
        r.print(&format!("прокрутка {i}"));
    }
    summary("прокрутка", &reports);
}

#[test]
#[ignore]
fn caret_damage_report() {
    let (scale, width, height) = size();
    let (w, hh) = (width as u16, height as u16);
    let mut h = setup(scale, width, height, &format!("Строка с **жирным** и `кодом`.\n\n{}", plain_body(40)));
    let mut frames = Frames::new(&mut h, scale, w, hh, false);
    for (label, paragraph) in [("каретка: первый щелчок в обычную строку", 3), ("каретка: другая обычная строка", 5), ("каретка: строка с разметкой", 0), ("каретка: обратно в обычную", 4)] {
        let point = line_point(&h, paragraph, scale);
        click_at(&mut h, point, scale);
        frames.draw(&mut h).print(label);
    }
    for i in 0..2 {
        h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
        frames.draw(&mut h).print(&format!("каретка: стрелка вниз {i}"));
    }
    h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
    frames.draw(&mut h).print("каретка: стрелка вправо");
}

#[test]
#[ignore]
fn table_damage_report() {
    let (scale, width, height) = size();
    let (w, hh) = (width as u16, height as u16);
    let mut h = setup(scale, width, height, &table_body(6));
    let mut frames = Frames::new(&mut h, scale, w, hh, false);
    let (reports, border) = drag_border(&mut h, &mut frames, 8);
    eprintln!("граница строки таблицы на y = {border:.0}");
    for (i, r) in reports.iter().enumerate().take(3) {
        r.print(&format!("таблица {i}"));
    }
    summary("таблица", &reports);
}

#[test]
#[ignore]
fn caret_note_report() {
    let (scale, width, height) = size();
    let (w, hh) = (width as u16, height as u16);
    let source = std::env::var("AQ_NOTE").unwrap();
    let mut h = setup(scale, width, height, &std::fs::read_to_string(source).unwrap());
    let ids = h.app.note_ids.as_ref().unwrap();
    let body = rect_of(&h, ids.body.id(), scale);
    eprintln!("колонка текста {body:?}");
    let mut frames = Frames::new(&mut h, scale, w, hh, true);
    let dx: f64 = std::env::var("AQ_X").ok().and_then(|v| v.parse().ok()).unwrap_or(300.0);
    for step in 0..12 {
        let (x, y) = (body.x0 + dx * scale, body.y0 + (40.0 + 61.0 * f64::from(step)) * scale);
        if y > f64::from(height) - 20.0 {
            break;
        }
        click_at(&mut h, (x, y), scale);
        let report = frames.draw(&mut h);
        report.print(&format!("щелчок {step} в ({x:.0}, {y:.0}), расхождений {}", report.differ));
    }
}

const SCALE: f64 = 1.75;
const WIDTH: u32 = 1750;
const HEIGHT: u32 = 1051;

#[test]
fn editor_scroll_shifts_only_the_text_column() {
    let mut h = setup(SCALE, WIDTH, HEIGHT, &plain_body(80));
    let column = rect_of(&h, h.app.note_ids.as_ref().unwrap().body.id(), SCALE);
    let mut frames = Frames::new(&mut h, SCALE, WIDTH as u16, HEIGHT as u16, true);
    let reports = scroll_frames(&mut h, &mut frames, 9);
    for (i, r) in reports.iter().enumerate() {
        assert_eq!(r.differ, 0, "шаг {i}: прокрутка расходится с полной отрисовкой в {} пикселях", r.differ);
        assert!(r.shown.iter().all(|a| f64::from(a.w) <= column.width() + 56.0), "шаг {i}: в окно выводится шире колонки текста: {:?}", r.shown);
    }
    let moved: Vec<_> = reports.iter().flat_map(|r| r.moved.iter()).collect();
    assert!(moved.len() >= 6, "прокрутка не сдвигала пиксели: {moved:?}");
    for m in moved {
        assert!(f64::from(m.0) >= column.x0 - 16.0 && f64::from(m.0 + m.2) <= column.x1 + 40.0, "сдвиг {m:?} шире колонки текста {column:?}");
    }
}

#[test]
fn table_row_resize_repaints_only_the_table_and_below() {
    let mut h = setup(SCALE, WIDTH, HEIGHT, &table_body(2));
    let ids = h.app.note_ids.as_ref().unwrap();
    let body = rect_of(&h, ids.body.id(), SCALE);
    let offset = ids.body.get(&h.root).unwrap().inner().text().find("| Имя").unwrap();
    let top = body.y0 + ids.body.get(&h.root).unwrap().inner().top_of(offset).unwrap() * SCALE;
    let mut frames = Frames::new(&mut h, SCALE, WIDTH as u16, HEIGHT as u16, true);
    let (reports, _) = drag_border(&mut h, &mut frames, 4);
    assert_ne!(h.body_text(), table_body(2), "строка таблицы не изменила высоту");
    for (i, r) in reports.iter().enumerate() {
        assert_eq!(r.differ, 0, "шаг {i}: изменение высоты строки расходится с полной отрисовкой в {} пикселях", r.differ);
        assert!(r.raster > 0, "шаг {i}: таблица не перерисована");
        assert!(!r.above(top), "шаг {i}: перерисовано то, что выше таблицы (верх таблицы {top}): {:?}", r.rects);
    }
}

#[test]
fn caret_repaints_only_what_the_reveal_changes() {
    let list: String = (1..=12).map(|i| format!("{i}. пункт списка номер {i}, длинная строка текста, которая занимает почти всю ширину колонки\n")).collect();
    let mut h = setup(SCALE, WIDTH, HEIGHT, &format!("Обычная строка текста без разметки, достаточно длинная.\n\n{list}"));
    let column = rect_of(&h, h.app.note_ids.as_ref().unwrap().body.id(), SCALE);
    let mut frames = Frames::new(&mut h, SCALE, WIDTH as u16, HEIGHT as u16, true);
    let point = line_point(&h, 0, SCALE);
    click_at(&mut h, point, SCALE);
    let plain = frames.draw(&mut h);
    assert_eq!(plain.differ, 0, "каретка в обычной строке: расхождение с полной отрисовкой в {} пикселях", plain.differ);
    assert!(plain.raster < 3000, "каретка в обычной строке перерисовала больше самой каретки: {:?}", plain.rects);
    assert_eq!(plain.shown.iter().map(|a| u64::from(a.w) * u64::from(a.h)).sum::<u64>(), plain.raster, "в окно выводится больше перерисованного: {:?}", plain.shown);
    let ids = h.app.note_ids.as_ref().unwrap();
    let editor = ids.body.get(&h.root).unwrap();
    let top = editor.inner().top_of(editor.inner().text().find("3. пункт").unwrap()).unwrap();
    click_at(&mut h, (column.x0 + 2.0 * SCALE, column.y0 + (top + 10.0) * SCALE), SCALE);
    let marker = frames.draw(&mut h);
    assert_eq!(marker.differ, 0, "каретка у номера пункта: расхождение с полной отрисовкой в {} пикселях", marker.differ);
    assert!(marker.raster > 0, "номер пункта не перерисован");
    let widest = marker.rects.iter().map(|r| f64::from(r.2)).fold(0.0, f64::max);
    assert!(widest < column.width() / 2.0, "каретка у номера пункта перерисовала всю строку: {:?}", marker.rects);
}
