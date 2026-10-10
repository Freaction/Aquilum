//! Структура и методы ScrollArea.

use masonry::core::{LayoutCtx, NewWidget, Widget, WidgetMut, WidgetPod};
use masonry::kurbo::Size;
use masonry::layout::{LenDef, Length, SizeDef};

#[derive(Clone, Copy)]
pub(super) struct Glide {
    pub from: f64,
    pub to: f64,
    pub speed: f64,
    pub elapsed: f64,
}

impl Glide {
    pub fn at(&self, duration: f64) -> (f64, f64) {
        let s = (self.elapsed / duration).min(1.0);
        let (s2, s3) = (s * s, s * s * s);
        let position = (2.0 * s3 - 3.0 * s2 + 1.0) * self.from + (s3 - 2.0 * s2 + s) * duration * self.speed + (3.0 * s2 - 2.0 * s3) * self.to;
        let speed = (6.0 * s2 - 6.0 * s) * (self.from - self.to) / duration + (3.0 * s2 - 4.0 * s + 1.0) * self.speed;
        let (lo, hi) = (self.from.min(self.to), self.from.max(self.to));
        (position.clamp(lo, hi), speed)
    }
}

pub struct ScrollArea<W: Widget> {
    pub(super) child: WidgetPod<W>,
    pub(super) offset: f64,
    pub(super) glide: Option<Glide>,
    pub(super) content_height: f64,
    pub(super) viewport: Size,
    /// Содержимое выше видимой области — полоса показана и занимает место.
    pub(super) overflow: bool,
    /// Содержимое не ниже видимой области — чтобы его можно было центрировать по вертикали.
    pub(super) fill: bool,
    pub(super) thumb_hovered: bool,
    /// Перетаскивание ползунка: где нажали (y) и смещение в тот момент.
    pub(super) dragging: Option<(f64, f64)>,
}

impl<W: Widget> ScrollArea<W> {
    pub fn new(child: NewWidget<W>) -> Self {
        ScrollArea {
            child: child.to_pod(),
            offset: 0.0,
            glide: None,
            content_height: 0.0,
            viewport: Size::ZERO,
            overflow: false,
            fill: false,
            thumb_hovered: false,
            dragging: None,
        }
    }

    pub fn offset(&self) -> f64 {
        self.offset
    }

    pub fn at(mut self, offset: f64) -> Self {
        self.offset = offset.max(0.0);
        self
    }

    /// Содержимое не ниже видимой области (для пустых состояний по центру).
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<W>) {
        this.ctx.remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
        this.ctx.children_changed();
    }

    pub fn set_offset(this: &mut WidgetMut<'_, Self>, offset: f64) {
        this.widget.glide = None;
        this.widget.offset = offset.max(0.0);
        this.ctx.request_layout();
    }

    /// Прокрутить ровно настолько, чтобы полоса `[top, bottom]` содержимого стала видна.
    pub fn reveal(this: &mut WidgetMut<'_, Self>, top: f64, bottom: f64) {
        let viewport = this.widget.viewport.height;
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

    pub(super) fn max_offset(&self) -> f64 {
        (self.content_height - self.viewport.height).max(0.0)
    }

    pub(super) fn clamp(&mut self) {
        self.offset = self.offset.clamp(0.0, self.max_offset());
        let max = self.max_offset();
        if let Some(glide) = &mut self.glide {
            glide.to = glide.to.clamp(0.0, max);
        }
    }

    pub(super) fn jump(&mut self, offset: f64) {
        self.glide = None;
        self.offset = offset.clamp(0.0, self.max_offset());
    }

    pub(super) fn glide_by(&mut self, delta: f64) -> bool {
        let (base, speed) = match self.glide {
            Some(g) if (g.to - self.offset) * delta > 0.0 => (g.to, g.at(super::GLIDE_SECONDS).1),
            _ => (self.offset, 0.0),
        };
        let to = (base + delta).clamp(0.0, self.max_offset());
        if to == self.offset {
            self.glide = None;
            return false;
        }
        self.glide = Some(Glide { from: self.offset, to, speed, elapsed: 0.0 });
        true
    }

    pub(super) fn layout_child(&mut self, ctx: &mut LayoutCtx<'_>, width: f64, height: f64) -> Size {
        let auto = SizeDef::new(LenDef::Fixed(Length::px(width)), LenDef::MaxContent);
        let mut child = ctx.compute_size(&mut self.child, auto, Size::new(width, height).into());
        child.width = width;
        if self.fill {
            child.height = child.height.max(height);
        }
        child
    }
}
