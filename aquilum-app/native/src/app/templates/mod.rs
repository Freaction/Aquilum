pub mod dialog;
pub mod placeholders;
pub mod scan;
mod starters;

use std::path::PathBuf;

use masonry::accesskit::Role;
use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers};
use crate::ui::keymap::{self, Command, Scope};
use masonry::core::{NewWidget, WidgetId};
use masonry::kurbo::Point;
use masonry::widgets::Flex;

use super::{App, Outcome, Popup};
use crate::i18n::t;
use crate::ui::dialog::{CardSize, Modal};
use crate::ui::editor::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::scroll::ScrollArea;
use crate::ui::search::{self, Placeholder};
use crate::ui::widgets::Slot;

pub use placeholders::apply_placeholders;
pub use scan::NoteTemplate;
pub use starters::write_starters;
use starters::template_text;

pub struct TemplatesState {
    pub layer: WidgetId,
    pub field: Handle<TextEditor>,
    pub placeholder: Handle<Placeholder>,
    pub query: String,
    pub body: Handle<Slot>,
    pub list: Option<Handle<ScrollArea<Flex>>>,
    pub templates: Vec<NoteTemplate>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub cards: Vec<WidgetId>,
    pub create_button: Option<WidgetId>,
}

impl App {
    pub fn open_templates(&mut self, root: &mut RenderRoot) {
        if matches!(self.popup, Some(Popup::Templates(_))) {
            return;
        }
        let search_box = search::find_box(t("templateDialog.find"));
        let body = Slot::fit(NewWidget::new(Flex::column()).erased());
        let body_handle = Handle::of(&body);
        let card = search::dialog_card(search_box.widget, body, search::template_footer());
        let layer = Modal::new(card, &t("templateDialog.title"), CardSize::Anchored(search::card_size), Role::Dialog, false);
        let layer_id = layer.id();
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(search_box.field.id()));

        let templates = self.scan_templates();
        let filtered = (0..templates.len()).collect();
        self.popup = Some(Popup::Templates(TemplatesState {
            layer: layer_id,
            field: search_box.field,
            placeholder: search_box.placeholder,
            query: String::new(),
            body: body_handle,
            list: None,
            templates,
            filtered,
            selected: 0,
            cards: Vec::new(),
            create_button: None,
        }));
        self.render_templates(root);
    }

    pub fn close_templates(&mut self, root: &mut RenderRoot) {
        if let Some(Popup::Templates(state)) = self.popup.take() {
            crate::ui::dialog::close(root, state.layer);
            if let Some(ids) = &self.note_ids {
                root.focus_on(Some(ids.body.id()));
            }
        }
    }

    pub(super) fn templates_dir(&self) -> Option<PathBuf> {
        Some(crate::ui::settings::schema::templates_path(&self.tree.root()?, &self.workspace.core.settings.get_config().templates.folder))
    }

    fn scan_templates(&self) -> Vec<NoteTemplate> {
        let Some(dir) = self.templates_dir() else { return Vec::new() };
        if !dir.is_dir() {
            return Vec::new();
        }
        let mut list = Vec::new();
        scan::scan_dir(&dir, &dir, &mut list);
        list.sort_by(|a, b| a.relative.to_lowercase().cmp(&b.relative.to_lowercase()));
        list
    }

    pub fn templates_key(&mut self, root: &mut RenderRoot, key: &Key, modifiers: Modifiers) -> Option<Outcome> {
        let Some(Popup::Templates(state)) = &self.popup else { return None };
        let count = state.filtered.len();
        match keymap::command(&[Scope::List], key, modifiers)? {
            command @ (Command::ListNext | Command::ListPrevious) => {
                if count > 0 {
                    self.select_template(root, keymap::cycle(state.selected, count, command == Command::ListNext));
                }
                Some(Outcome::default())
            }
            Command::ListOpen => {
                if count > 0 {
                    Some(self.choose_template(root, state.selected))
                } else {
                    Some(Outcome::default())
                }
            }
            Command::Close => {
                self.close_templates(root);
                Some(Outcome::default())
            }
            _ => None,
        }
    }

    pub(super) fn choose_template(&mut self, root: &mut RenderRoot, index: usize) -> Outcome {
        let Some(Popup::Templates(state)) = &self.popup else { return Outcome::default() };
        let Some(&tmpl_idx) = state.filtered.get(index) else { return Outcome::default() };
        let template = state.templates[tmpl_idx].clone();
        self.close_templates(root);

        let content = template_text(&template.path);

        if self.note.is_some() {
            if let Some(ids) = &self.note_ids {
                let body = ids.body;
                root.focus_on(Some(body.id()));
                body.edit(root, |mut e| TextEditor::insert_text(&mut e, &content));
            }
            Outcome::default()
        } else {
            let title = template.name;
            let opened = self.create_note_titled(root, Some(&title));
            if opened {
                self.commit_text(root, crop::Rope::from(content));
            }
            Outcome { title: opened, ..Outcome::default() }
        }
    }

    pub(super) fn create_starter_templates(&mut self, root: &mut RenderRoot) {
        let Some(dir) = self.templates_dir() else { return };
        let created = write_starters(&self.workspace.core, &dir);
        self.tree.paths_changed(root, &created);
        let scanned = self.scan_templates();
        if let Some(Popup::Templates(state)) = &mut self.popup {
            state.filtered = dialog::filtered(&scanned, &state.query);
            state.templates = scanned;
            state.selected = 0;
        }
        self.render_templates(root);
    }
}

