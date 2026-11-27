use std::collections::HashMap;

use masonry::kurbo::{Point, Rect, RoundedRect};
use mermaid_rs_renderer::NodeShape;

use super::super::super::markdown::TEXT;
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, Label, Pen, Text, single};
use super::graph::edge;
use super::scene::Wire;
use super::layout::{NODE_GAP, RANK_GAP, measure};
use super::shape::Node;
use super::{arrow, unquote};
use crate::ui::tokens::size;

const PAD: f64 = 12.0;
const MIN: f64 = 96.0;

enum Kind<'a> {
    Leaf(&'a str, Text, NodeShape, (f64, f64)),
    Space,
    Group(&'a str, usize, Vec<Cell<'a>>),
}

struct Cell<'a> {
    kind: Kind<'a>,
    span: usize,
}

type Link<'a> = (&'a str, &'a str, Option<String>, bool);

fn tokens(line: &str) -> Vec<&str> {
    let (mut out, mut depth, mut quoted, mut start) = (Vec::new(), 0i32, false, None);
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '[' | '(' | '{' if !quoted => depth += 1,
            ']' | ')' | '}' if !quoted => depth -= 1,
            _ => {}
        }
        if c.is_whitespace() && depth <= 0 && !quoted {
            out.extend(start.take().map(|s| &line[s..i]));
        } else if start.is_none() {
            start = Some(i);
        }
    }
    out.extend(start.map(|s| &line[s..]));
    out
}

fn shape(open: &str) -> NodeShape {
    let starts = |p: &str| open.starts_with(p);
    match () {
        _ if starts("((") => NodeShape::Circle,
        _ if starts("([") => NodeShape::Stadium,
        _ if starts("[(") => NodeShape::Cylinder,
        _ if starts("{{") => NodeShape::Hexagon,
        _ if starts("{") => NodeShape::Diamond,
        _ if starts("(") => NodeShape::RoundRect,
        _ if starts("[/") => NodeShape::Parallelogram,
        _ if starts("[\\") => NodeShape::ParallelogramAlt,
        _ if starts(">") => NodeShape::Asymmetric,
        _ => NodeShape::Rectangle,
    }
}

fn leaf(token: &str) -> Cell<'_> {
    let spanned = token.rsplit_once(':').filter(|(_, s)| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()));
    let (body, span) = spanned.map_or((token, 1), |(b, s)| (b, s.parse().unwrap_or(1)));
    if body == "space" {
        return Cell { kind: Kind::Space, span };
    }
    let (id, rest) = body.find(['[', '(', '{', '>']).map_or((body, ""), |i| (&body[..i], &body[i..]));
    let label = if rest.is_empty() { id } else { rest.trim_matches(|c| "[](){}<>/\\\"".contains(c)) };
    let shape = shape(rest);
    let (text, dims) = measure(shape, unquote(label));
    Cell { kind: Kind::Leaf(id, text, shape, dims), span: span.max(1) }
}

fn link(line: &str) -> Option<Link<'_>> {
    let (at, len, head) = [("-->", true), ("==>", true), ("---", false)].iter().find_map(|(a, head)| line.find(a).map(|i| (i, a.len(), *head)))?;
    let (left, right) = (&line[..at], line[at + len..].trim());
    let (from, label) = left.split_once("--").map_or((left.trim(), None), |(f, l)| (f.trim(), Some(unquote(l).to_owned())));
    let (label, to) = right.strip_prefix('|').and_then(|r| r.split_once('|')).map_or((label, right), |(l, t)| (Some(unquote(l).to_owned()), t.trim()));
    Some((from, to, label.filter(|l| !l.is_empty()), head))
}

fn grid<'a>(lines: &[&'a str], i: &mut usize, links: &mut Vec<Link<'a>>) -> (usize, Vec<Cell<'a>>) {
    let (mut columns, mut cells) = (0, Vec::new());
    while *i < lines.len() {
        let line = lines[*i].trim();
        *i += 1;
        if line == "end" {
            break;
        }
        if let Some(n) = line.strip_prefix("columns ") {
            columns = n.trim().parse().unwrap_or(0);
        } else if let Some(l) = link(line) {
            links.push(l);
        } else {
            for token in tokens(line) {
                if token == "block" || token.starts_with("block:") {
                    let mut parts = token.split(':').skip(1);
                    let id = parts.next().unwrap_or_default();
                    let span = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                    let (inner, kids) = grid(lines, i, links);
                    cells.push(Cell { kind: Kind::Group(id, inner, kids), span });
                } else {
                    cells.push(leaf(token));
                }
            }
        }
    }
    (columns, cells)
}

