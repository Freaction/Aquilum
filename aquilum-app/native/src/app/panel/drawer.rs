use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};

use crate::app::App;
use crate::interface::{Screen, Side};
use crate::ui::widgets::{Collapse, IconButton, Pressed};

impl App {
    pub(in crate::app) fn apply_panels(&mut self, root: &mut RenderRoot) {
        let (sidebar, panel) = self.visible_panels();
        self.shown = Some((sidebar, panel));
        if let Some(chrome) = &self.chrome {
            chrome.set_screen(root, self.interface.screen());
            chrome.sidebar.edit(root, |mut c| Collapse::set_open(&mut c, sidebar));
            chrome.panel.edit(root, |mut c| Collapse::set_open(&mut c, panel));
            chrome.controls_inset.edit(root, |mut c| Collapse::set_open(&mut c, !panel));
            let label = crate::i18n::t(crate::ui::panel_label(panel));
            root.edit_widget(chrome.panel_toggle, |mut w| IconButton::set_label(&mut w.downcast(), &label));
        }
        self.apply_widths(root);
    }

    pub(in crate::app) fn set_drawer(&mut self, root: &mut RenderRoot, drawer: Option<Side>) {
        let screen = self.interface.screen();
        if screen.compact && screen.drawer != drawer {
            self.interface.set_screen(Screen { drawer, ..screen });
            self.apply_panels(root);
        }
    }

    pub(in crate::app) fn toggle_drawer(&mut self, root: &mut RenderRoot, side: Side) {
        let drawer = (self.interface.screen().drawer != Some(side)).then_some(side);
        self.set_drawer(root, drawer);
    }

    pub(in crate::app) fn on_drawer_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        let Some(chrome) = &self.chrome else { return false };
        if action.downcast_ref::<Pressed>().is_none() {
            return false;
        }
        if id == chrome.drawer {
            self.toggle_drawer(root, Side::Sidebar);
        } else if id == chrome.scrim || (id == chrome.toggle_sidebar && self.compact()) {
            self.set_drawer(root, None);
        } else {
            return false;
        }
        true
    }
}
