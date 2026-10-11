use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, RoundedRect, Size};
use masonry::peniko::Color;

use crate::ui::theme;
use crate::ui::tokens::size;

const THUMB_INSET: f64 = 3.0;
const THUMB_MIN: f64 = 24.0;
const PAGE: f64 = 0.875;

#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub track: Rect,
    pub viewport: f64,
    pub content: f64,
    pub offset: f64,
}

impl Span {
    pub fn max(&self) -> f64 {
        (self.content - self.viewport).max(0.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Scrollbar {
    axis: Axis,
    hovered: bool,
    drag: Option<(f64, f64)>,
}

impl Scrollbar {
    pub fn new(axis: Axis) -> Self {
        Scrollbar { axis, hovered: false, drag: None }
    }

    pub fn edge(axis: Axis, viewport: Size) -> Rect {
        match axis {
            Axis::Vertical => Rect::new(viewport.width - size::SCROLLBAR_WIDTH, 0.0, viewport.width, viewport.height),
            Axis::Horizontal => Rect::new(0.0, viewport.height - size::SCROLLBAR_WIDTH, viewport.width, viewport.height),
        }
    }

    fn along(&self, p: Point) -> f64 {
        match self.axis {
            Axis::Vertical => p.y,
            Axis::Horizontal => p.x,
        }
    }

    fn length(&self, r: Rect) -> f64 {
        match self.axis {
            Axis::Vertical => r.height(),
            Axis::Horizontal => r.width(),
        }
    }

    pub fn thumb(&self, span: &Span) -> Rect {
        let t = span.track.inflate(-THUMB_INSET, -THUMB_INSET);
        let room = self.length(t);
        let len = (room * (span.viewport / span.content.max(1.0)).min(1.0)).max(THUMB_MIN).min(room);
        let start = if span.max() > 0.0 { (room - len) * span.offset.clamp(0.0, span.max()) / span.max() } else { 0.0 };
        match self.axis {
            Axis::Vertical => Rect::new(t.x0, t.y0 + start, t.x1, t.y0 + start + len),
            Axis::Horizontal => Rect::new(t.x0 + start, t.y0, t.x0 + start + len, t.y1),
        }
    }

    pub fn press(&mut self, at: Point, span: &Span) -> Option<f64> {
        if span.max() <= 0.0 || !span.track.contains(at) {
            return None;
        }
        let thumb = self.thumb(span);
        if thumb.contains(at) {
            self.drag = Some((self.along(at), span.offset));
            return Some(span.offset);
        }
        let page = span.viewport * PAGE;
        let before = self.along(at) < self.along(thumb.origin());
        Some((span.offset + if before { -page } else { page }).clamp(0.0, span.max()))
    }

    pub fn drag(&self, at: Point, span: &Span) -> Option<f64> {
        let (start, offset) = self.drag?;
        let travel = (self.length(span.track) - THUMB_INSET * 2.0 - self.length(self.thumb(span))).max(1.0);
        Some((offset + (self.along(at) - start) / travel * span.max()).clamp(0.0, span.max()))
    }

    pub fn release(&mut self) -> bool {
        self.drag.take().is_some()
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn active(&self) -> bool {
        self.hovered || self.drag.is_some()
    }

    pub fn hover(&mut self, at: Option<Point>, span: &Span) -> bool {
        let hovered = at.is_some_and(|at| span.max() > 0.0 && self.thumb(span).contains(at));
        std::mem::replace(&mut self.hovered, hovered) != hovered
    }

    pub fn set_hovered(&mut self, hovered: bool) {
        self.hovered = hovered;
    }

    pub fn color(&self) -> Color {
        let t = theme::current();
        if self.active() { t.scrollbar_thumb_hover } else { t.scrollbar_thumb }
    }

    pub fn shape(&self, span: &Span) -> RoundedRect {
        let thumb = self.thumb(span);
        RoundedRect::from_rect(thumb, thumb.width().min(thumb.height()) / 2.0)
    }

    pub fn paint(&self, painter: &mut Painter<'_>, span: &Span) {
        painter.fill(span.track, theme::current().scrollbar_track).draw();
        painter.fill(self.shape(span), self.color()).draw();
    }
}