fn fitted(shape: NodeShape, cell: Rect, dims: (f64, f64)) -> Rect {
    match shape {
        NodeShape::Circle | NodeShape::DoubleCircle | NodeShape::Diamond => Rect::from_center_size(cell.center(), dims),
        _ => cell,
    }
}

fn natural(cells: &[Cell]) -> f64 {
    cells.iter().map(|c| match &c.kind {
        Kind::Leaf(.., dims) if c.span == 1 => dims.0,
        Kind::Group(_, _, kids) => natural(kids),
        _ => 0.0,
    }).fold(MIN, f64::max)
}

fn lay<'a>(cells: &'a [Cell<'a>], columns: usize, origin: Point, width: f64, out: &mut Vec<(Rect, &'a Kind<'a>)>) -> f64 {
    let columns = if columns == 0 { cells.iter().map(|c| c.span).sum::<usize>().max(1) } else { columns };
    let col = (width - NODE_GAP * (columns - 1) as f64) / columns as f64;
    let (mut c, mut y, mut tall, mut row) = (0, origin.y, 0.0f64, Vec::new());
    let stretch = |row: &mut Vec<usize>, tall: f64, out: &mut Vec<(Rect, &'a Kind<'a>)>| row.drain(..).for_each(|k| out[k].0.y1 = out[k].0.y0 + tall);
    for cell in cells {
        let span = cell.span.min(columns);
        if c + span > columns {
            stretch(&mut row, tall, out);
            (c, y, tall) = (0, y + tall + RANK_GAP, 0.0);
        }
        let (x, w) = (origin.x + c as f64 * (col + NODE_GAP), span as f64 * col + (span - 1) as f64 * NODE_GAP);
        let high = match &cell.kind {
            Kind::Group(_, inner, kids) => {
                let at = out.len();
                out.push((Rect::ZERO, &cell.kind));
                let high = lay(kids, *inner, Point::new(x + PAD, y + PAD), w - PAD * 2.0, out) + PAD * 2.0;
                out[at].0 = Rect::new(x, y, x + w, y + high);
                high
            }
            Kind::Leaf(.., dims) => {
                row.push(out.len());
                out.push((Rect::new(x, y, x + w, y + dims.1), &cell.kind));
                dims.1
            }
            Kind::Space => 0.0,
        };
        (tall, c) = (tall.max(high), c + span);
    }
    stretch(&mut row, tall, out);
    y + tall - origin.y
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let (mut i, mut links) = (0, Vec::new());
    let (columns, cells) = grid(lines, &mut i, &mut links);
    let count = if columns == 0 { cells.iter().map(|c| c.span).sum::<usize>() } else { columns };
    if count == 0 {
        return;
    }
    let mut out = Vec::new();
    lay(&cells, columns, Point::ZERO, count as f64 * natural(&cells) + NODE_GAP * (count - 1) as f64, &mut out);
    let mut nodes: HashMap<&str, Node> = HashMap::new();
    for (r, kind) in &out {
        match kind {
            Kind::Leaf(id, _, shape, dims) => _ = nodes.insert(id, Node { rect: fitted(*shape, *r, *dims), shape: *shape }),
            Kind::Group(id, ..) => {
                canvas.shape(RoundedRect::from_rect(*r, size::RADIUS_XL), None, Some(Pen::border(Tone::Border)));
                if !id.is_empty() {
                    nodes.insert(id, Node { rect: *r, shape: NodeShape::Rectangle });
                }
            }
            Kind::Space => {}
        }
    }
    for (from, to, label, head) in &links {
        let (Some(a), Some(b)) = (nodes.get(from), nodes.get(to)) else { continue };
        let walls: Vec<_> = nodes.iter().filter(|(id, _)| *id != from && *id != to).map(|(_, n)| n.rect).collect();
        let wire = Wire::between(edge(from, to, label.clone(), (false, *head)), label.as_deref().map(single), *a, *b, arrow::sides(*a, *b, &walls));
        arrow::edges(&[wire], &nodes, true, |_| Tone::Edge, canvas);
    }
    for (r, kind) in &out {
        if let Kind::Leaf(_, text, shape, dims) = kind {
            let node = Node { rect: fitted(*shape, *r, *dims), shape: *shape };
            if let Some(outline) = node.outline() {
                canvas.shape(outline, Some(Tone::Editor), Some(Pen::border(Tone::Border)));
            }
            canvas.label(Label::block(r.center(), text, FONT, TEXT));
        }
    }
}
