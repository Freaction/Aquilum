use std::path::PathBuf;
use std::time::Duration;

use masonry::app::RenderRoot;
use masonry::core::{ErasedAction, WidgetId};

use super::{App, Outcome};
use crate::cover;
use crate::frontmatter::{self, BOOK_COVER, BOOK_FILE, PAGE_COVER, PAGE_COVER_POSITION};
use crate::i18n::t;
use crate::images;
use crate::ui::book::{BookView, HeroView, PageCover, ThumbAction, Visual};
use crate::ui::widgets::{IconButton, Pressed, Slot, TextButton};
use crate::vault_files::{self, IMAGE_EXTENSIONS, Source};

#[derive(Debug)]
pub(super) enum Picked {
    Workspace(PathBuf),
    PageCover(PathBuf, PathBuf),
    BookCover(PathBuf, PathBuf),
    Book(PathBuf, Option<PathBuf>),
}

const BOOK_EXTENSIONS: [&str; 4] = ["epub", "mobi", "azw3", "fb2"];
const BYTES_PER_PAGE: u64 = 1024;

pub(super) fn synthetic_pages(existing: Option<&str>, bytes: u64) -> String {
    let total = bytes.div_ceil(BYTES_PER_PAGE).max(1);
    let parsed = existing.and_then(|p| p.split_once('/')).and_then(|(a, b)| Some((a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?))).filter(|(_, b)| *b > 0.0);
    let current = parsed.map_or(0, |(a, b)| ((a / b).clamp(0.0, 1.0) * total as f64).round() as u64);
    format!("{current}/{total}")
}

fn parse_position(value: Option<&str>) -> f64 {
    let Some(value) = value.map(str::trim) else { return 50.0 };
    let percent = |s: &str| s.strip_suffix('%').and_then(|n| n.trim().parse::<f64>().ok());
    let parts: Vec<&str> = value.split_whitespace().collect();
    let y = match parts.as_slice() {
        [x, y] if percent(x).is_some() => percent(y),
        [single] => percent(single),
        _ => None,
    };
    y.map_or(50.0, |y| y.clamp(0.0, 100.0))
}

fn format_position(y: f64) -> String {
    format!("{}%", y.round().clamp(0.0, 100.0) as i64)
}

impl App {
    pub(super) fn hero_view(&self) -> Option<HeroView> {
        let fields = frontmatter::parse(&self.editor_text)?;
        let (has_cover, is_book) = (fields.has_page_cover(), fields.is_book());
        if !has_cover && !is_book {
            return None;
        }
        let root = self.tree.root();
        let cover = has_cover.then(|| {
            let value = fields.text(PAGE_COVER);
            match cover::pattern_id(value) {
                Some(id) => Visual::Pattern(id),
                None => value
                    .and_then(|v| vault_files::resolve(root, v))
                    .and_then(|path| images::load(&path))
                    .map_or(Visual::Pattern(cover::DEFAULT_PATTERN), Visual::Image),
            }
        });
        let book = is_book.then(|| BookView {
            cover: fields
                .text(BOOK_COVER)
                .and_then(|v| vault_files::resolve(root, v))
                .and_then(|path| images::load(&path))
                .unwrap_or_else(images::default_book_cover),
            has_file: fields.text(BOOK_FILE).is_some_and(|f| !f.trim().is_empty()),
        });
        Some(HeroView { cover, position: parse_position(fields.text(PAGE_COVER_POSITION)), book, viewport: self.window.height })
    }

    pub(super) fn hero_key(&self) -> Option<Vec<String>> {
        let fields = frontmatter::parse(&self.editor_text)?;
        if !fields.has_page_cover() && !fields.is_book() {
            return None;
        }
        Some(
            [PAGE_COVER, BOOK_COVER, PAGE_COVER_POSITION, BOOK_FILE]
                .iter()
                .map(|k| fields.text(k).unwrap_or_default().to_owned())
                .chain([fields.has_page_cover().to_string(), fields.is_book().to_string()])
                .collect(),
        )
    }

