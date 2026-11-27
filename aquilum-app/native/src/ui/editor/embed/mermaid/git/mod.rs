mod parse;

use masonry::kurbo::{BezPath, Circle, Line, Point, Rect, RoundedRect, Vec2};

use super::super::super::markdown::{MUTED, TEXT};
use super::super::tone::Tone;
use super::arrow::bow;
use super::canvas::{BOLD, Canvas, FONT, LINE, LINK, Label, Pen, chip_rect, width, wrap};
use super::hue;
use crate::ui::tokens::size;
use parse::{Mark, parse};

const LANE: f64 = 72.0;
const STEP: f64 = 48.0;
const DOT: f64 = 7.0;
const SMALL: f64 = FONT - 2.0;
const WRAP: f64 = 400.0;

#[derive(Clone, Copy, PartialEq)]
enum Flow {
    Right,
    Down,
    Up,
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    chart(lines, Flow::Right, canvas);
}

pub(super) fn down(lines: &[&str], canvas: &mut Canvas) {
    chart(lines, Flow::Down, canvas);
}

pub(super) fn up(lines: &[&str], canvas: &mut Canvas) {
    chart(lines, Flow::Up, canvas);
}

fn chart(lines: &[&str], flow: Flow, canvas: &mut Canvas) {
    let git = parse(lines);
    let Some(last) = git.commits.len().checked_sub(1) else { return };
    let right = flow == Flow::Right;
    let mut order: Vec<usize> = (0..git.lanes.len()).collect();
    order.sort_by(|&a, &b| git.lanes[a].1.total_cmp(&git.lanes[b].1).then(a.cmp(&b)));
    let mut rank = vec![0; git.lanes.len()];
    order.iter().enumerate().for_each(|(r, &l)| rank[l] = r);
    let tint = |lane: usize| hue(rank[lane]);
    let tags: Vec<_> = git.commits.iter().map(|c| c.tag.as_deref().map(|t| wrap(t, WRAP))).collect();
    let room: Vec<f64> = git.commits.iter().zip(&tags).map(|(c, t)| c.id.as_deref().map_or(0.0, |i| width(i, SMALL)).max(t.as_ref().map_or(0.0, |t| chip_rect(Point::ZERO, t).width()))).collect();
    let names = git.lanes.iter().map(|(n, _)| width(n, FONT) * BOLD).fold(0.0, f64::max) + size::GAP_XL * 2.0;
    let head = LINE + size::GAP_XS * 2.0;
    let spread = if right { LANE } else { (names + size::GAP_XL).max(room.iter().copied().fold(0.0, f64::max) * 2.0 + DOT * 2.0 + size::GAP_LG * 2.0).max(LANE) };
    let mut along = vec![if right { names + STEP } else { head + STEP }];
    for k in 1..room.len() {
        let gap = if right { STEP.max((room[k - 1] + room[k]) / 2.0 + size::GAP_LG) } else { STEP };
        along.push(along[k - 1] + gap);
    }
    let put = |a: f64, lane: usize| {
        let l = rank[lane] as f64 * spread;
        match flow {
            Flow::Right => Point::new(a, l),
            Flow::Down => Point::new(l, a),
            Flow::Up => Point::new(l, -a),
        }
    };
    let ahead = (put(1.0, 0) - put(0.0, 0)).normalize();
    let at = |k: usize| put(along[k], git.commits[k].lane);
    for (l, (name, _)) in git.lanes.iter().enumerate() {
        let start = if right { names } else { head };
        canvas.shape(Line::new(put(start, l), put(along[last] + STEP / 2.0, l)), None, Some(Pen::border(Tone::Guide).dashed()));
        let chip = Rect::from_center_size(put(start / 2.0, l), (names, head));
        canvas.shape(RoundedRect::from_rect(chip, size::RADIUS_LG), Some(Tone::Panel(tint(l))), None);
        canvas.label(Label::new(chip.center(), name.clone(), FONT, TEXT).bold());
    }
    for (k, c) in git.commits.iter().enumerate() {
        for (n, &p) in c.parents.iter().enumerate() {
            let (a, b) = (at(p), at(k));
            let pen = Pen::new(Tone::Ink(tint(if n == 0 { c.lane } else { git.commits[p].lane })), LINK);
            let pen = if c.mark == Mark::Pick && n > 0 { pen.dashed() } else { pen };
            if git.commits[p].lane == c.lane {
                canvas.shape(Line::new(a, b), None, Some(pen));
                continue;
            }
            let curve = bow(a, ahead, b, -ahead);
            let mut path = BezPath::new();
            path.move_to(a);
            path.curve_to(curve.p1, curve.p2, b);
            canvas.shape(path, None, Some(pen));
        }
    }
    for (k, (c, tag)) in git.commits.iter().zip(&tags).enumerate() {
        let (p, ink) = (at(k), Tone::Ink(tint(c.lane)));
        let ring = Pen::new(ink, LINK);
        match c.mark {
            Mark::Normal => canvas.shape(Circle::new(p, DOT), Some(ink), None),
            Mark::Highlight => canvas.shape(RoundedRect::from_rect(Rect::from_center_size(p, (DOT * 2.0, DOT * 2.0)), size::RADIUS_SM), Some(ink), None),
            Mark::Merge | Mark::Pick => {
                canvas.shape(Circle::new(p, DOT), Some(Tone::Editor), Some(ring));
                canvas.shape(Circle::new(p, DOT / 2.5), Some(ink), None);
            }
            Mark::Reverse => {
                canvas.shape(Circle::new(p, DOT), Some(Tone::Editor), Some(ring));
                let d = DOT / 2.0;
                canvas.shape(Line::new(p + Vec2::new(-d, -d), p + Vec2::new(d, d)), None, Some(ring));
                canvas.shape(Line::new(p + Vec2::new(-d, d), p + Vec2::new(d, -d)), None, Some(ring));
            }
        }
        let aside = DOT + size::GAP_SM;
        if let Some(id) = &c.id {
            let label = Label::new(p, id.clone(), SMALL, MUTED);
            canvas.label(if right { Label { at: p + Vec2::new(0.0, aside + SMALL * 0.75), ..label } } else { Label { at: p + Vec2::new(aside, 0.0), ..label.start(0.0) } });
        }
        if let Some(tag) = tag {
            let plate = chip_rect(Point::ZERO, tag);
            let shift = if right { Vec2::new(0.0, aside + plate.height() / 2.0) } else { Vec2::new(aside + plate.width() / 2.0, 0.0) };
            canvas.chip(p - shift, tag);
        }
    }
}
