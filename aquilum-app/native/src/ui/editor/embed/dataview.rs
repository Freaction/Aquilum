use std::ops::Range;

use masonry::core::BrushIndex;
use masonry::kurbo::{Point, Rect};
use masonry::parley::Layout;

use super::super::markdown::{DANGER, MUTED, SECONDARY, TEXT};
use super::super::resolve::{Fetch, Grid, Part, Shape};
use super::super::table::fit_columns;
use super::metrics::{BAR_HEIGHT, BAR_MIN, BULLET_INDENT, CELL_X, CELL_Y, CHECK, CHECK_GAP, PENDING};
use super::panel::PAD;
use super::{Action, Drawing, Font, Item, Spec, Tone, card, link_hits};
use crate::i18n::t;

fn cell_text(parts: &[Part]) -> (String, Vec<(Range<usize>, String)>) {
    let mut text = String::new();
    let mut links = Vec::new();
    for part in parts {
        match part {
            Part::Text(s) => text.push_str(s),
            Part::Link { text: s, target } => {
                links.push((text.len()..text.len() + s.len(), target.clone()));
                text.push_str(s);
            }
            _ => {}
        }
    }
    (text, links)
}

fn task(parts: &[Part]) -> Option<(bool, Action)> {
    parts.iter().find_map(|p| match p {
        Part::Check { done, target, line } => Some((*done, Action::Task { target: target.clone(), line: *line })),
        _ => None,
    })
}

pub fn dataview(grid: &Fetch<Grid>, available: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) -> Drawing {
    let inner = (available - PAD * 2.0).max(1.0);
    let mut items = Vec::new();
    let grid = match grid {
        Fetch::Ready(grid) if !grid.rows.is_empty() => grid,
        Fetch::Pending => {
            items.extend(card(available, PENDING));
            return Drawing::new(PENDING, items);
        }
        other => {
            let (message, brush) = if let Fetch::Missing(message) = other { (message.clone(), DANGER) } else { (t("search.noResults"), MUTED) };
            let layout = make(&Spec { wrap: Some(inner), ..Spec::plain(&message, Font::Editor, brush) });
            let height = f64::from(layout.height()) + PAD * 2.0;
            items.extend(card(available, height));
            items.push(Item::Text(Point::new(PAD, PAD), layout, message));
            return Drawing::new(height, items);
        }
    };
    let side = if grid.shape == Shape::Table { PAD - CELL_X } else { PAD };
    let widths = (grid.shape == Shape::Table).then(|| column_widths(grid, available - side * 2.0, make));
    let mut span = widths.as_ref().map_or(inner, |widths| widths.iter().sum());
    let mut y = PAD - CELL_Y;
    let mut body = Vec::new();
    let mut hits = Vec::new();
    if let Some(title) = &grid.title {
        let layout = make(&Spec { bold: true, wrap: Some(inner), ..Spec::plain(title, Font::Editor, SECONDARY) });
        span = span.max(f64::from(layout.width()));
        let h = f64::from(layout.height()) + CELL_Y * 2.0;
        body.push(Item::Text(Point::new(PAD, y + CELL_Y), layout, title.clone()));
        y += h;
        body.push(Item::Fill(Rect::new(side, y - 1.0, side + span, y).to_rounded_rect(0.0), Tone::Divider));
    }
    match widths {
        Some(widths) => {
            let m = widths.len();
            let mut xs = vec![side];
            for w in &widths {
                xs.push(xs.last().copied().unwrap_or(side) + w);
            }
            let mut rows: Vec<Vec<Vec<Part>>> = Vec::new();
            if !grid.columns.is_empty() {
                rows.push(grid.columns.iter().map(|c| vec![Part::Text(c.clone())]).collect());
            }
            rows.extend(grid.rows.iter().cloned());
            let count = rows.len();
            for (r, row) in rows.iter().enumerate() {
                let header = r == 0 && !grid.columns.is_empty();
                let mut height: f64 = 0.0;
                let mut cells = Vec::new();
                for (c, parts) in row.iter().enumerate().take(m) {
                    let w = widths[c] - CELL_X * 2.0;
                    if let Some(Part::Progress(p)) = parts.iter().find(|p| matches!(p, Part::Progress(_))) {
                        height = height.max(BAR_HEIGHT + CELL_Y * 4.0);
                        cells.push((c, None, Some(*p), None));
                        continue;
                    }
                    let check = task(parts);
                    let (text, links) = cell_text(parts);
                    let ranges: Vec<Range<usize>> = links.iter().map(|(r, _)| r.clone()).collect();
                    let wrap = if check.is_some() { w - CHECK - CHECK_GAP } else { w };
                    let layout = make(&Spec { bold: header, links: &ranges, wrap: Some(wrap.max(1.0)), ..Spec::plain(&text, Font::Editor, if header { SECONDARY } else { TEXT }) });
                    height = height.max(f64::from(layout.height()).max(if check.is_some() { CHECK } else { 0.0 }) + CELL_Y * 2.0);
                    cells.push((c, Some((layout, links, text)), None, check));
                }
                for (c, layout, progress, check) in cells {
                    let x = xs[c] + CELL_X;
                    if let Some(p) = progress {
                        let w = widths[c] - CELL_X * 2.0;
                        let bar = Rect::new(x, y + (height - BAR_HEIGHT) / 2.0, x + w, y + (height + BAR_HEIGHT) / 2.0);
                        body.push(Item::Fill(bar.to_rounded_rect(BAR_HEIGHT / 2.0), Tone::AccentSubtle));
                        let fill = Rect::new(bar.x0, bar.y0, bar.x0 + bar.width() * (p / 100.0).clamp(0.0, 1.0), bar.y1);
                        body.push(Item::Fill(fill.to_rounded_rect(BAR_HEIGHT / 2.0), Tone::Accent));
                    }
                    let mut tx = x;
                    if let Some((done, action)) = check {
                        let rect = Rect::new(x, y + CELL_Y, x + CHECK, y + CELL_Y + CHECK);
                        body.push(Item::Check(rect, done));
                        hits.push((rect, action, None));
                        tx += CHECK + CHECK_GAP;
                    }
                    if let Some((layout, links, text)) = layout {
                        let at = Point::new(tx, y + CELL_Y);
                        hits.extend(link_hits(&layout, at, &links));
                        body.push(Item::Text(at, layout, text));
                    }
                }
                y += height;
                if r + 1 < count {
                    body.push(Item::Fill(Rect::new(side, y - 1.0, side + span, y).to_rounded_rect(0.0), Tone::Divider));
                }
            }
        }
        None => {
            for row in &grid.rows {
                let parts: Vec<Part> = row.iter().flatten().cloned().collect();
                let check = task(&parts);
                let (text, links) = cell_text(&parts);
                let ranges: Vec<Range<usize>> = links.iter().map(|(r, _)| r.clone()).collect();
                let indent = if grid.shape == Shape::Tasks { CHECK + CHECK_GAP } else { BULLET_INDENT * 2.0 };
                let layout = make(&Spec { links: &ranges, wrap: Some((inner - indent).max(1.0)), ..Spec::plain(&text, Font::Editor, TEXT) });
                let h = f64::from(layout.height()).max(if check.is_some() { CHECK } else { 0.0 }) + CELL_Y * 2.0;
                match check {
                    Some((done, action)) => {
                        let rect = Rect::new(PAD, y + (h - CHECK) / 2.0, PAD + CHECK, y + (h + CHECK) / 2.0);
                        body.push(Item::Check(rect, done));
                        hits.push((rect, action, None));
                    }
                    None => body.push(Item::Bullet(Point::new(PAD + BULLET_INDENT, y + CELL_Y + f64::from(layout.lines().next().map_or(0.0, |l| (l.metrics().block_min_coord + l.metrics().block_max_coord) / 2.0))))),
                }
                let at = Point::new(PAD + indent, y + CELL_Y);
                hits.extend(link_hits(&layout, at, &links));
                body.push(Item::Text(at, layout, text));
                y += h;
            }
        }
    }
    let (height, width) = (y + PAD - CELL_Y, span + side * 2.0);
    items.extend(card(width, height));
    items.extend(body);
    Drawing { hits, view: available, width, ..Drawing::new(height, items) }
}

