mod math;
mod text;

use masonry::core::BrushIndex;
use masonry::kurbo::{Affine, BezPath, Point, Rect, RoundedRect, Shape, Stroke};
use masonry::parley::Layout;

use super::super::super::markdown::SECONDARY;
use super::super::drawing::{Drawing, Item};
use super::super::panel::PAD;
use super::super::tone::{Paint, Tone};
use super::super::{Font, Spec};
use crate::ui::tokens::size;
pub(super) use text::{Label, Text, chip_rect, clean, single, width, wrap};
use text::{CHIP, aligned};

pub(super) const FONT: f64 = 14.0;
pub(super) const LINE: f64 = FONT * 1.5;
pub(super) const BORDER: f64 = 1.0;
pub(super) const LINK: f64 = 1.5;
pub(super) const BOLD: f64 = 1.08;
const MIN_SCALE: f64 = 0.85;
const SLACK: f64 = 4.0;
const MARGIN: f64 = PAD * 3.0;

#[derive(Clone, Copy)]
pub(super) struct Pen {
    pub tone: Tone,
    pub width: f64,
    pub dashed: bool,
}

impl Pen {
    pub fn new(tone: Tone, width: f64) -> Self {
        Pen { tone, width, dashed: false }
    }

    pub fn border(tone: Tone) -> Self {
        Pen::new(tone, BORDER)
    }

    pub fn dashed(self) -> Self {
        Pen { dashed: true, ..self }
    }
}

fn plate(r: Rect, items: &mut Vec<Item>) {
    let path = RoundedRect::from_rect(r, size::RADIUS_LG).to_path(0.1);
    items.push(Item::Path(path.clone(), Paint::Tone(Tone::Editor), None));
    items.push(Item::Path(path, Paint::Tone(Tone::Border), Some(Stroke::new(BORDER))));
}

#[derive(Default)]
pub(super) struct Canvas {
    shapes: Vec<(BezPath, Option<Tone>, Option<Pen>)>,
    labels: Vec<Label>,
    bounds: Option<Rect>,
}

impl Canvas {
    pub fn shape(&mut self, path: impl Shape, fill: Option<Tone>, pen: Option<Pen>) {
        let path = path.to_path(0.1);
        let spread = pen.map_or(0.0, |pen| pen.width / 2.0);
        self.grow(path.bounding_box().inflate(spread, spread));
        let pen = match (fill, pen) {
            (Some(Tone::Panel(hue)), None) => Some(Pen::border(Tone::Ink(hue))),
            _ => pen,
        };
        if let Some(Tone::Panel(_)) = fill {
            self.shapes.push((path.clone(), Some(Tone::Editor), None));
        }
        self.shapes.push((path, fill, pen));
    }

    pub fn wash(&mut self, path: impl Shape, tone: Tone) {
        let path = path.to_path(0.1);
        self.grow(path.bounding_box());
        self.shapes.push((path, Some(tone), None));
    }

    pub fn label(&mut self, label: Label) {
        if label.text.trim().is_empty() {
            return;
        }
        self.grow(label.rect());
        self.labels.push(label);
    }

    pub fn chip(&mut self, at: Point, text: &Text) {
        self.grow(chip_rect(at, text));
        self.label(Label { chip: true, ..Label::block(at, text, CHIP, SECONDARY) });
    }

    fn grow(&mut self, rect: Rect) {
        self.bounds = Some(self.bounds.map_or(rect, |b| b.union(rect)));
    }

    pub fn finish(self, available: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) -> Option<Drawing> {
        let bounds = self.bounds.filter(|b| b.width() > 0.0 && b.height() > 0.0)?;
        let scale = ((available - PAD * 2.0) / bounds.width()).clamp(MIN_SCALE, 1.0);
        let width = bounds.width() * scale + PAD * 2.0;
        let left = ((available - width) / 2.0).max(0.0) + PAD;
        let at = Affine::translate((left, MARGIN)) * Affine::scale(scale) * Affine::translate(-bounds.origin().to_vec2());
        let mut items = Vec::with_capacity(self.shapes.len() + self.labels.len());
        for (mut path, fill, pen) in self.shapes {
            path.apply_affine(at);
            if let Some(tone) = fill {
                items.push(Item::Path(path.clone(), Paint::Tone(tone), None));
            }
            if let Some(pen) = pen {
                let stroke = Stroke::new(pen.width);
                let stroke = if pen.dashed { stroke.with_dashes(0.0, [pen.width * 4.0, pen.width * 3.0]) } else { stroke };
                items.push(Item::Path(path, Paint::Tone(pen.tone), Some(stroke)));
            }
        }
        for label in self.labels {
            if label.text.contains("$$") {
                math::mixed(&label, at, scale, make, &mut items);
                continue;
            }
            let px = (label.size * scale).round().clamp(8.0, 40.0) as u8;
            let tall = (label.rect().height() + size::GAP_XS * 2.0) * scale;
            let wrap = (label.width > 0.0 && !label.chip).then(|| label.width * scale * 1.1 + SLACK);
            let layout = make(&Spec { bold: label.bold, wrap, align: label.align, ..Spec::plain(&label.text, Font::Ui(px), label.brush) });
            let anchor = at * label.at;
            let (w, h) = (wrap.unwrap_or(f64::from(layout.width())), f64::from(layout.height()));
            if label.chip {
                plate(Rect::from_center_size(anchor, (w + size::GAP_MD * 2.0 * scale, tall)), &mut items);
            }
            let x = aligned(label.align, anchor.x, w);
            items.push(Item::Text(Point::new(x, anchor.y - h / 2.0), layout, label.text));
        }
        let mut drawing = Drawing::card(available, bounds.height() * scale + MARGIN * 2.0, items);
        drawing.width = drawing.width.max(width);
        Some(drawing)
    }
}
