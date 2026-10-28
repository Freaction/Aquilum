use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget,
    WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Insets, Rect, Size};
use masonry::layout::{LenReq, Length};

use crate::ui::theme;
use crate::ui::tokens::size;
use crate::ui::widgets::{Rule, place};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Both,
    List,
    Detail,
}

impl Pane {
    pub fn of(compact: bool, listing: bool) -> Self {
        match (compact, listing) {
            (false, _) => Pane::Both,
            (true, true) => Pane::List,
            (true, false) => Pane::Detail,
        }
    }

    fn shown(self) -> [bool; 5] {
        match self {
            Pane::Both => [true, false, true, true, true],
            Pane::List => [true, false, true, false, false],
            Pane::Detail => [false, true, false, false, true],
        }
    }
}

pub struct Panes {
    pods: [WidgetPod<dyn Widget>; 5],
    pane: Pane,
    insets: Insets,
}

impl Panes {
    pub fn new(top: NewWidget<impl Widget>, sub: NewWidget<impl Widget>, list: NewWidget<impl Widget>, detail: NewWidget<impl Widget>) -> NewWidget<Self> {
        let rule = Rule::new(Axis::Vertical, || theme::current().dialog_border);
        let pods = [top.erased().to_pod(), sub.erased().to_pod(), list.erased().to_pod(), rule.erased().to_pod(), detail.erased().to_pod()];
        NewWidget::new(Panes { pods, pane: Pane::Both, insets: Insets::ZERO })
    }

    pub fn set(this: &mut WidgetMut<'_, Self>, pane: Pane, insets: Insets) {
        if this.widget.pane == pane && this.widget.insets == insets {
            return;
        }
        this.widget.pane = pane;
        this.widget.insets = insets;
        for (pod, shown) in this.widget.pods.iter_mut().zip(pane.shown()) {
            this.ctx.set_stashed(pod, !shown);
        }
        this.ctx.request_layout();
    }
}

impl Widget for Panes {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for pod in &mut self.pods {
            ctx.register_child(pod);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            for (pod, shown) in self.pods.iter_mut().zip(self.pane.shown()) {
                ctx.set_stashed(pod, !shown);
            }
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let insets = if self.pane == Pane::Both { Insets::ZERO } else { self.insets };
        let room = Rect::new(insets.x0, insets.y0, (size.width - insets.x1).max(insets.x0), (size.height - insets.y1).max(insets.y0));
        let [top, sub, list, rule, detail] = &mut self.pods;
        let head = if self.pane == Pane::Detail { sub } else { top };
        let height = ctx.compute_length(head, LenReq::MaxContent.into(), room.size().into(), Axis::Vertical, Some(Length::px(room.width()))).get();
        let line = (room.y0 + height).min(room.y1);
        place(ctx, head, Rect { y1: line, ..room });
        let body = Rect { y0: line, ..room };
        match self.pane {
            Pane::Both => {
                let edge = body.x0 + size::SETTINGS_NAV_WIDTH;
                place(ctx, list, Rect { x1: edge, ..body });
                place(ctx, rule, Rect { x0: edge, x1: edge + size::BORDER_1, ..body });
                place(ctx, detail, Rect { x0: edge + size::BORDER_1, ..body });
            }
            Pane::List => place(ctx, list, body),
            Pane::Detail => place(ctx, detail, body),
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&self.pods.each_ref().map(WidgetPod::id))
    }
}
