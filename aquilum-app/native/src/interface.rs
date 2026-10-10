//! Состояние интерфейса между запусками — то, что фронтенд держит в localStorage:
//! масштаб (`aquilum-scale-factor`) и открытость левой панели (`aquilum_sidebar_left_open`).
//! Файл `interface.json` в каталоге данных.

use std::path::{Path, PathBuf};

use aquilum_core::files::document::write_file_atomic_impl;
use serde::{Deserialize, Serialize};

/// Пределы и шаг масштаба — `MIN_SCALE`, `MAX_SCALE`, `SCALE_STEP` из `modules/scaling`.
pub const MIN_SCALE: f64 = 0.7;
pub const MAX_SCALE: f64 = 1.5;
pub const SCALE_STEP: f64 = 0.05;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct State {
    scale: f64,
    sidebar_open: bool,
    right_open: bool,
    panel_mode: Option<String>,
    analysis_method: Option<String>,
    /// Раздел настроек, на котором закрыли окно (`aquilum_settings_section`).
    settings_section: Option<String>,
    metadata_expanded: Vec<String>,
    focus_mode: bool,
    graph: GraphPrefs,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GraphPrefs {
    pub node_size: f64,
    pub spread: f64,
    pub highlight_depth: usize,
    pub labels: bool,
    pub heatmap_axis: u8,
}

impl Default for GraphPrefs {
    fn default() -> Self {
        GraphPrefs { node_size: 1.0, spread: 1.0, highlight_depth: 1, labels: true, heatmap_axis: 0 }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            scale: 1.0,
            sidebar_open: true,
            right_open: true,
            panel_mode: None,
            analysis_method: None,
            settings_section: None,
            metadata_expanded: Vec::new(),
            focus_mode: false,
            graph: GraphPrefs::default(),
        }
    }
}

pub struct Interface {
    path: PathBuf,
    state: State,
}

impl Interface {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("interface.json");
        let mut state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<State>(&json).ok())
            .unwrap_or_default();
        // Как `loadScale`: значение вне пределов — масштаб по умолчанию.
        if !(MIN_SCALE..=MAX_SCALE).contains(&state.scale) {
            state.scale = 1.0;
        }
        Interface { path, state }
    }

    pub fn right_open(&self) -> bool {
        self.state.right_open
    }

    pub fn set_right_open(&mut self, open: bool) {
        self.state.right_open = open;
        self.save();
    }

    pub fn panel_mode(&self) -> Option<&str> {
        self.state.panel_mode.as_deref()
    }

    pub fn set_panel_mode(&mut self, mode: &str) {
        self.state.panel_mode = Some(mode.to_owned());
        self.save();
    }

    pub fn analysis_method(&self) -> Option<&str> {
        self.state.analysis_method.as_deref()
    }

    pub fn set_analysis_method(&mut self, method: &str) {
        self.state.analysis_method = Some(method.to_owned());
        self.save();
    }

    pub fn settings_section(&self) -> Option<&str> {
        self.state.settings_section.as_deref()
    }

    pub fn set_settings_section(&mut self, section: &str) {
        if self.state.settings_section.as_deref() != Some(section) {
            self.state.settings_section = Some(section.to_owned());
            self.save();
        }
    }

    pub fn scale(&self) -> f64 {
        self.state.scale
    }

    /// `setInterfaceScale`: округление до сотых и пределы.
    pub fn set_scale(&mut self, scale: f64) -> bool {
        let next = ((scale * 100.0).round() / 100.0).clamp(MIN_SCALE, MAX_SCALE);
        if next == self.state.scale {
            return false;
        }
        self.state.scale = next;
        self.save();
        true
    }

    pub fn sidebar_open(&self) -> bool {
        self.state.sidebar_open
    }

    pub fn set_sidebar_open(&mut self, open: bool) {
        self.state.sidebar_open = open;
        self.save();
    }

    pub fn focus_mode(&self) -> bool {
        self.state.focus_mode
    }

    pub fn set_focus_mode(&mut self, focus: bool) {
        self.state.focus_mode = focus;
        self.save();
    }

    pub fn metadata_expanded(&self, path: &Path) -> bool {
        let key = path.to_string_lossy();
        self.state.metadata_expanded.iter().any(|p| *p == key)
    }

    pub fn set_metadata_expanded(&mut self, path: &Path, expanded: bool) {
        let key = path.to_string_lossy().into_owned();
        self.state.metadata_expanded.retain(|p| *p != key);
        if expanded {
            self.state.metadata_expanded.push(key);
        }
        self.save();
    }

    pub fn move_metadata(&mut self, from: &Path, to: &Path) {
        if self.metadata_expanded(from) && !self.metadata_expanded(to) {
            let from = from.to_string_lossy();
            self.state.metadata_expanded.retain(|p| *p != from);
            self.state.metadata_expanded.push(to.to_string_lossy().into_owned());
            self.save();
        }
    }

    pub fn graph(&self) -> GraphPrefs {
        self.state.graph
    }

    pub fn set_graph(&mut self, prefs: GraphPrefs) {
        if self.state.graph != prefs {
            self.state.graph = prefs;
            self.save();
        }
    }

    fn save(&self) {
        let Ok(json) = serde_json::to_string(&self.state) else { return };
        if let Err(e) = write_file_atomic_impl(&self.path, &json, None) {
            eprintln!("состояние интерфейса не сохранено: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_rounded_clamped_and_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let mut interface = Interface::load(dir.path());
        assert_eq!(interface.scale(), 1.0);
        assert!(interface.set_scale(1.0 + SCALE_STEP * 3.0));
        assert_eq!(interface.scale(), 1.15);
        assert!(interface.set_scale(9.0));
        assert_eq!(interface.scale(), MAX_SCALE);
        assert!(!interface.set_scale(9.0));
        interface.set_sidebar_open(false);
        let reloaded = Interface::load(dir.path());
        assert_eq!(reloaded.scale(), MAX_SCALE);
        assert!(!reloaded.sidebar_open());
    }
}
