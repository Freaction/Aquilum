use std::path::Path;
use std::time::{Duration, Instant};
use crate::ui::editor::TextEditor;

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, NewWidget, Widget, WidgetId};
use masonry::widgets::{TextAction};

use super::{App, Outcome};
use crate::file_ops;
use crate::frontmatter;
use crate::ui::note::{self, Caption, MetadataToggled, NoteView, TitleAction, TitleField};
use crate::ui::widgets::{Pressed, Slot};

impl App {
    fn metadata_state(&self) -> Option<bool> {
        frontmatter::end(&self.editor_text)?;
        Some(self.note.as_ref().is_some_and(|n| self.interface.metadata_expanded(&n.path)))
    }

    fn hidden_len(&self) -> usize {
        match self.metadata_state() {
            Some(false) => frontmatter::hidden_len(&self.editor_text),
            _ => 0,
        }
    }

    pub(super) fn note_widget(&mut self) -> NewWidget<dyn Widget> {
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

    fn scroll_geometry(&self, root: &RenderRoot) -> Option<(f64, f64, f64)> {
        let ids = self.note_ids.as_ref()?;
        let scroll = root.get_widget(ids.scroll.id())?;
        let body = root.get_widget(ids.body.id())?;
        let top = |t: masonry::kurbo::Affine| (t * masonry::kurbo::Point::ZERO).y;
        let offset = scroll.downcast::<crate::ui::scroll::ScrollArea<masonry::widgets::Flex>>()?.inner().offset();
        Some((top(scroll.ctx().window_transform()), top(body.ctx().window_transform()), offset))
    }

    pub(super) fn capture_view(&mut self, root: &RenderRoot) {
        let Some((viewport, editor, _)) = self.scroll_geometry(root) else { return };
        let Some(ids) = &self.note_ids else { return };
        let Some(body) = root.get_widget(ids.body.id()).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()) else { return };
        let Some((offset, px)) = body.inner().anchor_at(viewport - editor) else { return };
        let anchor = self.editor_text[..(self.hidden + offset).min(self.editor_text.len())].encode_utf16().count() as i64;
        self.view = Some((anchor, px));
        self.view_due = Some(Instant::now() + VIEW_SAVE_DELAY);
    }

