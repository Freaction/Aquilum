use masonry::kurbo::{Circle, Line, Point, Vec2};
use mermaid_rs_renderer::{Edge, EdgeArrowhead, EdgeDecoration as Mark, EdgeStyle};

use super::super::super::tone::Tone;
use super::super::canvas::{Canvas, LINK, Pen};
use super::super::shape::polygon;

const HEAD: f64 = 9.0;
const MARK: f64 = 6.0;

pub(super) fn cardinality(mark: Option<Mark>) -> Option<&'static str> {
    match mark? {
        Mark::CrowsFootOne => Some("1"),
        Mark::CrowsFootZeroOne => Some("0..1"),
        Mark::CrowsFootMany => Some("1..N"),
        Mark::CrowsFootZeroMany => Some("0..N"),
        _ => None,
    }
}

pub(super) fn pen(style: EdgeStyle, tone: Tone) -> Pen {
    let pen = Pen::new(tone, if style == EdgeStyle::Thick { LINK + 1.0 } else { LINK });
    if style == EdgeStyle::Dotted { pen.dashed() } else { pen }
}

#[derive(Clone, Copy)]
pub(super) enum End {
    None,
    Arrow,
    Hollow,
    Open,
    Mark(Mark),
}

impl End {
    fn of(arrow: bool, kind: Option<EdgeArrowhead>, mark: Option<Mark>) -> End {
        match (mark, arrow, kind) {
            (Some(mark), ..) if cardinality(Some(mark)).is_some() => End::None,
            (Some(mark), ..) => End::Mark(mark),
            (None, true, Some(EdgeArrowhead::OpenTriangle)) => End::Hollow,
            (None, true, Some(EdgeArrowhead::ClassDependency)) => End::Open,
            (None, true, None) => End::Arrow,
            (None, false, _) => End::None,
        }
    }

    pub fn pair(edge: &Edge) -> [End; 2] {
        let start = End::of(edge.arrow_start, edge.arrow_start_kind, edge.start_decoration);
        let end = match End::of(edge.arrow_end, edge.arrow_end_kind, edge.end_decoration) {
            End::None if cardinality(edge.end_decoration).is_some() => End::Arrow,
            end => end,
        };
        [start, end]
    }

    pub fn inset(self) -> f64 {
        match self {
            End::Arrow => HEAD - 1.0,
            End::Hollow => HEAD * 1.3,
            End::Mark(Mark::Circle) => MARK * 1.2,
            End::Mark(Mark::Diamond | Mark::DiamondFilled) => HEAD * 1.2,
            _ => 0.0,
        }
    }

    pub fn draw(self, tip: Point, dir: Vec2, pen: Pen, canvas: &mut Canvas) {
        let side = Vec2::new(-dir.y, dir.x);
        let at = |along: f64, across: f64| tip - dir * along + side * across;
        let pen = Pen { dashed: false, ..pen };
        let line = |canvas: &mut Canvas, a: Point, b: Point| canvas.shape(Line::new(a, b), None, Some(pen));
        let ring = |canvas: &mut Canvas, along: f64| canvas.shape(Circle::new(at(along, 0.0), MARK * 0.6), Some(Tone::Editor), Some(pen));
        match self {
            End::None => {}
            End::Arrow => canvas.shape(polygon(&[tip, at(HEAD, HEAD * 0.45), at(HEAD, -HEAD * 0.45)], true), Some(pen.tone), None),
            End::Hollow => canvas.shape(polygon(&[tip, at(HEAD * 1.3, HEAD * 0.6), at(HEAD * 1.3, -HEAD * 0.6)], true), Some(Tone::Editor), Some(pen)),
            End::Open => canvas.shape(polygon(&[at(HEAD, HEAD * 0.5), tip, at(HEAD, -HEAD * 0.5)], false), None, Some(pen)),
            End::Mark(Mark::Circle) => ring(canvas, MARK * 0.6),
            End::Mark(Mark::Cross) => {
                line(canvas, at(MARK * 1.4, MARK * 0.7), at(0.0, -MARK * 0.7));
                line(canvas, at(MARK * 1.4, -MARK * 0.7), at(0.0, MARK * 0.7));
            }
            End::Mark(mark @ (Mark::Diamond | Mark::DiamondFilled)) => {
                let fill = if mark == Mark::Diamond { Tone::Editor } else { pen.tone };
                canvas.shape(polygon(&[tip, at(HEAD * 0.6, HEAD * 0.45), at(HEAD * 1.2, 0.0), at(HEAD * 0.6, -HEAD * 0.45)], true), Some(fill), Some(pen));
            }
            End::Mark(_) => {}
        }
    }
}
