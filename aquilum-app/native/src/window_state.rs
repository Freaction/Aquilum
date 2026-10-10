//! Размер окна между запусками — как `src-tauri/src/window_state.rs`: тот же `window-state.json`
//! в каталоге данных, те же логические пиксели и пределы. Пока окно развёрнуто, запоминается
//! прежний обычный размер.

use std::path::{Path, PathBuf};

use aquilum_core::files::document::write_file_atomic_impl;
use serde::{Deserialize, Serialize};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::window::Window;

const MIN_WIDTH: f64 = 320.0;
const MIN_HEIGHT: f64 = 240.0;
const MAX_WIDTH: f64 = 7_680.0;
const MAX_HEIGHT: f64 = 4_320.0;

/// Размер окна при первом запуске — как `width`/`height` в `tauri.conf.json`.
pub const DEFAULT_SIZE: LogicalSize<f64> = LogicalSize::new(1000.0, 600.0);

#[derive(Clone, Copy, Deserialize, Serialize)]
struct State {
    width: f64,
    height: f64,
    maximized: bool,
    /// Положение окна; используется только с `--keep-position` (перезапуск при правке кода).
    #[serde(default)]
    position: Option<(i32, i32)>,
}

impl State {
    fn valid(self) -> bool {
        self.width.is_finite()
            && self.height.is_finite()
            && (MIN_WIDTH..=MAX_WIDTH).contains(&self.width)
            && (MIN_HEIGHT..=MAX_HEIGHT).contains(&self.height)
    }
}

pub struct WindowState {
    path: PathBuf,
    state: Option<State>,
}

impl WindowState {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("window-state.json");
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<State>(&json).ok())
            .filter(|state| state.valid());
        WindowState { path, state }
    }

    /// Размер, с которым создавать окно.
    pub fn size(&self) -> LogicalSize<f64> {
        self.state.map_or(DEFAULT_SIZE, |s| LogicalSize::new(s.width, s.height))
    }

    pub fn position(&self) -> Option<PhysicalPosition<i32>> {
        self.state.and_then(|s| s.position).map(|(x, y)| PhysicalPosition::new(x, y))
    }

    pub fn maximized(&self) -> bool {
        self.state.is_some_and(|s| s.maximized)
    }

    /// Запоминает текущий размер; вызывается на каждое изменение размера окна.
    pub fn observe(&mut self, window: &dyn Window) {
        let maximized = window.is_maximized();
        let size = window.surface_size().to_logical::<f64>(window.scale_factor());
        let next = match (self.state, maximized) {
            (Some(previous), true) => State { maximized: true, ..previous },
            (_, false) => {
                let position = window.outer_position().ok().map(|p| (p.x, p.y));
                State { width: size.width, height: size.height, maximized: false, position }
            }
            (None, true) => return,
        };
        if next.valid() {
            self.state = Some(next);
        }
    }

    /// Записывает размер на диск; вызывается при закрытии окна.
    pub fn persist(&mut self, window: &dyn Window) {
        self.observe(window);
        let Some(state) = self.state else { return };
        let Ok(json) = serde_json::to_string(&state) else { return };
        if let Err(error) = write_file_atomic_impl(&self.path, &json, None) {
            eprintln!("размер окна не сохранён: {error}");
        }
    }
}
