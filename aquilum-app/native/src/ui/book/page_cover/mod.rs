mod widget;

use super::*;

pub struct PageCover {
    visual: Visual,
    position: f64,
    repositioning: bool,
    drag: Option<(f64, f64)>,
    actions: WidgetPod<crate::ui::widgets::Appear>,
    book: Option<WidgetPod<TextButton>>,
    show_actions: bool,
    baked: Option<(BakeKey, ImageBrush)>,
    live: Option<ImageBrush>,
}

impl Drop for PageCover {
    fn drop(&mut self) {
        if let Some(marker) = &self.live {
            crate::render::live::forget(marker);
        }
    }
}

impl PageCover {
    pub(super) fn new(visual: Visual, position: f64, actions: NewWidget<ActionBar>, book: Option<NewWidget<TextButton>>) -> NewWidget<Self> {
        NewWidget::new(PageCover { visual, position, repositioning: false, drag: None, actions: crate::ui::widgets::Appear::new(actions, crate::ui::widgets::FAST).to_pod(), book: book.map(NewWidget::to_pod), show_actions: false, baked: None, live: None })
    }

    pub fn refresh(this: &mut WidgetMut<'_, Self>) {
        this.ctx.request_paint_only();
    }

    pub fn position(&self) -> f64 {
        self.position
    }

    pub fn repositioning(&self) -> bool {
        self.repositioning
    }

    pub fn set_repositioning(this: &mut WidgetMut<'_, Self>, on: bool, position: Option<f64>) {
        this.widget.repositioning = on;
        if let Some(position) = position {
            this.widget.position = position;
        }
        this.widget.drag = None;
        let show = on || this.ctx.is_hovered() || this.ctx.has_hovered();
        this.widget.show_actions = show;
        this.ctx.set_stashed(&mut this.widget.actions, !show);
        this.ctx.request_paint_only();
        this.ctx.request_layout();
    }

    fn overflow(&self, size_: Size) -> f64 {
        let Visual::Image(image) = &self.visual else { return 0.0 };
        let (w, h) = (f64::from(image.image.width), f64::from(image.image.height));
        if w <= 0.0 || h <= 0.0 {
            return 0.0;
        }
        (h * (size_.width / w).max(size_.height / h) - size_.height).max(0.0)
    }

    fn motion(&self) -> Option<&'static str> {
        match &self.visual {
            Visual::Pattern(id) if cover::animated(id) => Some(id),
            _ => None,
        }
    }

    fn sync_actions(&mut self, ctx: &mut UpdateCtx<'_>) {
        let show = self.repositioning || ctx.is_hovered() || ctx.has_hovered();
        if show != self.show_actions {
            self.show_actions = show;
            ctx.set_stashed(&mut self.actions, !show);
            ctx.request_layout();
        }
    }
}
