use std::sync::Arc;

use aquilum_core::files::gate;
use aquilum_core::history::Source;
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::{Point, Size};
use masonry::layout::Length;
use masonry::widgets::SizedBox;

use super::{App, Outcome};
use crate::links::{self, Listing, Method, Mode, Request, Target};
use crate::system;
use crate::ui::backlinks;
use crate::ui::titlebar::window_controls;
use crate::ui::tokens::size;
use crate::ui::widgets::{IconButton, ItemAction, Pressed, Slot};

impl App {
    pub(super) fn start_index(&self) {
        let Some(root) = self.workspace.root.clone() else { return };
        let core = Arc::clone(&self.workspace.core);
        std::thread::spawn(move || {
            let canonical = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
            if let Err(error) = core.watcher.watch(&canonical) {
                eprintln!("наблюдение за базой не запущено: {error}");
            }
            if let Err(error) = core.search.prepare(&root.to_string_lossy()) {
                eprintln!("индекс базы не подготовлен: {error:?}");
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
            let request = Request {
                generation: self.panel_generation,
                workspace: workspace.to_path_buf(),
                document,
                mode: self.panel_mode(),
                method: self.panel_method(),
                text: self.editor_text.clone(),
            };
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
            let width = if open { size::BACKLINKS_WIDTH } else { 0.0 };
            chrome.panel.edit(root, |mut panel| SizedBox::set_width(&mut panel, Length::px(width)));
            let inset = crate::ui::controls_inset(open);
            chrome.controls_inset.edit(root, |mut spacer| SizedBox::set_width(&mut spacer, Length::px(inset)));
        }
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
            && let Some((_, target)) = self.panel_body.as_ref()?.items.iter().find(|(item, _)| *item == id)
        {
            let target = target.clone();
            let title = self.open_target(root, target, *new_tab);
            return Some(Outcome { title, ..Outcome::default() });
        }
        None
    }

    pub(super) fn open_target(&mut self, root: &mut RenderRoot, target: Target, new_tab: bool) -> bool {
        match target {
            Target::Note(path) => self.open_in(root, path, new_tab),
            Target::Url(url) if url.starts_with(crate::reader::quotes::SCHEME) => {
                self.open_reader_link(root, &url);
                false
            }
            Target::Url(url) => {
                system::open(&url);
                false
            }
            Target::Wiki(target) => {
                let Some(workspace) = self.tree.root().map(|w| w.to_string_lossy().into_owned()) else { return false };
                let source = self.note.as_ref().map_or_else(|| workspace.clone(), |n| n.path.to_string_lossy().into_owned());
                let indexed = self.workspace.core.search.resolve_wiki_links(&workspace, &source, std::slice::from_ref(&target));
                let resolved = indexed.ok().and_then(|found| found.paths.into_iter().next().flatten()).map(std::path::PathBuf::from);
                if let Some(path) = resolved.or_else(|| links::find_note(std::path::Path::new(&workspace), &target)) {
                    return self.open_in(root, path, new_tab);
                }
                let Some(path) = self.tree.root().and_then(|w| links::linked_file_path(w, &target)) else { return false };
                if !path.exists() {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(error) = gate::create(&self.workspace.core, &path, "", Source::Me, None) {
                        eprintln!("{} не создана: {error:?}", path.display());
                        return false;
                    }
                    self.tree.paths_changed(root, std::slice::from_ref(&path));
                }
                self.open_in(root, path, new_tab)
            }
        }
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
    }

    pub fn window_resized(&mut self, root: &mut RenderRoot, window: Size) {
        self.window = window;
        if let Some((layer, _)) = self.controls {
            root.reposition_layer(layer, Point::new(window.width - size::TITLEBAR_CONTROLS_INSET, 0.0));
        }
        if let Some((hero, _)) = self.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()) {
            hero.edit(root, |mut hero| crate::ui::book::Hero::set_viewport(&mut hero, window.height));
        }
    }
}
