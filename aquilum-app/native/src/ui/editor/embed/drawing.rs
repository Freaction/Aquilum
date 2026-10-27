use std::ops::Range;

use masonry::core::{BrushIndex, render_text};
use masonry::imaging::record::Glyph;
use masonry::imaging::{Composite, GroupRef, PaintSink, Painter};
use masonry::kurbo::{Affine, BezPath, Cap, Circle, Join, Point, Rect, RoundedRect, Stroke};
use masonry::parley::Layout;
use masonry::peniko::{BlendMode, Brush, BrushRef, Color, Fill, FontData, ImageBrush, Style};

use super::super::markdown;
use super::metrics::COVER_RADIUS;
use super::panel;
use crate::ui::theme;

const UNDERLINE: f64 = 1.0;
const UNDERLINE_GAP: f64 = 2.0;

#[derive(Clone, Copy)]
pub(super) enum Tone {
    Panel,
    Divider,
    Accent,
    AccentSubtle,
    CoverBg,
}

pub(super) enum Item {
    Text(Point, Layout<BrushIndex>, String),
    Fill(RoundedRect, Tone),
    Image(RoundedRect, ImageBrush, [f64; 4]),
    Check(Rect, bool),
    Bullet(Point),
    Button(RoundedRect, bool),
    Dim(Box<Item>),
    Path(BezPath, Option<Color>, Option<Stroke>),
    Glyphs(FontData, f32, Vec<Glyph>, Option<Color>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Task { target: String, line: usize },
    Link { target: String },
    Read { file: std::path::PathBuf, page: Option<std::path::PathBuf>, title: String },
}

pub struct Drawing {
    pub height: f64,
    pub(super) items: Vec<Item>,
    pub hits: Vec<(Rect, Action, Option<f64>)>,
    pub(super) view: f64,
    pub(super) width: f64,
}

impl Drawing {
    pub(super) fn new(height: f64, items: Vec<Item>) -> Self {
        Drawing { height, items, hits: Vec::new(), view: 0.0, width: 0.0 }
    }

    pub fn max_scroll(&self) -> f64 {
        (self.width - self.view).max(0.0)
    }

    pub fn action_at(&self, p: Point) -> Option<&Action> {
        self.hits.iter().find(|(r, ..)| r.contains(p)).map(|(_, a, _)| a)
    }

    pub fn link_at(&self, p: Point) -> Option<usize> {
        self.hits.iter().position(|(r, action, line)| (line.is_some() || matches!(action, Action::Read { .. })) && r.contains(p))
    }

    pub fn paint_hover(&self, painter: &mut Painter<'_>, x: f64, y: f64, hit: usize) {
        let Some((_, action, _)) = self.hits.get(hit) else { return };
        let same = |j: &usize| self.hits[*j].1 == *action && self.hits[*j].2.is_some();
        let first = (0..hit).rev().take_while(same).last().unwrap_or(hit);
        let last = (hit + 1..self.hits.len()).take_while(same).last().unwrap_or(hit);
        let color = theme::current().text_link;
        for (rect, _, line) in &self.hits[first..=last] {
            if let Some(line) = line {
                painter.fill(Rect::new(x + rect.x0, y + line, x + rect.x1, y + line + UNDERLINE), color).draw();
            }
        }
    }

    pub fn paint(&self, painter: &mut Painter<'_>, x: f64, y: f64, scroll: f64, palette: &[Brush], hover: Option<usize>, search: Option<&crate::ui::editor::Search>) {
        if self.max_scroll() > 0.0 {
            painter.with_fill_clip(Rect::new(x, y, x + self.view, y + self.height), |painter| self.draw(painter, x - scroll, y, palette, hover, search));
        } else {
            self.draw(painter, x, y, palette, hover, search);
        }
    }

