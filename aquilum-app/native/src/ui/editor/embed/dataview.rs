use std::ops::Range;

use masonry::core::BrushIndex;
use masonry::kurbo::{Point, Rect};
use masonry::parley::Layout;

use super::super::markdown::{DANGER, SECONDARY, TEXT};
use super::super::resolve::{Fetch, Grid, Part, Shape};
use super::{Action, BAR_HEIGHT, BAR_MIN, BULLET_INDENT, CELL_X, CELL_Y, CHECK, CHECK_GAP, Drawing, Font, Item, NATURAL_MAX, PAD, PENDING, Spec, Tone, card, link_hits};

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
        Fetch::Ready(grid) => grid,
        Fetch::Pending => {
            items.extend(card(available, PENDING));
            return Drawing::new(PENDING, items);
        }
        Fetch::Missing(message) => {
            let layout = make(&Spec { wrap: Some(inner), ..Spec::plain(message, Font::Editor, DANGER) });
            let height = f64::from(layout.height()) + PAD * 2.0;
            items.extend(card(available, height));
            items.push(Item::Text(Point::new(PAD, PAD), layout));
            return Drawing::new(height, items);
        }
    };
    let mut y = PAD;
    let mut body = Vec::new();
    let mut hits = Vec::new();
    if let Some(title) = &grid.title {
        let layout = make(&Spec { bold: true, wrap: Some(inner), ..Spec::plain(title, Font::Editor, SECONDARY) });
        let h = f64::from(layout.height()) + CELL_Y * 2.0;
        body.push(Item::Text(Point::new(PAD, y + CELL_Y), layout));
        y += h;
        body.push(Item::Fill(Rect::new(PAD, y - 1.0, PAD + inner, y).to_rounded_rect(0.0), Tone::Divider));
    }
    match grid.shape {
        Shape::Table => {
            let m = grid.columns.len().max(grid.rows.iter().map(Vec::len).max().unwrap_or(0)).max(1);
            let mut natural = vec![0.0_f64; m];
            for (c, name) in grid.columns.iter().enumerate() {
                let layout = make(&Spec { bold: true, ..Spec::plain(name, Font::Editor, SECONDARY) });
                natural[c] = natural[c].max(f64::from(layout.width()));
            }
            for row in &grid.rows {
                for (c, parts) in row.iter().enumerate() {
                    let w = if parts.iter().any(|p| matches!(p, Part::Progress(_))) {
                        BAR_MIN
                    } else {
                        let (text, _) = cell_text(parts);
                        f64::from(make(&Spec::plain(&text, Font::Editor, TEXT)).width()).min(NATURAL_MAX)
                    };
                    natural[c] = natural[c].max(w);
                }
            }
            let natural: Vec<f64> = natural.into_iter().map(|w| w + CELL_X * 2.0).collect();
            let total: f64 = natural.iter().sum();
            let widths: Vec<f64> = natural.iter().map(|w| w * inner / total).collect();
            let mut xs = vec![PAD];
            for w in &widths {
                xs.push(xs.last().copied().unwrap_or(PAD) + w);
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
                    cells.push((c, Some((layout, links)), None, check));
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
                    if let Some((layout, links)) = layout {
                        let at = Point::new(tx, y + CELL_Y);
                        hits.extend(link_hits(&layout, at, &links));
                        body.push(Item::Text(at, layout));
                    }
                }
                y += height;
                if r + 1 < count {
                    body.push(Item::Fill(Rect::new(PAD, y - 1.0, PAD + inner, y).to_rounded_rect(0.0), Tone::Divider));
                }
            }
        }
        Shape::List | Shape::Tasks => {
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
                    None => body.push(Item::Bullet(Point::new(PAD + BULLET_INDENT, y + CELL_Y + f64::from(layout.lines().next().map_or(0.0, |l| (l.metrics().min_coord + l.metrics().max_coord) / 2.0))))),
                }
                let at = Point::new(PAD + indent, y + CELL_Y);
                hits.extend(link_hits(&layout, at, &links));
                body.push(Item::Text(at, layout));
                y += h;
            }
        }
    }
    let height = y + PAD;
    items.extend(card(available, height));
    items.extend(body);
    Drawing { height, items, hits }
}

