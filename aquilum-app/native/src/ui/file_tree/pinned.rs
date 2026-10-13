use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerButton, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget,
    WidgetMut,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Rect, Size};
use masonry::layout::{LenReq, Length};

use super::model::Row;
use super::row::{Look, Texts, paint_row};
use super::step;
use crate::ui::theme;
use crate::ui::tokens::size;

#[derive(Debug)]
pub struct Unpin(pub usize);

struct Held {
    index: usize,
    row: Row,
    texts: Texts,
}

pub struct Pinned {
    held: Vec<Held>,
    hover: Option<usize>,
    pressed: Option<usize>,
}

impl Pinned {
    pub fn new(pins: &[usize], rows: &[Row]) -> Self {
        Pinned { held: Self::build(Vec::new(), pins, rows), hover: None, pressed: None }
    }

    fn build(mut old: Vec<Held>, pins: &[usize], rows: &[Row]) -> Vec<Held> {
        pins.iter()
            .map(|&index| {
                let row = rows[index].clone();
                let texts = old.iter().position(|h| h.row == row).map(|at| old.swap_remove(at).texts).unwrap_or_default();
                Held { index, row, texts }
            })
            .collect()
    }

    pub fn set(this: &mut WidgetMut<'_, Self>, pins: &[usize], rows: &[Row]) {
        let held = &this.widget.held;
        if held.len() == pins.len() && held.iter().zip(pins).all(|(h, &index)| h.index == index && h.row == rows[index]) {
            return;
        }
        let resized = held.len() != pins.len();
        let old = std::mem::take(&mut this.widget.held);
        this.widget.held = Self::build(old, pins, rows);
        if resized {
            this.ctx.request_layout();
        }
        this.ctx.request_paint_only();
    }

    #[cfg(test)]
    pub fn names(&self) -> Vec<String> {
        self.held.iter().map(|h| h.row.name.clone()).collect()
    }

    fn at(&self, y: f64) -> Option<usize> {
        (y >= 0.0).then(|| (y / step()) as usize).filter(|&at| at < self.held.len())
    }
}

impl Widget for Pinned {
    type Action = Unpin;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Move(update) => {
                let at = self.at(ctx.local_position(update.current.position).y);
                if at != self.hover {
                    self.hover = at;
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.pressed = self.at(ctx.local_position(e.state.position).y);
                ctx.capture_pointer();
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) => {
                let at = self.at(ctx.local_position(e.state.position).y);
                if let Some(held) = at.filter(|_| at == self.pressed).and_then(|at| self.held.get(at)) {
                    ctx.submit_action::<Unpin>(Unpin(held.index));
                }
                self.pressed = None;
            }
            PointerEvent::Cancel(_) => self.pressed = None,
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(false) if self.hover.take().is_some() => ctx.request_paint_only(),
            Update::FontsChanged => {
                self.held.iter_mut().for_each(|h| h.texts.clear());
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(self.held.len() as f64 * step()),
            (Axis::Horizontal, LenReq::FitContent(space)) => Length::px((space.get() - size::SCROLLBAR_WIDTH).max(0.0)),
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.set_clip_path(size.to_rect());
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if self.held.is_empty() {
            return;
        }
        let t = theme::current();
        let bounds = ctx.content_box();
        let (left, right) = (bounds.x0 + size::SPACE_8, bounds.x1 - size::SPACE_8);
        let hover = self.hover;
        for (at, held) in self.held.iter_mut().enumerate() {
            let top = bounds.y0 + at as f64 * step();
            painter.fill(Rect::new(bounds.x0, top, bounds.x1, top + step()), t.sidebar_bg).draw();
            let row = Rect::new(left, top, right, top + step() - size::SPACE_2);
            paint_row(ctx, painter, row, &held.row, Look { hovered: hover == Some(at), drop_over: false }, &mut held.texts);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
