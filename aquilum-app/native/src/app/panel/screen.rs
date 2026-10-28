use masonry::app::RenderRoot;
use masonry::kurbo::{Insets, Point, Size};
use masonry::layout::Length;
use masonry::properties::Padding;

use crate::app::App;
use crate::interface::{Screen, WidthClass};
use crate::ui::editor::TextEditor;
use crate::ui::titlebar::{OWN_CONTROLS, window_controls};
use crate::ui::tokens::size;

fn controls_origin(window: Size) -> Point {
    Point::new(window.width - size::TITLEBAR_CONTROLS_INSET, 0.0)
}

impl App {
    pub(in crate::app) fn compact(&self) -> bool {
        self.interface.screen().compact
    }

    pub(in crate::app) fn room(&self) -> Size {
        let insets = self.interface.screen().insets;
        Size::new((self.window.width - insets.x0 - insets.x1).max(0.0), (self.window.height - insets.y0 - insets.y1).max(0.0))
    }

    pub fn attach_window_controls(&mut self, root: &mut RenderRoot, window: Size) {
        if OWN_CONTROLS {
            if let Some((layer, _)) = self.controls.take() {
                root.remove_layer(layer);
            }
            let (widget, maximize) = window_controls(self.maximized);
            self.controls = Some((widget.id(), maximize));
            root.add_layer(widget, controls_origin(window));
        }
        self.window_resized(root, window);
    }

    pub fn window_resized(&mut self, root: &mut RenderRoot, window: Size) {
        self.fit_window(root, window, self.interface.screen().insets);
    }

    pub fn fit_window(&mut self, root: &mut RenderRoot, window: Size, insets: Insets) {
        let before = self.interface.screen();
        let compact = window.width > 0.0 && WidthClass::of(window.width) == WidthClass::Compact;
        let drawer = if compact == before.compact { before.drawer } else { None };
        let class = compact != before.compact;
        let hidden = !self.shown.is_some_and(|(_, panel)| panel);
        self.window = window;
        self.interface.set_screen(Screen { compact, drawer, insets, ..before });
        self.apply_panels(root, false);
        self.fit_settings(root);
        self.fit_reader(root);
        if class {
            self.apply_focus(root);
            self.fit_note_page(root);
        }
        if class || hidden && self.panel_visible() {
            self.request_panel(root);
        }
        if let Some((layer, _)) = self.controls {
            root.reposition_layer(layer, controls_origin(self.window));
        }
        let height = self.room().height;
        if let Some((hero, _)) = self.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()) {
            hero.edit(root, |mut hero| crate::ui::book::Hero::set_viewport(&mut hero, height));
        }
        if insets.y1 > before.insets.y1 {
            self.keep_caret(root);
        }
    }

    fn fit_note_page(&mut self, root: &mut RenderRoot) {
        let Some(ids) = &self.note_ids else { return };
        let (pad_left, pad_right) = if self.compact() {
            (size::SPACE_12, size::SPACE_4)
        } else {
            (size::EDITOR_PADDING_X, size::EDITOR_PADDING_X)
        };
        ids.wrapper.edit(root, |mut flex| {
            flex.insert_prop(Padding {
                left: Length::px(pad_left),
                right: Length::px(pad_right),
                top: Length::ZERO,
                bottom: Length::ZERO,
            });
        });
    }

    fn keep_caret(&self, root: &mut RenderRoot) {
        let Some(ids) = &self.note_ids else { return };
        if root.focused_widget() == Some(ids.body.id()) {
            ids.body.edit(root, |mut body| TextEditor::keep_caret(&mut body));
        }
    }
}
