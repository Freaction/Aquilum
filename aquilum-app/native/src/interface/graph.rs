use serde::{Deserialize, Serialize};

use super::Interface;

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

impl Interface {
    pub fn graph(&self) -> GraphPrefs {
        self.state.graph
    }

    pub fn set_graph(&mut self, prefs: GraphPrefs) {
        if self.state.graph != prefs {
            self.state.graph = prefs;
            self.save();
        }
    }
}
