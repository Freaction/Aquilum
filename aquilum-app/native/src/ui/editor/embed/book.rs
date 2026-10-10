use masonry::core::BrushIndex;
use masonry::kurbo::{Point, Rect};
use masonry::parley::Layout;

use super::super::markdown::{ACCENT, MUTED, ON_ACCENT, SUCCESS, TEXT};
use super::super::resolve::{Book, Fetch};
use super::{COVER_H, COVER_RADIUS, COVER_W, Drawing, Font, Item, Spec, Tone};
use crate::i18n::t;
use crate::ui::tokens::{number, size};

const ELLIPSIS: &str = "…";

pub struct Row {
    pub book: Fetch<Book>,
    pub target: String,
    pub title: String,
}

fn clipped(text: &str, width: f64, spec: impl Fn(&str) -> Spec<'_>, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) -> Layout<BrushIndex> {
    let layout = make(&spec(text));
    if f64::from(layout.width()) <= width {
        return layout;
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate = format!("{}{ELLIPSIS}", &text[..chars[mid].0]);
        if f64::from(make(&spec(&candidate)).width()) <= width { lo = mid } else { hi = mid - 1 }
    }
    let shown = format!("{}{ELLIPSIS}", &text[..chars.get(lo).map_or(text.len(), |c| c.0)]);
    make(&spec(&shown))
}

fn row(items: &mut Vec<Item>, hits: &mut Vec<(Rect, super::Action, Option<f64>)>, entry: &Row, top: f64, available: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) {
    let pad = size::GAP_XL;
    let book = match &entry.book {
        Fetch::Ready(book) => book.clone(),
        _ => return,
    };
    let cover = Rect::new(pad, top + pad, pad + COVER_W, top + pad + COVER_H).to_rounded_rect(COVER_RADIUS);
    items.push(Item::Fill(cover, Tone::CoverBg));
    items.push(Item::Image(cover, book.cover.unwrap_or_else(crate::images::default_book_cover), [0.0, 0.0, 100.0, 100.0]));
    let ui = size::FONT_SIZE_UI_BASE as u8;
    let progress_text = book.pages.map_or_else(|| "Прогресс: —".to_owned(), |(a, b)| format!("Прогресс: {a}/{b}"));
    let done = book.pages.is_some_and(|(a, b)| b > 0 && a >= b);
    let progress = make(&Spec::plain(&progress_text, Font::Ui(ui), if done { SUCCESS } else { MUTED }));
    let label = t("book.read");
    let button = book.linked.then(|| make(&Spec { weight: Some(number::FONT_WEIGHT_UI_MEDIUM as f32), ..Spec::plain(&label, Font::Ui(size::BUTTON_FONT_SIZE as u8), ON_ACCENT) }));
    let button_w = button.as_ref().map_or(0.0, |b| f64::from(b.width()) + size::BUTTON_XS_PADDING_X * 2.0);
    let right = available - pad;
    let side = button_w.max(f64::from(progress.width()));
    let text_x = pad + COVER_W + pad;
    let text_w = (right - side - pad - text_x).max(1.0);
    let title_brush = if book.linked { ACCENT } else { TEXT };
    let title = clipped(&entry.title, text_w, |s| Spec::plain(s, Font::Editor, title_brush.clone()), make);
    let author_text = if book.author.is_empty() { "Автор".to_owned() } else { book.author.clone() };
    let author = clipped(&author_text, text_w, |s| Spec::plain(s, Font::Editor, MUTED), make);
    let block = f64::from(title.height()) + f64::from(author.height());
    let ty = top + pad + (COVER_H - block) / 2.0;
    if book.linked {
        let shown = title.lines().next().map_or(0, |l| l.text_range().end);
        hits.extend(super::link_hits(&title, Point::new(text_x, ty), &[(0..shown, entry.target.clone())]));
    }
    let th = f64::from(title.height());
    items.push(Item::Text(Point::new(text_x, ty), title));
    items.push(Item::Text(Point::new(text_x, ty + th), author));
    if let Some(button) = button {
        let rect = Rect::new(right - button_w, top + pad, right, top + pad + size::BUTTON_XS_HEIGHT);
        let label_y = rect.y0 + (size::BUTTON_XS_HEIGHT - f64::from(button.height())) / 2.0;
        items.push(Item::Button(rect.to_rounded_rect(size::BUTTON_XS_RADIUS), book.file.is_some()));
        if let Some(file) = &book.file {
            hits.push((rect, super::Action::Read { file: file.clone(), page: book.page.clone(), title: book.title.clone() }, None));
        }
        let text = Item::Text(Point::new(rect.x0 + size::BUTTON_XS_PADDING_X, label_y), button);
        items.push(if book.file.is_some() { text } else { Item::Dim(Box::new(text)) });
    }
    let py = top + pad + COVER_H - f64::from(progress.height());
    items.push(Item::Text(Point::new(right - f64::from(progress.width()), py), progress));
}

pub fn books(rows: &[Row], available: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) -> Drawing {
    let row_h = COVER_H + size::GAP_XL * 2.0;
    let height = row_h * rows.len() as f64;
    let mut items: Vec<Item> = super::card(available, height).into();
    let mut hits = Vec::new();
    for (i, entry) in rows.iter().enumerate() {
        let top = row_h * i as f64;
        if i > 0 {
            items.push(Item::Fill(Rect::new(0.0, top - 0.5, available, top + 0.5).to_rounded_rect(0.0), Tone::Divider));
        }
        row(&mut items, &mut hits, entry, top, available, make);
    }
    Drawing { height, items, hits }
}
