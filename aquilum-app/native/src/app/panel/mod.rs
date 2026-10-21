mod open;

use std::sync::Arc;

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::{Point, Size};

use aquilum_core::files::document::head;

use super::{App, Outcome};
use crate::links::{self, Listing, Method, Mode, Request};
use crate::ui::backlinks;
use crate::ui::titlebar::window_controls;
use crate::ui::tokens::size;
use crate::ui::widgets::{IconButton, ItemAction, Pressed, Slot};

impl App {
    pub(in crate::app) fn bind_sync(&self) {
        let core = &self.workspace.core;
        let vault = self.workspace.root.clone().and_then(|root| core.ui_state.workspace(&root).map(|known| (root, known.sync)));
        core.sync.set_vault(vault, crate::dates::offset_seconds());
    }

    pub(super) fn start_index(&self) {
        self.bind_sync();
        let Some(root) = self.workspace.root.clone() else { return };
        let core = Arc::clone(&self.workspace.core);
        std::thread::spawn(move || {
            let canonical = aquilum_core::search::paths::canonical_path(&root);
            if let Err(error) = core.watcher.watch(&canonical) {
                report!("наблюдение за базой не запущено: {error}");
            }
            if let Err(error) = core.search.prepare(&root.to_string_lossy()) {
                report!("индекс базы не подготовлен: {error:?}");
            }
        });
    }

    pub(super) fn panel_visible(&self) -> bool {
        self.interface.right_open() && !self.interface.focus_mode()
    }

    pub(super) fn panel_methods(&self) -> Vec<Method> {
        Method::enabled(&self.workspace.core.settings.get_config())
    }

    pub(super) fn panel_mode(&self) -> Mode {
        let mode = self.interface.panel_mode().and_then(Mode::from_key).unwrap_or(Mode::Backlinks);
        if mode == Mode::Analysis && self.panel_methods().is_empty() { Mode::Backlinks } else { mode }
    }

    fn panel_method(&self) -> Method {
        let methods = self.panel_methods();
        let stored = self.interface.analysis_method().and_then(Method::from_key).unwrap_or(Method::Bm25f);
        if methods.contains(&stored) { stored } else { methods.first().copied().unwrap_or(stored) }
    }

    pub(super) fn request_panel(&mut self, root: &mut RenderRoot) {
        self.panel_generation += 1;
        let key = self.tabs.active().path.clone().map(|path| (path, self.panel_mode(), self.panel_method()));
        let refresh = self.listing.is_some() && key.is_some() && key == self.panel_key && self.panel_visible();
        self.panel_key = key;
        if !refresh {
            self.listing = None;
        }
        if self.panel_mode() == Mode::History {
            self.update_mode_buttons(root);
            if self.panel_visible() {
                self.load_history(root);
            }
            return;
        }
        let document = self.tabs.active().path.clone();
        if let (true, Some(workspace), Some(document)) = (self.panel_visible(), self.tree.root(), document) {
            let (mode, method) = (self.panel_mode(), self.panel_method());
            let text = match (mode, method) {
                (Mode::Analysis, Method::Wikixiv) => head(&crate::rope::Text(&self.text)).unwrap_or_default(),
                _ => String::new(),
            };
            let request = Request { generation: self.panel_generation, workspace: workspace.to_path_buf(), document, mode, method, text };
            links::spawn(Arc::clone(&self.workspace.core), request, self.panel_tx.clone(), Arc::clone(&self.workspace.wake));
        }
        if !refresh {
            self.render_panel(root);
        }
    }

    fn update_mode_buttons(&self, root: &mut RenderRoot) {
        let Some(chrome) = &self.chrome else { return };
        let mode = self.panel_mode();
        for (button, item) in &chrome.panel_parts.modes {
            button.edit(root, |mut b| IconButton::set_pressed(&mut b, *item == mode));
        }
    }

    pub(super) fn render_panel(&mut self, root: &mut RenderRoot) {
        self.update_mode_buttons(root);
        let mode = self.panel_mode();
        if mode == Mode::History {
            self.render_history(root);
            return;
        }
        let Some(chrome) = &self.chrome else { return };
        let (body, bindings) = backlinks::body(mode, &self.panel_methods(), self.panel_method(), self.listing.as_ref());
        chrome.panel_parts.body.edit(root, |mut slot| Slot::set_child(&mut slot, body));
        self.panel_body = Some(bindings);
    }

