use super::Interface;

impl Interface {
    pub fn update_dismissed(&self, version: &str) -> bool {
        self.state.dismissed_update.as_deref() == Some(version)
    }

    pub fn dismiss_update(&mut self, version: &str) {
        self.state.dismissed_update = Some(version.to_owned());
        self.save();
    }
}
