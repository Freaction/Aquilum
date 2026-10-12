use std::path::Path;

use super::Interface;

impl Interface {
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
}
