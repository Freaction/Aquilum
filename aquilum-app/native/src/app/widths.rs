use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};

use super::App;
use crate::interface::{Side, drawer_width};
use crate::ui::tokens::size;
use crate::ui::widgets::{Collapse, SashAction};

const MIN: f64 = 180.0;
const EDITOR_MIN: f64 = 360.0;

fn default(side: Side) -> f64 {
    match side {
        Side::Sidebar => size::SIDEBAR_WIDTH + size::BORDER_1,
        Side::Panel => size::BACKLINKS_WIDTH,
    }
}

fn other(side: Side) -> Side {
    match side {
        Side::Sidebar => Side::Panel,
        Side::Panel => Side::Sidebar,
    }
}

impl App {
    fn wanted(&self, side: Side) -> f64 {
        self.interface.width(side).unwrap_or(default(side)).max(MIN)
    }

    fn fit(&self, side: Side, width: f64) -> f64 {
        let (sidebar, panel) = self.visible_panels();
        let open = if other(side) == Side::Sidebar { sidebar } else { panel };
        let rail = if self.interface.focus_mode() { 0.0 } else { size::SIDEBAR_RAIL_WIDTH };
        let taken = rail + EDITOR_MIN + if open { self.wanted(other(side)) } else { 0.0 };
        let room = if self.window.width > 0.0 { self.room().width - taken } else { f64::INFINITY };
        width.min(room).max(MIN)
    }

    pub(in crate::app) fn panel_width(&self, side: Side) -> f64 {
        if self.compact() { drawer_width(self.room().width, side) } else { self.fit(side, self.wanted(side)) }
    }

    fn set_panel_width(&self, root: &mut RenderRoot, side: Side, width: f64) {
        let Some(chrome) = &self.chrome else { return };
        let handle = if side == Side::Sidebar { &chrome.sidebar } else { &chrome.panel };
        handle.edit(root, |mut collapse| Collapse::set_width(&mut collapse, width));
    }

    pub(in crate::app) fn apply_widths(&self, root: &mut RenderRoot) {
        for side in [Side::Sidebar, Side::Panel] {
            self.set_panel_width(root, side, self.panel_width(side));
        }
    }

    pub(in crate::app) fn on_sash(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> bool {
        let (Some(&sash), Some(chrome)) = (action.downcast_ref::<SashAction>(), &self.chrome) else { return false };
        let side = if id == chrome.sidebar_sash { Side::Sidebar } else if id == chrome.panel_parts.sash { Side::Panel } else { return false };
        if self.compact() {
            return true;
        }
        match sash {
            SashAction::Press => {
                let width = self.panel_width(side);
                self.resizing = Some((side, width, width));
            }
            SashAction::Drag(dx) => {
                let Some((_, start, _)) = self.resizing else { return true };
                let width = self.fit(side, start + if side == Side::Sidebar { dx } else { -dx });
                self.resizing = Some((side, start, width));
                self.set_panel_width(root, side, width);
            }
            SashAction::Release => {
                if let Some((_, start, width)) = self.resizing.take()
                    && width != start
                {
                    self.interface.set_width(side, Some(width));
                }
            }
            SashAction::Reset => {
                self.resizing = None;
                self.interface.set_width(side, None);
                self.apply_widths(root);
            }
        }
        true
    }
}
