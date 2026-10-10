use crate::files::document::write_file_atomic_impl;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{LogicalSize, WebviewWindow, Window};

const MIN_WIDTH: f64 = 320.0;
const MIN_HEIGHT: f64 = 240.0;
const MAX_WIDTH: f64 = 7_680.0;
const MAX_HEIGHT: f64 = 4_320.0;

#[derive(Clone, Copy, Deserialize, Serialize)]
struct WindowState {
    width: f64,
    height: f64,
    maximized: bool,
}

impl WindowState {
    fn valid(self) -> bool {
        self.width.is_finite()
            && self.height.is_finite()
            && (MIN_WIDTH..=MAX_WIDTH).contains(&self.width)
            && (MIN_HEIGHT..=MAX_HEIGHT).contains(&self.height)
    }
}

pub struct WindowStateManager {
    path: PathBuf,
    state: Mutex<Option<WindowState>>,
}

impl WindowStateManager {
    pub fn new(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join("window-state.json");
        let state = fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<WindowState>(&json).ok())
            .filter(|state| state.valid());
        Self {
            path,
            state: Mutex::new(state),
        }
    }

    pub fn restore(&self, window: &WebviewWindow) {
        let Some(state) = *self.state.lock().unwrap() else {
            return;
        };
        let _ = window.set_size(LogicalSize::new(state.width, state.height));
        if state.maximized {
            let _ = window.maximize();
        }
    }

    pub fn initialize(&self, window: &WebviewWindow) {
        if self.state.lock().unwrap().is_some() {
            return;
        }
        let size = window.inner_size().ok().map(|size| {
            size.to_logical::<f64>(window.scale_factor().unwrap_or(1.0))
        });
        let Some(size) = size else {
            return;
        };
        let state = WindowState {
            width: size.width,
            height: size.height,
            maximized: false,
        };
        if state.valid() {
            *self.state.lock().unwrap() = Some(state);
        }
    }

    pub fn observe(&self, window: &Window) {
        let maximized = window.is_maximized().unwrap_or(false);
        let size = window.inner_size().ok().map(|size| size.to_logical::<f64>(window.scale_factor().unwrap_or(1.0)));
        let mut state = self.state.lock().unwrap();
        let next = match (*state, maximized, size) {
            (Some(previous), true, _) => WindowState { maximized: true, ..previous },
            (_, false, Some(size)) => WindowState {
                width: size.width,
                height: size.height,
                maximized: false,
            },
            (None, true, _) | (_, false, None) => return,
        };
        if !next.valid() {
            return;
        }
        *state = Some(next);
    }

    fn persist(&self) {
        let state = *self.state.lock().unwrap();
        let Some(state) = state else {
            return;
        };
        self.write(state);
    }

    pub fn capture_and_persist(&self, window: &Window) {
        self.observe(window);
        self.persist();
    }

    fn write(&self, state: WindowState) {
        let Ok(json) = serde_json::to_string(&state) else {
            return;
        };
        if let Err(error) = write_file_atomic_impl(&self.path, &json, None) {
            eprintln!("[aquilum:window] размер окна не сохранён: {error}");
        }
    }
}
