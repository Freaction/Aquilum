use masonry::core::{LayoutCtx, NewWidget, Widget, WidgetMut, WidgetPod};
use masonry::kurbo::{Axis, Size, Vec2};
use masonry::layout::{LenDef, Length, SizeDef};

use super::bar::{Scrollbar, Span};

pub struct ScrollArea<W: Widget> {
    pub(super) child: WidgetPod<W>,
    pub(super) axis: Axis,
    pub(super) offset: f64,
    pub(super) glide: Option<super::glide::Glide>,
    pub(super) wheel: super::glide::Wheel,
    pub(super) edge: f64,
    pub(super) content: f64,
    pub(super) viewport: Size,
    pub(super) overflow: bool,
    pub(super) fill: bool,
    pub(super) bar: Scrollbar,
    pub(super) published: Option<u64>,
    pub(super) thumb: masonry::peniko::ImageBrush,
}

impl<W: Widget> Drop for ScrollArea<W> {
    fn drop(&mut self) {
        if let Some(id) = self.published {
            crate::render::scroll::forget(id);
        }
        crate::render::sprite::forget(&self.thumb);
    }
}

impl<W: Widget> ScrollArea<W> {
    pub fn new(child: NewWidget<W>) -> Self {
        Self::along(Axis::Vertical, child)
    }

    pub fn horizontal(child: NewWidget<W>) -> Self {
        Self::along(Axis::Horizontal, child)
    }

    fn along(axis: Axis, child: NewWidget<W>) -> Self {
        ScrollArea {
            child: child.to_pod(),
            axis,
            offset: 0.0,
            glide: None,
            wheel: super::glide::Wheel::default(),
            edge: 0.0,
            content: 0.0,
            viewport: Size::ZERO,
            overflow: false,
            fill: false,
            bar: Scrollbar::new(axis),
            published: None,
            thumb: crate::render::sprite::marker(),
        }
    }

    pub fn offset(&self) -> f64 {
        self.offset
    }

    pub fn at(mut self, offset: f64) -> Self {
        self.offset = offset.max(0.0);
        self
    }

    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<W>) {
        this.ctx.remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
        this.ctx.children_changed();
    }

    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, W> {
        this.ctx.get_mut(&mut this.widget.child)
    }

    pub fn set_offset(this: &mut WidgetMut<'_, Self>, offset: f64) {
        this.widget.glide = None;
        this.widget.offset = offset.max(0.0);
        this.ctx.request_compose();
        this.ctx.request_post_paint();
    }

    #[cfg(feature = "memory-probe")]
    pub fn set_fraction(this: &mut WidgetMut<'_, Self>, fraction: f64) {
        let offset = this.widget.max_offset() * fraction;
        Self::set_offset(this, offset);
    }

    pub fn page(&self) -> f64 {
        self.view() * super::PAGE
    }

    pub fn scroll_by(this: &mut WidgetMut<'_, Self>, delta: f64) -> f64 {
        let from = this.widget.offset;
        let to = (from + delta).clamp(0.0, this.widget.max_offset());
        Self::set_offset(this, to);
        to - from
    }

    pub fn reveal(this: &mut WidgetMut<'_, Self>, top: f64, bottom: f64) {
        let viewport = this.widget.view();
        let offset = this.widget.offset;
        let next = if top < offset {
            top
        } else if bottom > offset + viewport {
            bottom - viewport
        } else {
            return;
        };
        Self::set_offset(this, next);
    }

    pub(super) fn view(&self) -> f64 {
        self.viewport.get_coord(self.axis)
    }

    pub(super) fn max_offset(&self) -> f64 {
        (self.content - self.view()).max(0.0)
    }

    pub(super) fn span(&self) -> Span {
        Span { track: Scrollbar::edge(self.axis, self.viewport), viewport: self.view(), content: self.content, offset: self.offset }
    }

    pub(super) fn shift(&self, offset: f64) -> Vec2 {
        self.axis.pack_vec2(-offset, 0.0)
    }

    pub(super) fn clamp(&mut self) {
        self.offset = self.offset.clamp(0.0, self.max_offset());
        let max = self.max_offset();
        if self.glide.is_some_and(|g| g.to > max) {
            self.glide = None;
        }
    }

    pub(super) fn jump(&mut self, offset: f64) {
        self.glide = None;
        self.offset = offset.clamp(0.0, self.max_offset());
    }

    pub(super) fn glide_by(&mut self, delta: f64) -> bool {
        self.glide = self.wheel.retarget(self.glide, self.offset, delta, self.max_offset());
        self.glide.is_some()
    }

    pub(super) fn layout_child(&mut self, ctx: &mut LayoutCtx<'_>, size: Size, bar: f64) -> Size {
        match self.axis {
            Axis::Vertical => {
                let width = (size.width - bar).max(0.0);
                let auto = SizeDef::new(LenDef::Fixed(Length::px(width)), LenDef::MaxContent);
                let mut child = ctx.compute_size(&mut self.child, auto, Size::new(width, size.height).into());
                child.width = width;
                if self.fill {
                    child.height = child.height.max(size.height);
                }
                child
            }
            Axis::Horizontal => {
                let height = (size.height - bar).max(0.0);
                let auto = SizeDef::new(LenDef::MaxContent, LenDef::Fixed(Length::px(height)));
                let mut child = ctx.compute_size(&mut self.child, auto, Size::new(size.width, height).into());
                child.height = height;
                child.width = child.width.max(size.width);
                child
            }
        }
    }
}