    pub(super) fn on_panel_results(&mut self, root: &mut RenderRoot) {
        let latest: Option<Listing> = self.panel_rx.try_iter().filter(|l| l.generation == self.panel_generation).last();
        if let Some(listing) = latest {
            let same = self.listing.as_ref().is_some_and(|old| old.items == listing.items && old.notice == listing.notice);
            self.listing = Some(listing);
            if !same {
                self.render_panel(root);
            }
        }
    }

    fn toggle_panel(&mut self, root: &mut RenderRoot) {
        let open = !self.interface.right_open();
        self.interface.set_right_open(open);
        if let Some(chrome) = &self.chrome {
            chrome.panel.edit(root, |mut panel| crate::ui::widgets::Collapse::set_open(&mut panel, open));
            self.shown = Some(self.visible_panels());
            chrome.controls_inset.edit(root, |mut spacer| crate::ui::widgets::Collapse::set_open(&mut spacer, !open));
            let label = crate::i18n::t(crate::ui::panel_label(open));
            root.edit_widget(chrome.panel_toggle, |mut w| crate::ui::widgets::IconButton::set_label(&mut w.downcast(), &label));
        }
        self.apply_widths(root);
        self.request_panel(root);
    }

    pub(super) fn on_panel_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<Outcome> {
        let chrome = self.chrome.as_ref()?;
        if action.downcast_ref::<Pressed>().is_some() {
            if id == chrome.panel_toggle {
                self.toggle_panel(root);
                return Some(Outcome::default());
            }
            if let Some(&(_, mode)) = chrome.panel_parts.modes.iter().find(|(b, _)| b.id() == id) {
                self.interface.set_panel_mode(mode.key());
                self.request_panel(root);
                return Some(Outcome::default());
            }
            if let Some(&(_, method)) = self.panel_body.as_ref()?.chips.iter().find(|(chip, _)| *chip == id) {
                self.interface.set_analysis_method(method.key());
                self.request_panel(root);
                return Some(Outcome::default());
            }
        }
        if let Some(ItemAction::Open { new_tab }) = action.downcast_ref::<ItemAction>()
            && let Some((_, target, _)) = self.panel_body.as_ref()?.items.iter().find(|(item, _, _)| *item == id)
        {
            let target = target.clone();
            let title = self.open_target(root, target, *new_tab);
            return Some(Outcome { title, ..Outcome::default() });
        }
        if let Some(ItemAction::Hover(hovered)) = action.downcast_ref::<ItemAction>()
            && let Some((_, _, reasons)) = self.panel_body.as_ref()?.items.iter().find(|(item, _, _)| *item == id)
        {
            let terms = if *hovered { reasons.clone() } else { Vec::new() };
            if let Some(ids) = &self.note_ids {
                ids.body.edit(root, |mut ed| crate::ui::editor::TextEditor::set_hover_terms(&mut ed, terms));
            }
            return Some(Outcome::default());
        }
        None
    }

    pub fn attach_window_controls(&mut self, root: &mut RenderRoot, window: Size) {
        if cfg!(target_os = "macos") {
            return;
        }
        if let Some((layer, _)) = self.controls.take() {
            root.remove_layer(layer);
        }
        let (widget, maximize) = window_controls(self.maximized);
        let layer = widget.id();
        root.add_layer(widget, Point::new(window.width - size::TITLEBAR_CONTROLS_INSET, 0.0));
        self.controls = Some((layer, maximize));
        self.window = window;
        self.apply_widths(root);
    }

    pub fn window_resized(&mut self, root: &mut RenderRoot, window: Size) {
        self.window = window;
        self.apply_widths(root);
        if let Some((layer, _)) = self.controls {
            root.reposition_layer(layer, Point::new(window.width - size::TITLEBAR_CONTROLS_INSET, 0.0));
        }
        if let Some((hero, _)) = self.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()) {
            hero.edit(root, |mut hero| crate::ui::book::Hero::set_viewport(&mut hero, window.height));
        }
    }
}