    pub(super) fn refresh_hero(&mut self, root: &mut RenderRoot) {
        let key = self.hero_key();
        if key == self.hero_shown {
            return;
        }
        self.hero_shown = key;
        let parts = crate::ui::note::hero_parts(self.hero_view());
        let Some(ids) = &mut self.note_ids else { return };
        ids.hero = parts.hero;
        let (slot, spacer, column) = (ids.hero_slot, ids.spacer, ids.column);
        slot.edit(root, |mut slot| Slot::set_child(&mut slot, parts.widget));
        spacer.edit(root, |mut spacer| masonry::widgets::SizedBox::set_height(&mut spacer, masonry::layout::Length::px(parts.top)));
        column.edit(root, |mut column| crate::ui::note::EditorColumn::set_inset(&mut column, parts.inset));
    }

    pub(super) fn set_fields(&mut self, root: &mut RenderRoot, fields: &[(&str, String)]) {
        let mut text = self.editor_text.clone();
        for (key, value) in fields {
            match frontmatter::set_field(&text, key, value) {
                Some(next) => text = next,
                None => return,
            }
        }
        self.editor_text = text;
        if let Some(note) = &mut self.note {
            note.edit(self.editor_text.clone(), Duration::ZERO);
            note.save(&self.workspace.core);
        }
        self.hero_shown = None;
        self.sync_body(root);
        self.refresh_hero(root);
    }

