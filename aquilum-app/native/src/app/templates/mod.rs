pub mod dialog;
pub mod placeholders;
pub mod scan;

use std::path::{Path, PathBuf};

use aquilum_core::Core;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

use masonry::accesskit::Role;
use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers, NamedKey};
use masonry::core::{NewWidget, WidgetId};
use masonry::kurbo::{Axis, Point};
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;

use super::{App, Outcome, Popup};
use crate::i18n::t;
use crate::ui::dialog::{CardSize, Modal};
use crate::ui::editor::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::scroll::ScrollArea;
use crate::ui::search;
use crate::ui::theme;
use crate::ui::widgets::{Rule, Slot};

const NOTE_STARTER: &str = "---\ncreated: {{date:DD-MM-YYYY}} | ({{time:HH:mm}})\ntags: []\nauthor: \nsource: \n---\n\n\n\n\n***\n\nСсылки:\n- [[]]\n";
const BOOK_STARTER: &str = "---\ncreated: {{date:DD-MM-YYYY}} | ({{time:HH:mm}})\ncover: true\ntype: book\nauthor: \nstatus: to-read\npages: \ntags: [книга, ]\nrating: 0\nBook_cover: \nPage_cover: pattern:tunnel\nПуть к файлу: \nreader_position: \nread_percent: \n---\n\n\n\n***\n\nСсылки:\n- [[]]\n";

pub use placeholders::apply_placeholders;
pub use scan::NoteTemplate;

pub struct TemplatesState {
    pub layer: WidgetId,
    pub field: Handle<TextEditor>,
    pub body: Handle<Slot>,
    pub list: Option<Handle<ScrollArea<Flex>>>,
    pub templates: Vec<NoteTemplate>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub cards: Vec<WidgetId>,
    pub create_button: Option<WidgetId>,
}

pub(in crate::app) fn write_starters(core: &Core, folder: &Path) -> Vec<PathBuf> {
    if let Err(error) = std::fs::create_dir_all(folder) {
        report!("папка шаблонов не создана: {error}");
        return Vec::new();
    }
    let mut created = Vec::new();
    for (name, content) in [("Note.md", NOTE_STARTER), ("Book.md", BOOK_STARTER)] {
        let path = folder.join(name);
        if path.exists() {
            continue;
        }
        match gate::create(core, &path, content, Source::Me, None) {
            Ok(_) => created.push(path),
            Err(error) => report!("{} не создан: {error:?}", path.display()),
        }
    }
    created
}

impl App {
    pub fn open_templates(&mut self, root: &mut RenderRoot) {
        if matches!(self.popup, Some(Popup::Templates(_))) {
            return;
        }
        let search_box = search::search_box("");
        let body = Slot::new(NewWidget::new(Flex::column()).erased());
        let body_handle = Handle::of(&body);
        let card = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(search_box.widget)
            .with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default))
            .with(body, 1.0)
            .with_fixed(Rule::new(Axis::Horizontal, || theme::current().border_default))
            .with_fixed(search::footer());
        let layer = Modal::new(NewWidget::new(card), &t("templateDialog.title"), CardSize::Window(search::card_size), Role::Dialog, false);
        let layer_id = layer.id();
        root.add_layer(layer, Point::ORIGIN);
        root.focus_on(Some(search_box.field.id()));

        let templates = self.scan_templates();
        let filtered = (0..templates.len()).collect();
        self.popup = Some(Popup::Templates(TemplatesState {
            layer: layer_id,
            field: search_box.field,
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

    pub fn templates_key(&mut self, root: &mut RenderRoot, key: &Key, _modifiers: Modifiers) -> Option<Outcome> {
        let Some(Popup::Templates(state)) = &self.popup else { return None };
        let count = state.filtered.len();
        match key {
            Key::Named(NamedKey::ArrowDown) => {
                if count > 0 {
                    self.select_template(root, (state.selected + 1) % count);
                }
                Some(Outcome::default())
            }
            Key::Named(NamedKey::ArrowUp) => {
                if count > 0 {
                    self.select_template(root, (state.selected + count - 1) % count);
                }
                Some(Outcome::default())
            }
            Key::Named(NamedKey::Enter) => {
                if count > 0 {
                    Some(self.choose_template(root, state.selected))
                } else {
                    Some(Outcome::default())
                }
            }
            Key::Named(NamedKey::Escape) => {
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

        let content = std::fs::read_to_string(&template.path).unwrap_or_default();
        let content = apply_placeholders(&content);

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
                self.commit_editor_text(root, content);
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
            state.templates = scanned;
            state.filtered = (0..state.templates.len()).collect();
            state.selected = 0;
        }
        self.render_templates(root);
    }
}
