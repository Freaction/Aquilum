use masonry::kurbo::{Point, Rect, RoundedRect, RoundedRectRadii};

use super::super::super::markdown::{MUTED, TEXT};
use super::super::panel::Tone as Hue;
use super::super::tone::Tone;
use super::canvas::{BOLD, Canvas, FONT, LINE, Label, Pen, Text, width, wrap};
use super::unquote;
use crate::ui::tokens::size;

const COLUMN: f64 = 216.0;
const GAP: f64 = 16.0;
const PAD: f64 = size::GAP_LG;
const STRIPE: f64 = 4.0;
const SMALL: f64 = FONT - 2.0;

struct Card {
    text: Text,
    meta: String,
    priority: Option<Hue>,
}

struct Column {
    title: String,
    cards: Vec<Card>,
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn text(item: &str) -> &str {
    let body = item.split("@{").next().unwrap_or(item).trim();
    match (body.find(['[', '(']), body.rfind([']', ')'])) {
        (Some(a), Some(b)) if b > a => unquote(&body[a + 1..b]),
        _ => unquote(body),
    }
}

fn meta(item: &str) -> Vec<(&str, &str)> {
    let Some(at) = item.find("@{") else { return Vec::new() };
    let inner = item[at + 2..].trim_end().trim_end_matches('}');
    inner.split(',').filter_map(|pair| pair.split_once(':')).map(|(k, v)| (k.trim(), v.trim().trim_matches(['\'', '"']))).collect()
}

fn priority(value: &str) -> Option<Hue> {
    match value.to_lowercase().as_str() {
        "very high" => Some(Hue::Danger),
        "high" => Some(Hue::Warning),
        "low" => Some(Hue::Info),
        "very low" => Some(Hue::Success),
        _ => None,
    }
}

fn parse(lines: &[&str]) -> Vec<Column> {
    let base = lines.iter().map(|l| indent(l)).min().unwrap_or(0);
    let mut columns: Vec<Column> = Vec::new();
    for line in lines {
        let item = line.trim();
        if indent(line) <= base {
            columns.push(Column { title: text(item).to_owned(), cards: Vec::new() });
            continue;
        }
        let Some(column) = columns.last_mut() else { continue };
        let data = meta(item);
        let get = |key: &str| data.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
        let info: Vec<&str> = [get("ticket"), get("assigned")].into_iter().flatten().collect();
        let text = wrap(text(item), COLUMN - size::GAP_MD * 2.0 - PAD * 2.0 - STRIPE);
        column.cards.push(Card { text, meta: info.join(" · "), priority: get("priority").and_then(priority) });
    }
    columns
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let columns = parse(lines);
    let tall = |c: &Card| c.text.height() + PAD * 2.0 + if c.meta.is_empty() { 0.0 } else { SMALL * 1.5 };
    let bottom = columns.iter().map(|c| c.cards.iter().map(|k| tall(k) + size::GAP_MD).sum::<f64>()).fold(0.0, f64::max);
    let head = LINE + PAD * 2.0;
    for (k, column) in columns.iter().enumerate() {
        let x0 = k as f64 * (COLUMN + GAP);
        let panel = Rect::new(x0, 0.0, x0 + COLUMN, head + bottom + size::GAP_MD);
        canvas.shape(RoundedRect::from_rect(panel, size::RADIUS_XL), Some(Tone::Panel(Hue::Surface)), None);
        let title = Point::new(x0 + PAD, PAD + LINE / 2.0);
        canvas.label(Label::new(title, column.title.as_str(), FONT, TEXT).bold().start(0.0));
        let count = Point::new(title.x + width(&column.title, FONT) * BOLD + size::GAP_SM, title.y);
        canvas.label(Label::new(count, column.cards.len().to_string(), SMALL, MUTED).start(0.0));
        let mut y = head;
        for card in &column.cards {
            let r = Rect::new(x0 + size::GAP_MD, y, x0 + COLUMN - size::GAP_MD, y + tall(card));
            canvas.shape(RoundedRect::from_rect(r, size::RADIUS_LG), Some(Tone::Editor), Some(Pen::border(Tone::Border)));
            if let Some(hue) = card.priority {
                let stripe = Rect::new(r.x0, r.y0, r.x0 + STRIPE, r.y1);
                canvas.shape(RoundedRect::from_rect(stripe, RoundedRectRadii::new(size::RADIUS_LG, 0.0, 0.0, size::RADIUS_LG)), Some(Tone::Ink(hue)), None);
            }
            let left = r.x0 + PAD + STRIPE;
            canvas.label(Label::block(Point::new(left, r.y0 + PAD + card.text.height() / 2.0), &card.text, FONT, TEXT).start(card.text.width));
            if !card.meta.is_empty() {
                canvas.label(Label::new(Point::new(left, r.y1 - PAD - SMALL * 0.5), card.meta.as_str(), SMALL, MUTED).start(0.0));
            }
            y = r.y1 + size::GAP_MD;
        }
    }
}
