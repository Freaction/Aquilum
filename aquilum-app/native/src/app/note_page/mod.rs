pub mod actions;
pub mod edit;
pub mod export;
pub mod focus;
pub mod menu;
pub mod text;
pub mod view;
pub mod wide;

use std::path::Path;

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget};
use masonry::widgets::SizedBox;

use super::App;
use crate::frontmatter;
use crate::ui::editor::TextEditor;
use crate::ui::note::{self, NoteView, TitleField};
use crate::ui::widgets::{Caption, Slot};

impl App {
    pub(in crate::app) fn metadata_state(&self) -> Option<bool> {
        frontmatter::bounds_of(&crate::rope::Text(&self.text))?;
        Some(self.note.as_ref().is_some_and(|n| self.interface.metadata_expanded(&n.path)))
    }

    pub(in crate::app) fn hidden_len(&self) -> usize {
        match self.metadata_state() {
            Some(false) => frontmatter::bounds_of(&crate::rope::Text(&self.text)).map_or(0, |found| found.body),
            _ => 0,
        }
    }

    pub(in crate::app) fn note_widget(&mut self) -> NewWidget<dyn Widget> {
        let path = self.note.as_ref().map(|n| n.path.clone());
        let file_name = path.as_deref().and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let title = path.as_deref().and_then(Path::file_stem).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        self.hidden = self.hidden_len();
        let resolver = self.resolver(path.as_deref());
        let view = NoteView {
            resolver,
            file_name: &file_name,
            title: &title,
            body: self.body(),
            metadata: self.metadata_state(),
            can_back: self.navigation.can_back(),
            can_forward: self.navigation.can_forward(),
            focus_mode: self.interface.focus_mode(),
            wide: self.interface.wide(),
            hero: self.hero_view(),
            smart: crate::ui::editor::Smart::from_config(&self.workspace.core.settings.get_config()),
            wake: std::sync::Arc::clone(&self.workspace.wake),
        };
        self.hero_shown = self.hero_key();
        let (page, ids) = note::page(view);
        self.note_ids = Some(ids);
        self.restore = self.view.or_else(|| self.stored_view());
        page.erased()
    }

    fn resolver(&mut self, path: Option<&Path>) -> Option<std::sync::Arc<dyn crate::ui::editor::resolve::Resolver>> {
        let (path, root) = (path?, self.tree.root()?.to_path_buf());
        if self.embeds.as_ref().is_none_or(|e| e.document() != path || e.root() != root) {
            self.embeds = Some(super::embeds::Embeds::new(std::sync::Arc::clone(&self.workspace.core), root, path.to_path_buf(), self.workspace.data_dir.join("remote"), std::sync::Arc::clone(&self.workspace.wake)));
        }
        self.embeds.clone().map(|e| std::sync::Arc::new(e) as std::sync::Arc<dyn crate::ui::editor::resolve::Resolver>)
    }

    fn stored_view(&self) -> Option<(usize, f64)> {
        let (session, note) = (self.session.as_ref()?, self.note.as_ref()?);
        let document = session.document(&self.workspace.core, &note.path)?;
        let (units, px) = session.load_view(&self.workspace.core, document)?;
        Some((self.byte_of(usize::try_from(units).unwrap_or(0)), px))
    }

    pub(in crate::app) fn render_note(&mut self, root: &mut RenderRoot) {
        self.find = None;
        let page = self.note_widget();
        if let Some(chrome) = &self.chrome {
            chrome.note_page.edit(root, |mut slot| Slot::set_child(&mut slot, page));
        }
        self.refresh_save_error(root);
    }

    pub(in crate::app) fn refresh_save_error(&mut self, root: &mut RenderRoot) {
        let Some(ids) = &self.note_ids else { return };
        if let Some(note) = &self.note
            && let Some(err) = &note.save_error
        {
            let (banner, save_ids) = note::save_error_banner(err);
            self.save_error_ids = Some(save_ids);
            ids.error_slot.edit(root, |mut slot| Slot::set_child(&mut slot, banner.erased()));
        } else {
            self.save_error_ids = None;
            ids.error_slot.edit(root, |mut slot| Slot::set_child(&mut slot, NewWidget::new(SizedBox::empty()).erased()));
        }
    }

    pub(in crate::app) fn retry_save(&mut self, root: &mut RenderRoot) {
        let Some(note) = &mut self.note else { return };
        note.edit(std::time::Duration::ZERO);
        self.refresh_save_error(root);
    }

    pub(in crate::app) fn set_text(&mut self, root: &mut RenderRoot, text: crop::Rope) {
        self.text = text;
        self.sync_body(root);
        self.refresh_hero(root);
    }

    pub(in crate::app) fn sync_body(&mut self, root: &mut RenderRoot) {
        let hidden = self.hidden_len();
        let reveal = hidden == self.hidden;
        self.hidden = hidden;
        let Some(ids) = &self.note_ids else { return };
        let visible = self.body();
        ids.body.edit(root, |mut body| {
            let prev = body.widget.rope();
            let change = crate::rope::changed(&prev, &visible);
            if change.removed + change.inserted > 0 {
                crate::ui::editor::TextEditor::reset_rope(&mut body, visible.clone());
                if reveal && visible.byte_len() > prev.byte_len() {
                    crate::ui::editor::TextEditor::reveal_external_insert(&mut body, change.at..change.at + change.inserted);
                }
            }
        });
        let metadata = note::metadata_widget(self.metadata_state());
        ids.metadata.edit(root, |mut slot| Slot::set_child(&mut slot, metadata));
    }

    pub(in crate::app) fn refresh_note_names(&mut self, root: &mut RenderRoot) {
        self.renaming_to = None;
        let (Some(ids), Some(note)) = (&self.note_ids, &self.note) else { return };
        let name = note.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let stem = note.path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        ids.name.edit(root, |mut caption| Caption::set_text(&mut caption, &name));
        ids.title.edit(root, |mut title| {
            if title.widget.text() != stem {
                TextEditor::reset_text(&mut title, &stem);
            }
        });
        ids.title_box.edit(root, |mut field| TitleField::set_empty(&mut field, stem.is_empty()));
    }
}