    pub fn restore_view(&mut self, root: &mut RenderRoot) -> bool {
        let Some((anchor, px)) = self.restore else { return false };
        let Some((viewport, editor, offset)) = self.scroll_geometry(root) else { return false };
        let Some(ids) = &self.note_ids else { return false };
        let byte = utf16_to_byte(&self.editor_text, anchor).saturating_sub(self.hidden);
        let Some(top) = root.get_widget(ids.body.id()).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).and_then(|b| b.inner().top_of(byte)) else {
            return false;
        };
        let target = editor - viewport + offset + top + px;
        let scroll = ids.scroll;
        scroll.edit(root, |mut area| crate::ui::scroll::ScrollArea::set_offset(&mut area, target));
        self.restore = None;
        true
    }

    pub(super) fn flush_view(&mut self) {
        self.view_due = None;
        let (Some(session), Some(note), Some((anchor, px))) = (self.session.as_mut(), self.note.as_ref(), self.view) else { return };
        if let Some(document) = session.document(&self.workspace.core, &note.path) {
            session.save_view(&self.workspace.core, document, anchor, px);
        }
    }

    pub(super) fn render_note(&mut self, root: &mut RenderRoot) {
        let page = self.note_widget();
        if let Some(chrome) = &self.chrome {
            chrome.note_page.edit(root, |mut slot| Slot::set_child(&mut slot, page));
        }
    }

    pub(super) fn set_editor_text(&mut self, root: &mut RenderRoot, text: String) {
        self.editor_text = text;
        self.sync_body(root);
        self.refresh_hero(root);
    }

    pub(super) fn sync_body(&mut self, root: &mut RenderRoot) {
        self.hidden = self.hidden_len();
        let Some(ids) = &self.note_ids else { return };
        let visible = &self.editor_text[self.hidden..];
        ids.body.edit(root, |mut body| {
            if body.widget.text() != visible {
                crate::ui::editor::TextEditor::reset_text(&mut body, visible);
            }
        });
        let metadata = note::metadata_widget(self.metadata_state());
        ids.metadata.edit(root, |mut slot| Slot::set_child(&mut slot, metadata));
    }

    pub(super) fn refresh_note_names(&mut self, root: &mut RenderRoot) {
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

    pub(super) fn on_note_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<Outcome> {
        let ids = self.note_ids.as_ref()?;
        if id == ids.scroll.id() {
            if action.downcast_ref::<crate::ui::scroll::Scrolled>().is_some() {
                self.capture_view(root);
            }
            return Some(Outcome::default());
        }
        if id == ids.body.id() {
            if let Some(TextAction::Changed(visible)) = action.downcast_ref::<TextAction>() {
                self.body_changed(root, visible);
            }
            return Some(Outcome::default());
        }
        if id == ids.title.id() {
            match action.downcast_ref::<TextAction>() {
                Some(TextAction::Changed(text)) => {
                    let empty = text.is_empty();
                    ids.title_box.edit(root, |mut field| TitleField::set_empty(&mut field, empty));
                }
                Some(TextAction::Entered(text)) => {
                    let text = text.clone();
                    self.commit_title(root, &text);
                    if let Some(ids) = &self.note_ids {
                        let body = ids.body;
                        root.focus_on(Some(body.id()));
                        body.edit(root, |mut body| crate::ui::editor::TextEditor::select_byte_range(&mut body, 0, 0));
                    }
                }
                _ => {}
            }
            return Some(Outcome::default());
        }
        if id == ids.title_box.id() && action.downcast_ref::<TitleAction>().is_some() {
            let title = ids.title;
            let text = title.get(root).map(|t| t.inner().text().to_string()).unwrap_or_default();
            self.commit_title(root, &text);
            return Some(Outcome::default());
        }
        if action.downcast_ref::<Pressed>().is_some() {
            if id == ids.back {
                return Some(self.navigate(root, -1));
            }
            if id == ids.forward {
                return Some(self.navigate(root, 1));
            }
            if id == ids.focus {
                self.toggle_focus_mode(root);
                return Some(Outcome::default());
            }
        }
        if action.downcast_ref::<MetadataToggled>().is_some() {
            self.toggle_metadata(root);
            return Some(Outcome::default());
        }
        None
    }

    pub fn navigate(&mut self, root: &mut RenderRoot, delta: isize) -> Outcome {
        let Some(path) = self.navigation.step(delta) else { return Outcome::default() };
        if !path.is_file() {
            return Outcome::default();
        }
        self.tabs.open(path);
        self.show_active(root);
        Outcome { title: true, ..Outcome::default() }
    }

    pub fn toggle_focus_mode(&mut self, root: &mut RenderRoot) {
        let focus = !self.interface.focus_mode();
        self.interface.set_focus_mode(focus);
        self.replace_root(root);
        self.show_active(root);
    }

    fn body_changed(&mut self, root: &mut RenderRoot, visible: &str) {
        let had = frontmatter::end(&self.editor_text).is_some();
        let mut text = self.editor_text[..self.hidden].to_owned();
        text.push_str(visible);
        self.editor_text = text;
        if let Some(note) = &mut self.note {
            let debounce = self.workspace.core.settings.get_config().editor.save_debounce_ms;
            note.edit(self.editor_text.clone(), Duration::from_millis(u64::from(debounce)));
        }
        self.refresh_hero(root);
        let has = frontmatter::end(&self.editor_text).is_some();
        if has != had {
            if has && let Some(note) = &self.note {
                self.interface.set_metadata_expanded(&note.path, true);
            }
            let metadata = note::metadata_widget(self.metadata_state());
            if let Some(ids) = &self.note_ids {
                ids.metadata.edit(root, |mut slot| Slot::set_child(&mut slot, metadata));
            }
        }
    }

    fn toggle_metadata(&mut self, root: &mut RenderRoot) {
        let Some(expanded) = self.metadata_state() else { return };
        let Some(path) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        self.interface.set_metadata_expanded(&path, !expanded);
        self.sync_body(root);
    }

    fn commit_title(&mut self, root: &mut RenderRoot, input: &str) {
        let Some(from) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        match file_ops::rename_target(&from, input) {
            Some((to, _)) if self.renaming_to.as_ref() == Some(&to) => {}
            Some((to, _)) => {
                self.renaming_to = Some(to.clone());
                file_ops::rename(&self.workspace.core, from, to);
            }
            None => self.refresh_note_names(root),
        }
    }
}

const VIEW_SAVE_DELAY: Duration = Duration::from_millis(500);

fn utf16_to_byte(text: &str, units: i64) -> usize {
    let mut seen = 0i64;
    for (byte, c) in text.char_indices() {
        if seen >= units {
            return byte;
        }
        seen += c.len_utf16() as i64;
    }
    text.len()
}
