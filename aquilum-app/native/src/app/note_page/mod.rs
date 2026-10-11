pub mod actions;
pub mod export;
pub mod menu;
pub mod view;

use std::path::Path;

use masonry::app::RenderRoot;
use masonry::core::{NewWidget, Widget};
use masonry::widgets::SizedBox;

use super::App;
use crate::frontmatter;
use crate::ui::editor::TextEditor;
use crate::ui::note::{self, Caption, NoteView, TitleField};
use crate::ui::widgets::Slot;

impl App {
    pub(in crate::app) fn metadata_state(&self) -> Option<bool> {
        frontmatter::end(&self.editor_text)?;
        Some(self.note.as_ref().is_some_and(|n| self.interface.metadata_expanded(&n.path)))
    }

    pub(in crate::app) fn hidden_len(&self) -> usize {
        match self.metadata_state() {
            Some(false) => frontmatter::hidden_len(&self.editor_text),
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
            body: &self.editor_text[self.hidden..],
            metadata: self.metadata_state(),
            can_back: self.navigation.can_back(),
            can_forward: self.navigation.can_forward(),
            focus_mode: self.interface.focus_mode(),
            hero: self.hero_view(),
            smart: crate::ui::editor::Smart::from_config(&self.workspace.core.settings.get_config()),
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

    fn stored_view(&self) -> Option<(i64, f64)> {
        let (session, note) = (self.session.as_ref()?, self.note.as_ref()?);
        let document = session.document(&self.workspace.core, &note.path)?;
        session.load_view(&self.workspace.core, document)
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
        let _ = note.retry_save(&self.workspace.core, &self.editor_text);
        self.refresh_save_error(root);
    }

    pub(in crate::app) fn set_editor_text(&mut self, root: &mut RenderRoot, text: String) {
        self.editor_text = text;
        self.sync_body(root);
        self.refresh_hero(root);
    }

    pub(in crate::app) fn sync_body(&mut self, root: &mut RenderRoot) {
        let hidden = self.hidden_len();
        let reveal = hidden == self.hidden;
        self.hidden = hidden;
        let Some(ids) = &self.note_ids else { return };
        let visible = &self.editor_text[self.hidden..];
        ids.body.edit(root, |mut body| {
            let prev = body.widget.text().to_owned();
            if prev != visible {
                let prev_len = prev.len();
                crate::ui::editor::TextEditor::reset_text(&mut body, visible);
                if reveal && visible.len() > prev_len {
                    let prefix = prev.bytes().zip(visible.bytes()).take_while(|(a, b)| a == b).count();
                    let inserted_len = visible.len() - prev_len;
                    crate::ui::editor::TextEditor::reveal_external_insert(&mut body, prefix..prefix + inserted_len);
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
            if title.widget.text().to_string() != stem {
                TextEditor::reset_text(&mut title, &stem);
            }
        });
        ids.title_box.edit(root, |mut field| TitleField::set_empty(&mut field, stem.is_empty()));
    }
}
