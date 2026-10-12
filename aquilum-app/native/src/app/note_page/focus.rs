use masonry::app::RenderRoot;

use super::super::App;
use crate::ui::widgets::{Collapse, Slot, TextButton, Visible};

impl App {
    pub fn toggle_focus_mode(&mut self, root: &mut RenderRoot) {
        let focus = !self.interface.focus_mode();
        self.interface.set_focus_mode(focus);
        let (sidebar, panel) = self.visible_panels();
        self.shown = Some((sidebar, panel));
        let (home, home_button) = crate::ui::home_widget(self.home.is_some() && !focus);
        if let Some(chrome) = &mut self.chrome {
            chrome.sidebar.edit(root, |mut c| Collapse::set_open(&mut c, sidebar));
            chrome.panel.edit(root, |mut c| Collapse::set_open(&mut c, panel));
            chrome.controls_inset.edit(root, |mut c| Collapse::set_open(&mut c, !panel));
            for shown in &chrome.focus_hidden {
                shown.edit(root, |mut v| Visible::set_shown(&mut v, !focus));
            }
            chrome.focus_shown.edit(root, |mut v| Visible::set_shown(&mut v, focus));
            chrome.home_slot.edit(root, |mut slot| Slot::set_child(&mut slot, home));
            chrome.home_button = home_button;
        }
        if let Some(ids) = &self.note_ids {
            for shown in &ids.focus_hidden {
                shown.edit(root, |mut v| Visible::set_shown(&mut v, !focus));
            }
            ids.focus_button.edit(root, |mut b| TextButton::set_pressed(&mut b, focus));
        }
        if panel {
            self.request_panel(root);
        }
    }
}
