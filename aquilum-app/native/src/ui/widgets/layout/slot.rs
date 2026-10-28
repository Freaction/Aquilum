use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut,
    WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};

use crate::ui::widgets::Intro;

pub struct Slot {
    child: WidgetPod<dyn Widget>,
    fit: bool,
    shown: bool,
    intro: Option<Intro>,
}

impl Slot {
    pub fn new(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(Slot { child: child.to_pod(), fit: false, shown: true, intro: None })
    }

    pub fn fit(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        Self::shown(child, true)
    }

    pub fn shown(child: NewWidget<impl Widget + ?Sized>, shown: bool) -> NewWidget<Self> {
        NewWidget::new(Slot { child: child.erased().to_pod(), fit: true, shown, intro: None })
    }

    pub fn appear(mut slot: NewWidget<Self>, seconds: f64) -> NewWidget<Self> {
        slot.widget.intro = Some(Intro::new(seconds));
        slot
    }

    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<dyn Widget>) {
        this.ctx.remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
        this.ctx.children_changed();
    }

    pub fn clear(this: &mut WidgetMut<'_, Self>) {
        Self::set_child(this, NewWidget::new(masonry::widgets::SizedBox::empty()).erased());
    }

    pub fn set_shown(this: &mut WidgetMut<'_, Self>, shown: bool) {
        if this.widget.shown != shown {
            this.widget.shown = shown;
            this.ctx.set_stashed(&mut this.widget.child, !shown);
            this.ctx.request_layout();
        }
    }

    fn fade_in(&mut self, ctx: &mut UpdateCtx<'_>) {
        if let Some(intro) = &mut self.intro {
            intro.restart();
            ctx.set_opacity(intro.value() as f32);
            ctx.request_anim_frame();
        }
    }
}

impl Widget for Slot {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                ctx.set_stashed(&mut self.child, !self.shown);
                self.fade_in(ctx);
            }
            Update::StashedChanged(false) => self.fade_in(ctx),
            Update::StashedChanged(true) => self.intro.iter_mut().for_each(Intro::finish),
            _ => {}
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        if let Some(intro) = &mut self.intro {
            if intro.advance(interval) {
                ctx.request_anim_frame();
            }
            ctx.set_opacity(intro.value() as f32);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        if !self.shown {
            return Length::ZERO;
        }
        if self.fit {
            let context = ctx.context_size();
            return ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        }
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        if self.shown {
            ctx.run_layout(&mut self.child, size);
            ctx.place_child(&mut self.child, Point::ORIGIN);
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
