mod hero;
mod import;

use std::path::Path;

use aquilum_core::history::Source;
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};
use masonry::kurbo::Rect;

use super::{App, MenuCommand, Outcome, Popup};
use crate::cover;
use crate::frontmatter::{self, BOOK_COVER, BOOK_FILE, COVER, PAGE_COVER, PAGE_COVER_POSITION, PAGES, READER_POSITION, READ_PERCENT};
use crate::i18n::t;
use crate::ui::book::{CoverAction, ThumbAction};
use crate::ui::menu::{Entry, Menu};
use crate::ui::widgets::Pressed;

pub(super) use import::{Picked, synthetic_pages};

pub(super) const BOOK_EXTENSIONS: [&str; 7] = ["epub", "fb2", "mobi", "azw3", "azw", "prc", "pdf"];

#[derive(Clone, Copy, PartialEq)]
pub(in crate::app) enum BookCommand {
    Upload,
    Remove,
}

impl App {
    pub(super) fn set_fields(&mut self, root: &mut RenderRoot, fields: &[(&str, String)]) {
        let Some(path) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        self.write_fields(root, &path, Source::Me, fields);
    }

    pub(super) fn write_fields(&mut self, root: &mut RenderRoot, path: &Path, source: Source, fields: &[(&str, String)]) {
        let change = |text: &dyn aquilum_core::text::content::Content| frontmatter::set(&frontmatter::head(text), fields);
        if !self.change_note(root, path, source, change) {
            return;
        }
        if self.shows(path) {
            self.hero_shown = None;
            self.refresh_hero(root);
        }
    }

    pub(super) fn on_book_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<Outcome> {
        let (_, ids) = &self.note_ids.as_ref()?.hero;
        let pressed = action.downcast_ref::<Pressed>().is_some();
        let chosen = ids.actions.iter().find(|(button, _)| *button == id).map(|&(_, chosen)| chosen).filter(|_| pressed);
        let (more, replace_book, cover) = (ids.more, ids.replace_book, ids.cover);
        let thumbnail = ids.thumbnail.map(|t| t.id());
        let book_button = ids.book_button.map(|b| b.id());
        let book_menu = ids.book_menu;
        if let Some(chosen) = chosen {
            self.cover_action(root, chosen);
        } else if pressed && Some(id) == more {
            self.open_cover_menu(root, id);
        } else if pressed && Some(id) == replace_book {
            self.pick_image(false);
        } else if pressed && Some(id) == book_button {
            self.book_pressed(root);
        } else if pressed && Some(id) == book_menu {
            self.open_book_menu(root, id);
        } else if Some(id) == thumbnail {
            match action.downcast_ref::<ThumbAction>()? {
                ThumbAction::Remove => self.set_fields(root, &[(BOOK_COVER, String::new())]),
                ThumbAction::Paste => self.paste_book_cover(root),
            }
        } else {
            if let Some(cover) = cover
                && cover.get(root).is_some_and(|c| c.inner().repositioning())
            {
                self.toggle_reposition(root);
            }
            return None;
        }
        Some(Outcome::default())
    }

    fn open_book_menu(&mut self, root: &mut RenderRoot, button: WidgetId) {
        let labels = [t("book.replaceFile"), t("book.removeFile")];
        let has_file = self.fields().is_some_and(|f| f.text(BOOK_FILE).is_some_and(|s| !s.trim().is_empty()));
        let entries = [
            Entry { label: &labels[0], icon: Some(crate::ui::icons::UPLOAD), disabled: false, divided: false },
            Entry { label: &labels[1], icon: Some(crate::ui::icons::TRASH), disabled: !has_file, divided: true },
        ];
        let menu = Menu::entries(&t("book.fileActions"), &entries);
        let trigger = root.get_widget(button).map_or(Rect::ZERO, |w| w.ctx().bounding_box());
        let commands = vec![MenuCommand::Book(BookCommand::Upload), MenuCommand::Book(BookCommand::Remove)];
        self.show_menu(root, menu, trigger, |layer| Popup::Menu { layer, commands });
    }

    pub(super) fn book_command(&mut self, root: &mut RenderRoot, command: BookCommand) {
        match command {
            BookCommand::Upload => self.pick_book(root),
            BookCommand::Remove => {
                self.reset_book_position();
                self.set_fields(root, &[(BOOK_FILE, String::new()), (PAGES, String::new()), (READER_POSITION, String::new()), (READ_PERCENT, String::new())]);
            }
        }
    }

    fn reset_book_position(&self) {
        let Some(workspace) = self.tree.root() else { return };
        let Some(key) = self.fields().and_then(|f| f.text(BOOK_FILE).map(str::to_owned)) else { return };
        let ui = &self.workspace.core.ui_state;
        let outcome = ui.resolve_workspace(&workspace.to_string_lossy(), crate::dates::now_ms()).and_then(|workspace_id| ui.delete_reader_state(workspace_id, &key));
        if let Err(error) = outcome { report!("позиция книги не сброшена: {error:?}"); }
    }

    pub(in crate::app) fn cover_action(&mut self, root: &mut RenderRoot, action: CoverAction) {
        match action {
            CoverAction::Add | CoverAction::Replace => self.pick_image(true),
            CoverAction::UploadBook => self.pick_image(false),
            CoverAction::Random => {
                let current = self.fields().and_then(|f| f.text(PAGE_COVER).map(str::to_owned));
                let next = cover::random_pattern(current.as_deref());
                self.set_fields(root, &[(COVER, "true".into()), (PAGE_COVER, next), (PAGE_COVER_POSITION, String::new())]);
            }
            CoverAction::Reposition => self.toggle_reposition(root),
            CoverAction::Remove => self.set_fields(root, &[(COVER, String::new()), (PAGE_COVER, String::new()), (PAGE_COVER_POSITION, String::new())]),
        }
    }

    fn open_cover_menu(&mut self, root: &mut RenderRoot, button: WidgetId) {
        let Some((_, ids)) = self.note_ids.as_ref().map(|n| &n.hero) else { return };
        let actions = ids.menu.clone();
        let labels: Vec<_> = actions.iter().map(|action| action.entry()).collect();
        let entries: Vec<Entry<'_>> = labels.iter().map(|(label, icon)| Entry { label, icon: Some(*icon), disabled: false, divided: false }).collect();
        let menu = Menu::entries(&t("book.coverActions"), &entries);
        let trigger = root.get_widget(button).map_or(Rect::ZERO, |w| w.ctx().bounding_box());
        let commands = actions.into_iter().map(MenuCommand::Cover).collect();
        self.show_menu(root, menu, trigger, |layer| Popup::Menu { layer, commands });
    }
}
