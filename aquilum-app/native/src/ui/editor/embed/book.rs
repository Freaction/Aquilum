use masonry::core::BrushIndex;
use masonry::kurbo::{Point, Rect};
use masonry::parley::Layout;

use super::super::markdown::{ACCENT, MUTED, ON_ACCENT, SUCCESS, TEXT};
use super::super::resolve::{Book, Fetch};
use super::media::FULL;
use super::metrics::{COVER_H, COVER_RADIUS, COVER_W};
use super::panel::PAD;
use super::{Drawing, Font, Item, Spec, Tone};
use crate::i18n::{t, t_with};
use crate::ui::tokens::size;

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
    let ellipsis = make(&spec(ELLIPSIS));
    let end = crate::ui::text::cut(&layout, width - f64::from(ellipsis.width())).unwrap_or(text.len());
    if end == 0 { return ellipsis; }
    let shown = format!("{}{ELLIPSIS}", &text[..end]);
    make(&spec(&shown))
}

fn row(items: &mut Vec<Item>, hits: &mut Vec<(Rect, super::Action, Option<f64>)>, entry: &Row, top: f64, available: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) {
    let pad = PAD;
    let book = match &entry.book {
        Fetch::Ready(book) => book,
        _ => return,
    };
    let cover = Rect::new(pad, top + pad, pad + COVER_W, top + pad + COVER_H).to_rounded_rect(COVER_RADIUS);
    items.push(Item::Fill(cover, Tone::CoverBg));
    items.push(Item::Image(cover, book.cover.clone().unwrap_or_else(crate::images::default_book_cover), FULL));
    let ui = size::FONT_SIZE_UI_BASE as u8;
    let progress_text = book.pages.map_or_else(|| t("book.progressUnknown"), |(a, b)| t_with("book.progress", &[("current", &a.to_string()), ("total", &b.to_string())]));
    let done = book.pages.is_some_and(|(a, b)| b > 0 && a >= b);
    let progress = make(&Spec::plain(&progress_text, Font::Ui(ui), if done { SUCCESS } else { MUTED }));
    let label = t("book.read");
    let button = book.linked.then(|| make(&Spec::plain(&label, Font::Ui(size::BUTTON_FONT_SIZE as u8), ON_ACCENT)));
    let inset = size::BUTTON_XS_PADDING_X + size::BUTTON_LABEL_PADDING_X;
    let button_w = button.as_ref().map_or(0.0, |b| f64::from(b.width()) + inset * 2.0);
    let right = available - pad;
    let side = button_w.max(f64::from(progress.width()));
    let text_x = pad + COVER_W + pad;
    let text_w = (right - side - pad - text_x).max(1.0);
    let title_brush = if book.linked { ACCENT } else { TEXT };
    let title = clipped(&entry.title, text_w, |s| Spec::plain(s, Font::Editor, title_brush.clone()), make);
    let author_text = if book.author.is_empty() { t("book.author") } else { book.author.clone() };
    let author = clipped(&author_text, text_w, |s| Spec::plain(s, Font::Editor, MUTED), make);
    let block = f64::from(title.height()) + f64::from(author.height());
    let ty = top + pad + (COVER_H - block) / 2.0;
    if book.linked {
        let shown = title.lines().next().map_or(0, |l| l.text_range().end);
        hits.extend(super::link_hits(&title, Point::new(text_x, ty), &[(0..shown, entry.target.clone())]));
    }
    let th = f64::from(title.height());
    items.push(Item::Text(Point::new(text_x, ty), title, entry.title.clone()));
    items.push(Item::Text(Point::new(text_x, ty + th), author, author_text));
    if let Some(button) = button {
        let rect = Rect::new(right - button_w, top + pad, right, top + pad + size::BUTTON_XS_HEIGHT);
        let label_y = rect.y0 + (size::BUTTON_XS_HEIGHT - f64::from(button.height())) / 2.0;
        items.push(Item::Button(rect.to_rounded_rect(size::BUTTON_XS_RADIUS), book.file.is_some()));
        if let Some(file) = &book.file {
            hits.push((rect, super::Action::Read { file: file.clone(), page: book.page.clone(), title: book.title.clone() }, None));
        }
        let text = Item::Text(Point::new(rect.x0 + inset, label_y), button, label);
        items.push(if book.file.is_some() { text } else { Item::Dim(Box::new(text)) });
    }
    let py = top + pad + COVER_H - f64::from(progress.height());
    items.push(Item::Text(Point::new(right - f64::from(progress.width()), py), progress, progress_text));
}

pub fn books(rows: &[Row], available: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) -> Drawing {
    let row_h = COVER_H + PAD * 2.0;
    let height = row_h * rows.len() as f64;
    let mut items = Vec::new();
    let mut hits = Vec::new();
    for (i, entry) in rows.iter().enumerate() {
        let top = row_h * i as f64;
        if i > 0 {
            items.push(Item::Fill(Rect::new(0.0, top - 0.5, available, top + 0.5).to_rounded_rect(0.0), Tone::Divider));
        }
        row(&mut items, &mut hits, entry, top, available, make);
    }
    Drawing { hits, ..Drawing::card(available, height, items) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use masonry::parley::{FontContext, LayoutContext};

    #[test]
    fn a_short_book_title_can_be_clipped_to_a_tiny_width_cover_audit() {
        let _lock = crate::ui::theme::test_lock();
        let (mut fcx, mut lcx) = (FontContext::new(), LayoutContext::new());
        let mut make = |spec: &Spec<'_>| {
            let style = crate::ui::text::Style::ui(20.0);
            let font = crate::ui::theme::ui_font_settings();
            let mut layout = crate::ui::text::builder(&mut fcx, &mut lcx, spec.text, crate::ui::theme::ui_font(&font.family), style).build(spec.text);
            layout.break_all_lines(None);
            layout
        };
        for text in ["Ж", "😀", "a界", "Название книги"] {
            let layout = clipped(text, 0.1, |s| Spec::plain(s, Font::Editor, TEXT), &mut make);
            assert!(layout.lines().next().is_some(), "missing ellipsis for {text}");
        }
    }
}
