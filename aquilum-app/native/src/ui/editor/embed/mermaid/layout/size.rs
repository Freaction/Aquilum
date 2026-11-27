use masonry::kurbo::Point;
use mermaid_rs_renderer::NodeShape;

use super::super::canvas::{BOLD, Text, chip_rect, wrap};
use crate::ui::tokens::size;

const WRAP: f64 = 200.0;
const CHIP_WRAP: f64 = 160.0;
const MARKER: f64 = 14.0;
const ROW: f64 = 20.0;
const BAR: (f64, f64) = (72.0, 8.0);

pub(super) fn text(label: &str) -> Text {
    wrap(label, WRAP)
}

pub(super) fn chip(label: &str) -> (Text, (f64, f64)) {
    let block = wrap(label, CHIP_WRAP);
    let r = chip_rect(Point::ZERO, &block);
    (block, (r.width(), r.height()))
}

pub(super) fn node(shape: NodeShape, label: &Text, across: bool) -> (f64, f64) {
    let (tw, th) = (label.width, label.height());
    let pad = size::GAP_2XL;
    let empty = label.lines.iter().all(|line| line.trim().is_empty());
    match shape {
        _ if label.lines.iter().any(|line| line.trim() == "---") => (tw * BOLD + size::GAP_XL * 2.0, ROW * (label.lines.len() + 1) as f64),
        NodeShape::Circle | NodeShape::DoubleCircle if empty => (MARKER, MARKER),
        NodeShape::Circle | NodeShape::DoubleCircle => {
            let d = tw.hypot(th) + pad;
            (d, d)
        }
        NodeShape::Diamond => {
            let d = tw + th + pad * 1.5;
            (d, d)
        }
        NodeShape::ForkJoin if across => (BAR.1, BAR.0),
        NodeShape::ForkJoin => BAR,
        NodeShape::Text => (tw, th),
        NodeShape::Stadium | NodeShape::Hexagon => (tw + th + pad * 2.0, th + pad * 1.5),
        NodeShape::Cylinder => (tw + pad * 3.0, th + pad * 2.5),
        NodeShape::Parallelogram | NodeShape::ParallelogramAlt | NodeShape::Trapezoid | NodeShape::TrapezoidAlt | NodeShape::Asymmetric => (tw + th + pad * 3.0, th + pad * 2.0),
        _ => (tw + pad * 3.0, th + pad * 2.0),
    }
}
