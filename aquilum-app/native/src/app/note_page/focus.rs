use masonry::app::RenderRoot;

use super::super::App;
use crate::ui::widgets::{Collapse, TextButton, Visible};

impl App {
    pub fn toggle_focus_mode(&mut self, root: &mut RenderRoot) {
        if self.compact() {
            return;
        }
        self.interface.set_focus_mode(!self.interface.focus_mode());
        self.apply_focus(root);
        if self.panel_visible() {
            self.request_panel(root);
        }
    }

    pub(in crate::app) fn apply_focus(&mut self, root: &mut RenderRoot) {
        let focus = self.interface.focus_mode();
        if let Some(chrome) = &self.chrome {
            chrome.rail.edit(root, |mut c| Collapse::set_open(&mut c, !focus, true));
            for shown in &chrome.focus_hidden {
                shown.edit(root, |mut v| Visible::set_shown(&mut v, !focus));
            }
            chrome.focus_shown.edit(root, |mut v| Visible::set_shown(&mut v, focus));
        }
        if let Some(ids) = &self.note_ids {
            for shown in &ids.focus_hidden {
                shown.edit(root, |mut v| Visible::set_shown(&mut v, !focus));
            }
            ids.focus_button.edit(root, |mut b| TextButton::set_pressed(&mut b, focus));
            ids.focus_shown.edit(root, |mut v| Visible::set_shown(&mut v, !self.compact()));
        }
        self.apply_panels(root, true);
    }
}
