mod dialogs;
mod panels;
mod notes;
mod perf;
mod snapshots;
mod navigation;
mod reader;
mod graph;
mod misc;
mod damage;
#[cfg(feature = "memory-probe")]
mod memory;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use aquilum_core::settings::models::AppConfig;
use masonry::app::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
use masonry::core::keyboard::{Code, Key, KeyState, Location, Modifiers, NamedKey};
use masonry::core::{
    KeyboardEvent, PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerInfo, PointerState,
    PointerType, PointerUpdate, TextEvent,
};
use masonry::dpi::{PhysicalPosition, PhysicalSize};
use masonry::kurbo::Size;

use super::{App, Outcome, Popup};
use crate::ui::settings::schema::Section;
use crate::ui::theme;
use crate::workspace::Workspace;

const W: u32 = 1000;
const H: u32 = 600;
const MOUSE: PointerInfo = PointerInfo {
    pointer_id: Some(PointerId::PRIMARY),
    persistent_device_id: None,
    pointer_type: PointerType::Mouse,
};

fn bench_size() -> Option<(u32, u32)> {
    let size = std::env::var("AQUILUM_BENCH_SIZE").ok()?;
    let (w, h) = size.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

struct Harness {
    app: App,
    root: RenderRoot,
    signals: Rc<RefCell<Vec<RenderRootSignal>>>,
    vault: tempfile::TempDir,
    _data: tempfile::TempDir,
    _theme: std::sync::MutexGuard<'static, ()>,
}

impl Harness {
    pub(super) fn new() -> Self {
        Self::with(None, 1.0)
    }

    pub(super) fn with(real_vault: Option<&std::path::Path>, scale: f64) -> Self {
        let theme = theme::test_lock();
        let vault = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::write(vault.path().join("a.md"), "текст").unwrap();
        let mut config = AppConfig::default();
        config.ui.theme = if std::env::var_os("AQUILUM_BENCH_LIGHT").is_some() { "light".into() } else { "dark".into() };
        config.ui.language = "ru".into();
        std::fs::write(data.path().join("settings.json"), serde_json::to_string(&config).unwrap()).unwrap();
        let workspace = Workspace::open(Some(data.path()), Some(real_vault.unwrap_or(vault.path())), || {}).unwrap();
        let mut app = App::new(workspace, String::new());
        let signals: Rc<RefCell<Vec<RenderRootSignal>>> = Rc::default();
        let sink = signals.clone();
        let widget = app.root_widget(false);
        let options = RenderRootOptions {
            default_properties: Arc::new(theme::default_properties()),
            use_system_fonts: true,
            size_policy: WindowSizePolicy::User,
            size: bench_size().map_or(PhysicalSize::new((f64::from(W) * scale) as u32, (f64::from(H) * scale) as u32), |(w, h)| PhysicalSize::new(w, h)),
            scale_factor: scale,
            test_font: None,
        };
        let mut root = RenderRoot::new(widget, move |s| sink.borrow_mut().push(s), options);
        theme::register_fonts(&mut root);
        app.attach_window_controls(&mut root, Size::new(f64::from(W), f64::from(H)));
        let _ = root.redraw();
        let mut harness = Harness { app, root, signals, vault, _data: data, _theme: theme };
        harness.settle();
        harness
    }

    pub(super) fn settle(&mut self) {
        self.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_secs(1)));
        let _ = self.root.redraw();
    }

    pub(super) fn deliver(&mut self) -> Outcome {
        let _ = self.root.redraw();
        let mut outcome = Outcome::default();
        loop {
            let actions: Vec<_> = std::mem::take(&mut *self.signals.borrow_mut())
                .into_iter()
                .filter_map(|s| match s {
                    RenderRootSignal::Action(action, id) => Some((action, id)),
                    _ => None,
                })
                .collect();
            if actions.is_empty() {
                return outcome;
            }
            for (action, id) in actions {
                let next = self.app.on_action(&mut self.root, id, action, Size::new(f64::from(W), f64::from(H)));
                outcome.title |= next.title;
                outcome.rescale |= next.rescale;
            }
            let _ = self.root.redraw();
        }
    }

    pub(super) fn click(&mut self, x: f64, y: f64) -> Outcome {
        self.click_with(x, y, Modifiers::empty())
    }

    pub(super) fn click_with(&mut self, x: f64, y: f64, modifiers: Modifiers) -> Outcome {
        self.settle();
        let mut state =
            PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, modifiers, ..PointerState::default() };
        self.root.handle_pointer_event(PointerEvent::Move(PointerUpdate {
            pointer: MOUSE,
            current: state.clone(),
            coalesced: vec![],
            predicted: vec![],
        }));
        state.buttons.insert(PointerButton::Primary);
        let down = PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state: state.clone() };
        self.root.handle_pointer_event(PointerEvent::Down(down));
        state.buttons.remove(PointerButton::Primary);
        let up = PointerButtonEvent { pointer: MOUSE, button: Some(PointerButton::Primary), state };
        self.root.handle_pointer_event(PointerEvent::Up(up));
        self.deliver()
    }

    pub(super) fn key(&mut self, key: Key, modifiers: Modifiers) -> Outcome {
        for state in [KeyState::Down, KeyState::Up] {
            let event = KeyboardEvent {
                state,
                key: key.clone(),
                code: Code::Unidentified,
                location: Location::Standard,
                modifiers,
                repeat: false,
                is_composing: false,
            };
            self.root.handle_text_event(TextEvent::Keyboard(event));
        }
        self.deliver()
    }

    pub(super) fn escape(&mut self) -> Outcome {
        for state in [KeyState::Down, KeyState::Up] {
            let event = KeyboardEvent {
                state,
                key: Key::Named(NamedKey::Escape),
                code: Code::Escape,
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                repeat: false,
                is_composing: false,
            };
            self.root.handle_text_event(TextEvent::Keyboard(event));
        }
        self.deliver()
    }

    pub(super) fn config(&self) -> AppConfig {
        self.app.workspace.core.settings.get_config()
    }

    pub(super) fn section(&self) -> Option<Section> {
        match &self.app.popup {
            Some(Popup::Settings { dialog, .. }) => Some(dialog.section),
            _ => None,
        }
    }

    pub(super) fn menu_open(&self) -> bool {
        matches!(&self.app.popup, Some(Popup::Settings { menu: Some(_), .. }))
    }
}
