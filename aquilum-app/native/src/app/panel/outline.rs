use masonry::app::RenderRoot;

use super::super::{App, jobs};
use crate::links::{self, Item, Listing, Mode};
use crate::ui::widgets::DocumentItem;

impl App {
    pub(super) fn request_outline(&self) {
        let shown = self.tabs.active().path.as_deref().is_some_and(|path| self.shows(path));
        if !shown || !self.panel_visible() {
            return;
        }
        let (generation, text, hidden, cancel) = (self.panel_generation, self.text.clone(), self.hidden, self.outline_scan.next());
        jobs::deliver("фон: оглавление", self.panel_tx.clone(), &self.workspace.wake, move || links::outline(generation, text.byte_slice(hidden..), &cancel));
    }

    pub(in crate::app) fn refresh_outline(&mut self) {
        if self.panel_mode() == Mode::Outline {
            self.panel_generation += 1;
            self.request_outline();
        }
    }

    pub(super) fn keep_rows(&mut self, root: &mut RenderRoot, listing: &Listing) -> bool {
        let Some(old) = &self.listing else { return false };
        let same = |(a, b): (&Item, &Item)| a.counter == b.counter && a.depth == b.depth;
        if old.notice != listing.notice || old.items.len() != listing.items.len() || !old.items.iter().zip(&listing.items).all(same) {
            return false;
        }
        let Some(body) = &mut self.panel_body else { return false };
        for ((id, target, reasons), item) in body.items.iter_mut().zip(&listing.items) {
            root.edit_widget(*id, |mut row| DocumentItem::set_label(&mut row.downcast(), &item.title));
            (*target, *reasons) = (item.target.clone(), item.reasons.clone());
        }
        true
    }
}
