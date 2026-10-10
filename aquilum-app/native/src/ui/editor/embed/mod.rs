mod book;
mod dataview;
pub mod media;

use std::ops::Range;

use masonry::core::{BrushIndex, render_text};
use masonry::imaging::{PaintSink, Painter};
use masonry::kurbo::{Affine, BezPath, Cap, Circle, Join, Point, Rect, RoundedRect, Stroke};
use masonry::parley::{Alignment, Layout};
use masonry::peniko::{Brush, Color, ImageBrush};

use super::table::{Overlay, TableView};
use crate::ui::theme;
use crate::ui::tokens::size;

pub use book::{Row as BookRow, books};
pub use dataview::dataview;
pub use media::{MediaHit, MediaOverlay, MediaView};


pub(super) const PAD: f64 = 12.0;
pub(super) const CELL_X: f64 = 6.0;
pub(super) const CELL_Y: f64 = 4.0;
pub(super) const PENDING: f64 = 96.0;
pub(super) const RADIUS: f64 = 12.0;
pub(super) const BAR_HEIGHT: f64 = 8.0;
pub(super) const BAR_MIN: f64 = 120.0;
pub(super) const CHECK: f64 = 20.0;
pub(super) const CHECK_GAP: f64 = 12.0;
pub(super) const BULLET_INDENT: f64 = 12.0;
pub(super) const IMAGE_PAD_Y: f64 = 8.0;
pub(super) const IMAGE_PAD_X: f64 = 4.0;
pub(super) const COVER_W: f64 = 56.0;
pub(super) const COVER_H: f64 = 72.0;
pub(super) const COVER_RADIUS: f64 = 6.0;
pub(super) const NATURAL_MAX: f64 = 360.0;
const UNDERLINE: f64 = 1.0;
const UNDERLINE_GAP: f64 = 2.0;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    Editor,
    Ui(u8),
    Mono(u8),
}

pub struct Spec<'a> {
    pub text: &'a str,
    pub font: Font,
    pub brush: BrushIndex,
    pub bold: bool,
    pub weight: Option<f32>,
    pub links: &'a [Range<usize>],
    pub underline: bool,
    pub wrap: Option<f64>,
    pub align: Alignment,
    pub markdown: bool,
}

impl<'a> Spec<'a> {
    pub fn plain(text: &'a str, font: Font, brush: BrushIndex) -> Self {
        Spec { text, font, brush, bold: false, weight: None, links: &[], underline: false, wrap: None, align: Alignment::Start, markdown: false }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Tone {
    BlockBg,
    BlockBorder,
    Divider,
    Accent,
    AccentSubtle,
    CoverBg,
}

pub(super) enum Item {
    Text(Point, Layout<BrushIndex>),
    Fill(RoundedRect, Tone),
    Stroke(RoundedRect, Tone),
    Image(RoundedRect, ImageBrush, [f64; 4]),
    Check(Rect, bool),
    Bullet(Point),
    Button(RoundedRect, bool),
    Dim(Box<Item>),
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
}

impl Drawing {
    pub(super) fn new(height: f64, items: Vec<Item>) -> Self {
        Drawing { height, items, hits: Vec::new() }
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
}

fn underline_y(layout: &Layout<BrushIndex>, line: usize) -> f64 {
    layout.get(line).map_or(0.0, |l| f64::from(l.metrics().baseline)) + UNDERLINE_GAP
}

pub(super) fn link_hits(layout: &Layout<BrushIndex>, at: Point, links: &[(Range<usize>, String)]) -> Vec<(Rect, Action, Option<f64>)> {
    use masonry::parley::{Affinity, Cursor, Selection};
    let mut out = Vec::new();
    for (range, target) in links {
        let selection = Selection::new(Cursor::from_byte_index(layout, range.start, Affinity::Downstream), Cursor::from_byte_index(layout, range.end, Affinity::Upstream));
        for (b, line) in selection.geometry(layout) {
            out.push((Rect::new(b.x0, b.y0, b.x1, b.y1) + at.to_vec2(), Action::Link { target: target.clone() }, Some(at.y + underline_y(layout, line))));
        }
    }
    out
}

pub enum View {
    Table(TableView),
    Media(MediaView),
    Drawing(Drawing),
}

impl View {
    pub fn height(&self) -> f64 {
        match self {
            View::Table(t) => t.height,
            View::Media(m) => m.height,
            View::Drawing(d) => d.height,
        }
    }

    pub fn paint(&self, painter: &mut Painter<'_>, x: f64, y: f64, palette: &[Brush], overlay: &Overlay, media: &MediaOverlay, hover: Option<usize>) {
        match self {
            View::Table(t) => t.paint(painter, x, y, palette, overlay),
            View::Media(m) => m.paint(painter, x, y, media),
            View::Drawing(d) => d.paint(painter, x, y, palette, hover),
        }
    }
}

fn color(tone: Tone) -> Color {
    let tm = theme::current();
    match tone {
        Tone::BlockBg => tm.block_bg,
        Tone::BlockBorder => tm.block_border,
        Tone::Divider => tm.block_divider,
        Tone::Accent => tm.bg_accent,
        Tone::AccentSubtle => tm.bg_accent_subtle,
        Tone::CoverBg => tm.book_callout_cover_bg,
    }
}

fn paint_item<S: PaintSink + ?Sized>(painter: &mut Painter<'_, S>, item: &Item, at: Affine, palette: &[Brush], hover: Option<Rect>) {
    match item {
        Item::Text(p, layout) => render_text(painter, at * Affine::translate(p.to_vec2()), layout, palette, true),
        Item::Fill(shape, tone) => painter.fill(at.transform_rect_bbox(shape.rect()).to_rounded_rect(shape.radii()), color(*tone)).draw(),
        Item::Stroke(shape, tone) => {
            let r = at.transform_rect_bbox(shape.rect());
            painter.stroke(Rect::new(r.x0 + 0.5, r.y0 + 0.5, r.x1 - 0.5, r.y1 - 0.5).to_rounded_rect(shape.radii()), &Stroke::new(size::BORDER_1), color(*tone)).draw();
        }
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
                painter.with_group(masonry::imaging::GroupRef::new().with_composite(masonry::imaging::Composite::new(masonry::peniko::BlendMode::default(), 0.5)), paint);
            }
        }
        Item::Bullet(p) => painter.fill(Circle::new(at * *p, 2.5), theme::current().editor_list_marker_color).draw(),
        Item::Dim(inner) => painter.with_group(masonry::imaging::GroupRef::new().with_composite(masonry::imaging::Composite::new(masonry::peniko::BlendMode::default(), 0.5)), |painter| {
            paint_item(painter, inner, at, palette, hover)
        }),
    }
}

impl Drawing {
    fn paint(&self, painter: &mut Painter<'_>, x: f64, y: f64, palette: &[Brush], hover: Option<usize>) {
        let at = Affine::translate((x, y));
        let hover = hover.and_then(|i| self.hits.get(i)).map(|(r, ..)| *r);
        for item in &self.items {
            paint_item(painter, item, at, palette, hover);
        }
    }
}

pub(super) fn card(width: f64, height: f64) -> [Item; 2] {
    let shape = Rect::new(0.0, 0.0, width, height).to_rounded_rect(RADIUS);
    [Item::Fill(shape, Tone::BlockBg), Item::Stroke(shape, Tone::BlockBorder)]
}

