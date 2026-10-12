mod hero;
mod import;

use std::path::Path;

use aquilum_core::history::Source;
use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};

use super::{App, Outcome};
use crate::cover;
use crate::frontmatter::{self, BOOK_COVER, COVER, PAGE_COVER, PAGE_COVER_POSITION};
use crate::ui::book::ThumbAction;
use crate::ui::widgets::Pressed;

pub(super) use import::{Picked, synthetic_pages};

impl App {
    pub(super) fn set_fields(&mut self, root: &mut RenderRoot, fields: &[(&str, String)]) {
        let Some(path) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        self.write_fields(root, &path, Source::Me, fields);
    }

    pub(super) fn write_fields(&mut self, root: &mut RenderRoot, path: &Path, source: Source, fields: &[(&str, String)]) {
        if !self.edit_note_text(root, path, source, |text| frontmatter::set_fields(text, fields)) {
            return;
        }
        if self.note.as_ref().is_some_and(|n| n.is(path)) {
            self.hero_shown = None;
            self.refresh_hero(root);
        }
    }

    pub(super) fn on_book_action(&mut self, root: &mut RenderRoot, id: WidgetId, action: &ErasedAction) -> Option<Outcome> {
        let (_, ids) = self.note_ids.as_ref()?.hero.as_ref()?;
        let pressed = action.downcast_ref::<Pressed>().is_some();
        let (replace, random, remove, replace_book) = (ids.replace, ids.random, ids.remove, ids.replace_book);
        let reposition = ids.reposition;
        let cover = ids.cover;
        let thumbnail = ids.thumbnail.map(|t| t.id());
        let book_button = ids.book_button.map(|b| b.id());
        if pressed && Some(id) == replace {
            self.pick_image(true);
        } else if pressed && Some(id) == random {
            let current = frontmatter::parse(&self.editor_text).and_then(|f| f.text(PAGE_COVER).map(str::to_owned));
            let next = cover::random_pattern(current.as_deref());
            self.set_fields(root, &[(COVER, "true".into()), (PAGE_COVER, next), (PAGE_COVER_POSITION, String::new())]);
        } else if pressed && Some(id) == remove {
            self.set_fields(root, &[(COVER, String::new()), (PAGE_COVER, String::new()), (PAGE_COVER_POSITION, String::new())]);
        } else if pressed && reposition.is_some_and(|r| r.id() == id) {
            self.toggle_reposition(root);
        } else if pressed && Some(id) == replace_book {
            self.pick_image(false);
        } else if pressed && Some(id) == book_button {
            self.book_pressed(root);
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
}