fn column_widths(grid: &Grid, inner: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) -> Vec<f64> {
    let m = grid.columns.len().max(grid.rows.iter().map(Vec::len).max().unwrap_or(0)).max(1);
    let mut natural = vec![0.0_f64; m];
    for (c, name) in grid.columns.iter().enumerate() {
        natural[c] = natural[c].max(f64::from(make(&Spec { bold: true, ..Spec::plain(name, Font::Editor, SECONDARY) }).width()));
    }
    for row in &grid.rows {
        for (c, parts) in row.iter().enumerate() {
            let w = if parts.iter().any(|p| matches!(p, Part::Progress(_))) { BAR_MIN } else { f64::from(make(&Spec::plain(&cell_text(parts).0, Font::Editor, TEXT)).width()) };
            natural[c] = natural[c].max(w);
        }
    }
    let natural: Vec<f64> = natural.into_iter().map(|w| w + CELL_X * 2.0).collect();
    fit_columns(&natural, inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dataview_table_card_spans_the_whole_column() {
        let cell = |text: &str| vec![Part::Text(text.into())];
        let grid = Grid { shape: Shape::Table, title: None, columns: vec!["a".into(), "b".into()], rows: vec![vec![cell("1"), cell("2")]] };
        let drawing = dataview(&Fetch::Ready(grid), 800.0, &mut |_| Layout::default());
        let card = drawing.items.iter().find_map(|item| if let Item::Fill(shape, _) = item { Some(shape.rect().width()) } else { None });
        assert_eq!(card, Some(800.0));
        let empty = dataview(&Fetch::Ready(Grid { shape: Shape::List, title: None, columns: Vec::new(), rows: Vec::new() }), 800.0, &mut |_| Layout::default());
        assert!(empty.items.iter().any(|item| matches!(item, Item::Text(_, _, text) if *text == t("search.noResults"))), "пустой результат подписан");
        assert_eq!(drawing.max_scroll(), 0.0, "узкая таблица не прокручивается");
    }

    #[test]
    fn a_wide_dataview_table_scrolls_sideways() {
        let columns: Vec<String> = (0..10).map(|i| format!("колонка {i}")).collect();
        let row = columns.iter().map(|c| vec![Part::Text(c.clone())]).collect();
        let grid = Grid { shape: Shape::Table, title: None, columns, rows: vec![row] };
        let drawing = dataview(&Fetch::Ready(grid), 400.0, &mut |_| Layout::default());
        assert!(drawing.max_scroll() > 0.0, "десять колонок в 400 px должны прокручиваться вбок");
    }
}