    fn draw<S: PaintSink + ?Sized>(&self, painter: &mut Painter<'_, S>, x: f64, y: f64, palette: &[Brush], hover: Option<usize>, search: Option<&crate::ui::editor::Search>) {
        let at = Affine::translate((x, y));
        let hover = hover.and_then(|i| self.hits.get(i)).map(|(r, ..)| *r);
        let mut seen = 0;
        for item in &self.items {
            if let (Some(search), Item::Text(p, layout, text)) = (search, item) {
                let shown = layout.lines().last().map_or(0, |l| l.text_range().end);
                for found in search.ranges(text, &mut seen).into_iter().filter(|r| r.end <= shown) {
                    for (b, _) in markdown::range_rects(layout, found.start, found.end) {
                        painter.fill(Rect::new(x + p.x + b.x0, y + p.y + b.y0, x + p.x + b.x1, y + p.y + b.y1), theme::current().search_match_bg).draw();
                    }
                }
            }
            paint_item(painter, item, at, palette, hover);
        }
    }
}

fn underline_y(layout: &Layout<BrushIndex>, line: usize) -> f64 {
    layout.get(line).map_or(0.0, |l| f64::from(l.metrics().baseline)) + UNDERLINE_GAP
}

pub(super) fn link_hits(layout: &Layout<BrushIndex>, at: Point, links: &[(Range<usize>, String)]) -> Vec<(Rect, Action, Option<f64>)> {
    let mut out = Vec::new();
    for (range, target) in links {
        for (rect, line) in markdown::range_rects(layout, range.start, range.end) {
            out.push((rect + at.to_vec2(), Action::Link { target: target.clone() }, Some(at.y + underline_y(layout, line))));
        }
    }
    out
}

pub(super) fn card(width: f64, height: f64) -> [Item; 1] {
    [Item::Fill(Rect::new(0.0, 0.0, width, height).to_rounded_rect(panel::RADIUS), Tone::Panel)]
}

fn color(tone: Tone) -> Color {
    let tm = theme::current();
    match tone {
        Tone::Panel => panel::Tone::Surface.fill(),
        Tone::Divider => tm.block_divider,
        Tone::Accent => tm.bg_accent,
        Tone::AccentSubtle => tm.bg_accent_subtle,
        Tone::CoverBg => tm.book_callout_cover_bg,
    }
}

fn dimmed<S: PaintSink + ?Sized>(painter: &mut Painter<'_, S>, paint: impl FnOnce(&mut Painter<'_, S>)) {
    painter.with_group(GroupRef::new().with_composite(Composite::new(BlendMode::default(), 0.5)), paint);
}

pub(super) fn paint_item<S: PaintSink + ?Sized>(painter: &mut Painter<'_, S>, item: &Item, at: Affine, palette: &[Brush], hover: Option<Rect>) {
    match item {
        Item::Text(p, layout, _) => render_text(painter, at * Affine::translate(p.to_vec2()), layout, palette, true),
        Item::Fill(shape, tone) => painter.fill(at.transform_rect_bbox(shape.rect()).to_rounded_rect(shape.radii()), color(*tone)).draw(),
        Item::Image(shape, image, [l, t, cr, cb]) => {
            let r = at.transform_rect_bbox(shape.rect());
            let (w, h) = (f64::from(image.image.width) * (cr - l) / 100.0, f64::from(image.image.height) * (cb - t) / 100.0);
            let k = (r.width() / w).max(r.height() / h);
            let x = r.x0 + (r.width() - w * k) / 2.0 - f64::from(image.image.width) * l / 100.0 * k;
            let y = r.y0 + (r.height() - h * k) / 2.0 - f64::from(image.image.height) * t / 100.0 * k;
            painter.with_fill_clip(r.to_rounded_rect(shape.radii()), |painter| painter.draw_image(image, Affine::translate((x, y)) * Affine::scale(k)));
        }
        Item::Check(rect, done) => {
            let tm = theme::current();
            let r = at.transform_rect_bbox(*rect);
            let shape = r.to_rounded_rect(COVER_RADIUS);
            if *done {
                painter.fill(shape, tm.bg_accent).draw();
                let c = r.center();
                let mut check = BezPath::new();
                check.move_to((c.x - 4.5, c.y + 0.5));
                check.line_to((c.x - 1.5, c.y + 3.5));
                check.line_to((c.x + 4.5, c.y - 3.0));
                painter.stroke(&check, &Stroke::new(2.0).with_caps(Cap::Round).with_join(Join::Round), tm.icon_on_accent).draw();
            } else {
                painter.fill(shape, tm.bg_surface_overlay).draw();
            }
        }
        Item::Button(shape, enabled) => {
            let hovered = *enabled && hover == Some(shape.rect());
            let shape = at.transform_rect_bbox(shape.rect()).to_rounded_rect(shape.radii());
            let paint = |painter: &mut Painter<'_, S>| crate::ui::widgets::paint_primary(painter, shape, hovered, theme::current().shadow_action_xs);
            if *enabled {
                paint(painter);
            } else {
                dimmed(painter, paint);
            }
        }
        Item::Bullet(p) => painter.fill(Circle::new(at * *p, 2.5), theme::current().editor_list_marker_color).draw(),
        Item::Dim(inner) => dimmed(painter, |painter| paint_item(painter, inner, at, palette, hover)),
        Item::Path(path, color, stroke) => match stroke {
            Some(stroke) => painter.stroke(path, stroke, ink(color, palette)).transform(at).draw(),
            None => painter.fill(path, ink(color, palette)).transform(at).draw(),
        },
        Item::Glyphs(font, size, glyphs, color) => painter.glyphs(font, ink(color, palette)).transform(at).font_size(*size).hint(true).draw(&Style::Fill(Fill::NonZero), glyphs),
    }
}

fn ink<'a>(color: &'a Option<Color>, palette: &'a [Brush]) -> BrushRef<'a> {
    color.as_ref().map_or(BrushRef::from(&palette[markdown::TEXT.0]), BrushRef::from)
}
