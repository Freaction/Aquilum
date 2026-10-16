use masonry::app::RenderRoot;

use super::super::App;
use crate::cover;
use crate::frontmatter::{self, BOOK_COVER, BOOK_FILE, PAGE_COVER, PAGE_COVER_POSITION};
use crate::i18n::t;
use crate::images;
use crate::ui::book::{BookView, HeroView, PageCover, Visual};
use crate::ui::widgets::{IconButton, Slot};
use crate::vault_files;

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
    pub(in crate::app) fn hero_view(&self) -> Option<HeroView> {
        let fields = frontmatter::parse(&self.head())?;
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
            has_file: fields.text(BOOK_FILE).is_some(),
        });
        Some(HeroView { cover, position: parse_position(fields.text(PAGE_COVER_POSITION)), book, viewport: self.window.height })
    }

    pub(in crate::app) fn hero_key(&self) -> Option<Vec<String>> {
        let fields = frontmatter::parse(&self.head())?;
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

    pub(in crate::app) fn refresh_hero(&mut self, root: &mut RenderRoot) {
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

    pub(in crate::app) fn refresh_covers(&mut self, root: &mut RenderRoot) {
        if !cover::take_finished() {
            return;
        }
        if let Some(cover) = self.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
            cover.edit(root, |mut cover| PageCover::refresh(&mut cover));
        }
    }

    pub(super) fn toggle_reposition(&mut self, root: &mut RenderRoot) {
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
        let stored = frontmatter::parse(&self.head()).and_then(|f| f.text(PAGE_COVER_POSITION).map(str::to_owned));
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