    fn book_pressed(&mut self, root: &mut RenderRoot) {
        let file = frontmatter::parse(&self.editor_text).and_then(|f| f.text(BOOK_FILE).map(str::to_owned)).filter(|f| !f.trim().is_empty());
        if let Some(file) = file {
            if let Some(path) = vault_files::resolve(self.tree.root(), &file)
                && let Some(note) = self.note.as_ref().map(|n| n.path.clone())
            {
                let title = note.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                let position = frontmatter::parse(&self.editor_text).and_then(|f| f.text("reader_position").map(str::to_owned));
                self.open_reader(root, super::reader::Opening { file: path, page: Some(note), title, initial: position, peek: false });
            }
            return;
        }
        let Some(note) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        self.set_book_busy(root, true);
        let sender = self.picked_tx.clone();
        let wake = std::sync::Arc::clone(&self.workspace.wake);
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new().add_filter("Books", &BOOK_EXTENSIONS).pick_file();
            if sender.send(Picked::Book(note, picked)).is_ok() {
                wake();
            }
        });
    }

    fn set_book_busy(&self, root: &mut RenderRoot, busy: bool) {
        if let Some(button) = self.note_ids.as_ref().and_then(|n| n.hero.as_ref()).and_then(|(_, ids)| ids.book_button) {
            button.edit(root, |mut b| TextButton::set_disabled(&mut b, busy));
        }
    }

    fn import_book(&mut self, root: &mut RenderRoot, file: &std::path::Path) {
        let Some(workspace) = self.tree.root().map(std::path::Path::to_path_buf) else { return };
        match vault_files::import(&self.workspace.core, &workspace, Source::Path(file), "book", "epub") {
            Ok(relative) => {
                let bytes = std::fs::metadata(file).map_or(0, |m| m.len());
                let existing = frontmatter::parse(&self.editor_text).and_then(|f| f.text("pages").map(str::to_owned));
                self.set_fields(root, &[(BOOK_FILE, relative), ("pages", synthetic_pages(existing.as_deref(), bytes))]);
            }
            Err(error) => eprintln!("книга не загружена: {error:?}"),
        }
    }

    fn pick_image(&self, page_cover: bool) {
        let Some(note) = self.note.as_ref().map(|n| n.path.clone()) else { return };
        let sender = self.picked_tx.clone();
        let wake = std::sync::Arc::clone(&self.workspace.wake);
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new().add_filter("Images", IMAGE_EXTENSIONS).pick_file();
            if let Some(file) = picked {
                let message = if page_cover { Picked::PageCover(note, file) } else { Picked::BookCover(note, file) };
                if sender.send(message).is_ok() {
                    wake();
                }
            }
        });
    }

    pub(super) fn on_picked(&mut self, root: &mut RenderRoot, picked: Picked) -> bool {
        let current = |note: &PathBuf| self.note.as_ref().is_some_and(|n| n.is(note));
        match picked {
            Picked::Workspace(path) => return self.switch_workspace(root, path).title,
            Picked::PageCover(note, file) if current(&note) => {
                if let Some(relative) = self.import_cover(Source::Path(&file)) {
                    self.set_fields(root, &[("cover", "true".into()), (PAGE_COVER, relative), (PAGE_COVER_POSITION, String::new())]);
                }
            }
            Picked::BookCover(note, file) if current(&note) => {
                if let Some(relative) = self.import_cover(Source::Path(&file)) {
                    self.set_fields(root, &[(BOOK_COVER, relative)]);
                }
            }
            Picked::Book(note, file) => {
                self.set_book_busy(root, false);
                if let Some(file) = file.filter(|_| current(&note)) {
                    self.import_book(root, &file);
                }
            }
            Picked::PageCover(..) | Picked::BookCover(..) => {}
        }
        false
    }

    fn import_cover(&self, source: Source<'_>) -> Option<String> {
        let workspace = self.tree.root()?.to_path_buf();
        let fallback = match source {
            Source::Path(_) => "jpg",
            Source::Bytes(..) => "png",
        };
        vault_files::import(&self.workspace.core, &workspace, source, "cover", fallback)
            .map_err(|error| eprintln!("обложка не сохранена: {error:?}"))
            .ok()
    }

    pub(super) fn refresh_covers(&mut self, root: &mut RenderRoot) {
        if !cover::take_finished() {
            return;
        }
        if let Some(cover) = self.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
            cover.edit(root, |mut cover| PageCover::refresh(&mut cover));
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
            self.set_fields(root, &[("cover", "true".into()), (PAGE_COVER, next), (PAGE_COVER_POSITION, String::new())]);
        } else if pressed && Some(id) == remove {
            self.set_fields(root, &[("cover", String::new()), (PAGE_COVER, String::new()), (PAGE_COVER_POSITION, String::new())]);
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

    fn toggle_reposition(&mut self, root: &mut RenderRoot) {
        let Some((_, ids)) = self.note_ids.as_ref().and_then(|n| n.hero.as_ref()) else { return };
        let (Some(cover), Some(button)) = (ids.cover, ids.reposition) else { return };
        let Some((on, position)) = cover.get(root).map(|c| (c.inner().repositioning(), c.inner().position())) else { return };
        if !on {
            cover.edit(root, |mut cover| PageCover::set_repositioning(&mut cover, true, None));
            button.edit(root, |mut b| {
                IconButton::set_pressed(&mut b, true);
                IconButton::set_label(&mut b, &t("book.done"));
            });
            return;
        }
        let stored = frontmatter::parse(&self.editor_text).and_then(|f| f.text(PAGE_COVER_POSITION).map(str::to_owned));
        let formatted = format_position(position);
        if format_position(parse_position(stored.as_deref())) == formatted {
            cover.edit(root, |mut cover| PageCover::set_repositioning(&mut cover, false, None));
            button.edit(root, |mut b| {
                IconButton::set_pressed(&mut b, false);
                IconButton::set_label(&mut b, &t("book.coverPosition"));
            });
            return;
        }
        let value = if formatted == "50%" { String::new() } else { formatted };
        self.set_fields(root, &[(PAGE_COVER_POSITION, value)]);
    }

    fn paste_book_cover(&mut self, root: &mut RenderRoot) {
        let Ok(mut clipboard) = arboard::Clipboard::new() else { return };
        let Ok(image) = clipboard.get_image() else { return };
        let Some(buffer) = image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.bytes.into_owned()) else { return };
        let mut png = std::io::Cursor::new(Vec::new());
        if buffer.write_to(&mut png, image::ImageFormat::Png).is_err() {
            return;
        }
        let bytes = png.into_inner();
        if let Some(relative) = self.import_cover(Source::Bytes(&bytes, "image.png")) {
            self.set_fields(root, &[(BOOK_COVER, relative)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_position_parsing() {
        assert_eq!(parse_position(None), 50.0);
        assert_eq!(parse_position(Some("30%")), 30.0);
        assert_eq!(parse_position(Some("10% 140%")), 100.0);
        assert_eq!(parse_position(Some("abc")), 50.0);
        assert_eq!(format_position(49.6), "50%");
    }
}
