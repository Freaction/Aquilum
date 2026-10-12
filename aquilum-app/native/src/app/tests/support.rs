use super::*;
use crate::render::Renderer;
use crate::ui::settings::Binding;
use masonry::app::VisualLayerPlan;
use masonry::core::{PointerScrollEvent, ScrollDelta, WidgetId};

pub(super) fn wait_until(tries: usize, millis: u64, mut done: impl FnMut() -> bool) -> bool {
    for _ in 0..tries {
        if done() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }
    false
}

pub(super) fn bench_scale(default: f64) -> f64 {
    std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

pub(super) fn raster_threads() -> u16 {
    std::thread::available_parallelism().map_or(4, |n| n.get() as u16 - 1)
}

pub(super) fn channel(pixel: u32, shift: u32) -> i32 {
    ((pixel >> shift) & 0xff) as i32
}

pub(super) fn differing_pixels(a: &[u32], b: &[u32]) -> usize {
    a.iter().zip(b).filter(|&(&x, &y)| [0, 8, 16].iter().any(|&s| (channel(x, s) - channel(y, s)).abs() > 2)).count()
}

pub(super) fn render_full(layers: &VisualLayerPlan, width: u16, height: u16, scale: f64) -> Renderer {
    let mut full = Renderer::new(2);
    full.render(layers, width, height, scale, theme::background()).unwrap();
    full
}

pub(super) fn dump(name: &str, renderer: &Renderer) {
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join(name), renderer.png()).unwrap();
    }
}

impl Harness {
    pub(super) fn write_note(&self, name: &str, text: &str) -> std::path::PathBuf {
        let path = self.vault.path().join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    pub(super) fn frame(&mut self, width: u16, height: u16, scale: f64) -> Renderer {
        let (layers, _) = self.root.redraw();
        render_full(&layers, width, height, scale)
    }

    pub(super) fn snapshot(&mut self, name: &str) {
        if std::env::var_os("AQ_OUT").is_some() {
            let shot = self.frame(W as u16, H as u16, 1.0);
            dump(name, &shot);
        }
    }

    pub(super) fn step(&mut self) {
        self.app.on_core_events(&mut self.root);
        let _ = self.root.redraw();
    }

    pub(super) fn scroll(&mut self, at: (f64, f64), scale: f64, delta: (f64, f64)) {
        let state = PointerState { position: PhysicalPosition::new(at.0, at.1), count: 1, scale_factor: scale, ..PointerState::default() };
        self.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(delta.0, delta.1)), state }));
    }

    pub(super) fn pointer_move(&mut self, x: f64, y: f64) {
        let state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
        self.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state, coalesced: vec![], predicted: vec![] }));
    }

    pub(super) fn drag(&mut self, from: (f64, f64), to: (f64, f64)) -> Outcome {
        let state = |x: f64, y: f64, pressed: bool| {
            let mut state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
            if pressed {
                state.buttons.insert(PointerButton::Primary);
            }
            state
        };
        let moved = |s: PointerState| PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: s, coalesced: vec![], predicted: vec![] });
        self.root.handle_pointer_event(moved(state(from.0, from.1, false)));
        self.root.handle_pointer_event(PointerEvent::Down(PointerButtonEvent {
            pointer: MOUSE,
            button: Some(PointerButton::Primary),
            state: state(from.0, from.1, true),
        }));
        for step in 1..=10 {
            let k = f64::from(step) / 10.0;
            self.root.handle_pointer_event(moved(state(from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k, true)));
        }
        self.root.handle_pointer_event(PointerEvent::Up(PointerButtonEvent {
            pointer: MOUSE,
            button: Some(PointerButton::Primary),
            state: state(to.0, to.1, false),
        }));
        self.deliver()
    }

    pub(super) fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.key(Key::Character(ch.to_string().into()), Modifiers::empty());
        }
    }

    pub(super) fn focus_editor(&mut self) -> WidgetId {
        let editor = self.app.editor_id().unwrap();
        self.root.handle_text_event(TextEvent::WindowFocusChange(true));
        self.root.focus_on(Some(editor));
        editor
    }

    pub(super) fn body_text(&self) -> String {
        let body = self.app.note_ids.as_ref().unwrap().body;
        body.get(&self.root).unwrap().inner().text().to_string()
    }

    pub(super) fn tab_paths(&self) -> Vec<Option<String>> {
        self.app
            .tabs
            .tabs()
            .iter()
            .map(|t| t.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()))
            .collect()
    }

    pub(super) fn binding(&self, what: &str, pick: impl Fn(&Binding) -> bool) -> WidgetId {
        match &self.app.popup {
            Some(Popup::Settings { dialog, .. }) => dialog.bindings.iter().find_map(|(id, b)| pick(b).then_some(*id)).unwrap_or_else(|| panic!("{what}")),
            _ => panic!("настройки не открыты"),
        }
    }

    pub(super) fn nav(&self, section: Section) -> WidgetId {
        match &self.app.popup {
            Some(Popup::Settings { dialog, .. }) => dialog.nav.iter().find(|(_, s)| *s == section).map(|(id, _)| *id).unwrap(),
            _ => panic!("настройки не открыты"),
        }
    }

    pub(super) fn wait_cover(&mut self, wait: bool) {
        for _ in 0..500 {
            if crate::cover::take_finished() {
                if let Some(cover) = self.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
                    cover.edit(&mut self.root, |mut c| crate::ui::book::PageCover::refresh(&mut c));
                }
                return;
            }
            if !wait {
                return;
            }
            let _ = self.root.redraw();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
