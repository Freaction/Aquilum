use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, ComposeCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget,
    WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Rect, Size, Vec2};
use masonry::layout::{LayoutSize, LenReq, Length};

use super::scrim::Scrim;
use crate::interface::{Screen, Side, drawer_width};
use crate::ui::motion::{self, approach};
use crate::ui::tokens::size;
use crate::ui::widgets::{EASE_STANDARD, bezier, place};

pub struct Stage {
    main: WidgetPod<dyn Widget>,
    scrim: WidgetPod<dyn Widget>,
    rail: WidgetPod<dyn Widget>,
    sidebar: WidgetPod<dyn Widget>,
    panel: WidgetPod<dyn Widget>,
    screen: Screen,
    side: Side,
    progress: f64,
    reach: (f64, f64),
}

impl Stage {
    pub fn new(main: NewWidget<dyn Widget>, [rail, sidebar, panel]: [NewWidget<dyn Widget>; 3], screen: Screen) -> NewWidget<Self> {
        NewWidget::new(Stage {
            main: main.to_pod(),
            scrim: Scrim::new().erased().to_pod(),
            rail: rail.to_pod(),
            sidebar: sidebar.to_pod(),
            panel: panel.to_pod(),
            screen,
            side: screen.drawer.unwrap_or(Side::Sidebar),
            progress: if screen.drawer.is_some() { 1.0 } else { 0.0 },
            reach: (0.0, 0.0),
        })
    }

    pub fn scrim(&self) -> masonry::core::WidgetId {
        self.scrim.id()
    }

    pub fn set_screen(this: &mut WidgetMut<'_, Self>, screen: Screen) {
        let stage = &mut *this.widget;
        if stage.screen == screen {
            return;
        }
        if let Some(side) = screen.drawer.filter(|&side| side != stage.side) {
            stage.side = side;
            stage.progress = 0.0;
        }
        if !screen.compact {
            stage.progress = 0.0;
        }
        stage.screen = screen;
        stage.stash(|pod, hidden| this.ctx.set_stashed(pod, hidden));
        if stage.progress != stage.goal() {
            this.ctx.request_anim_frame();
        }
        this.ctx.request_layout();
    }

    fn goal(&self) -> f64 {
        if self.screen.drawer.is_some() { 1.0 } else { 0.0 }
    }

    fn eased(&self) -> f64 {
        bezier(EASE_STANDARD, self.progress)
    }

    fn shown(&self) -> Option<Side> {
        if !self.screen.compact {
            return None;
        }
        self.screen.drawer.or((self.progress > 0.0).then_some(self.side))
    }

    fn hidden(&self) -> [bool; 4] {
        let shown = self.shown();
        let away = |side| self.screen.compact && shown != Some(side);
        [shown.is_none(), away(Side::Sidebar), away(Side::Sidebar), away(Side::Panel)]
    }

    fn stash(&mut self, mut set: impl FnMut(&mut WidgetPod<dyn Widget>, bool)) {
        let hidden = self.hidden();
        for (pod, hidden) in [&mut self.scrim, &mut self.rail, &mut self.sidebar, &mut self.panel].into_iter().zip(hidden) {
            set(pod, hidden);
        }
    }

    fn row(&mut self, ctx: &mut LayoutCtx<'_>, room: Rect) {
        let height = room.height();
        let cross = Some(Length::px(height));
        let width = |ctx: &mut LayoutCtx<'_>, pod: &mut WidgetPod<dyn Widget>| {
            ctx.compute_length(pod, LenReq::MaxContent.into(), LayoutSize::maybe(Axis::Vertical, cross), Axis::Horizontal, cross).get()
        };
        let (rail, sidebar, panel) = (width(ctx, &mut self.rail), width(ctx, &mut self.sidebar), width(ctx, &mut self.panel));
        let main = (room.width() - rail - sidebar - panel).max(0.0);
        self.reach = (0.0, 0.0);
        let mut x = room.x0;
        for (pod, width) in [(&mut self.rail, rail), (&mut self.sidebar, sidebar), (&mut self.main, main), (&mut self.panel, panel)] {
            place(ctx, pod, Rect { x0: x, x1: x + width, ..room });
            x += width;
        }
    }

    fn drawers(&mut self, ctx: &mut LayoutCtx<'_>, room: Rect) {
        let [scrim, left, _, right] = self.hidden();
        place(ctx, &mut self.main, room);
        if !scrim {
            place(ctx, &mut self.scrim, room);
        }
        if !left {
            let (rail, width) = (room.x0 + size::SIDEBAR_RAIL_WIDTH, drawer_width(room.width(), Side::Sidebar));
            self.reach.0 = size::SIDEBAR_RAIL_WIDTH + width;
            place(ctx, &mut self.rail, Rect { x1: rail, ..room });
            place(ctx, &mut self.sidebar, Rect { x0: rail, x1: rail + width, ..room });
        }
        if !right {
            let width = drawer_width(room.width(), Side::Panel);
            self.reach.1 = width;
            place(ctx, &mut self.panel, Rect { x0: room.x1 - width, ..room });
        }
    }
}

impl Widget for Stage {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for pod in [&mut self.main, &mut self.scrim, &mut self.rail, &mut self.sidebar, &mut self.panel] {
            ctx.register_child(pod);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            self.stash(|pod, hidden| ctx.set_stashed(pod, hidden));
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let goal = self.goal();
        self.progress = approach(self.progress, goal, motion::step(interval as f64 / 1e9, motion::NORMAL));
        self.stash(|pod, hidden| ctx.set_stashed(pod, hidden));
        ctx.request_compose();
        if self.progress != goal {
            ctx.request_anim_frame();
        }
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        let eased = self.eased();
        let (left, right) = (self.reach.0 * (eased - 1.0), self.reach.1 * (1.0 - eased));
        ctx.set_child_scroll_translation(&mut self.rail, Vec2::new(left, 0.0));
        ctx.set_child_scroll_translation(&mut self.sidebar, Vec2::new(left, 0.0));
        ctx.set_child_scroll_translation(&mut self.panel, Vec2::new(right, 0.0));
        ctx.get_raw(&mut self.scrim).1.set_opacity(eased as f32);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let insets = self.screen.insets;
        let room = Rect::new(insets.x0, insets.y0, (size.width - insets.x1).max(insets.x0), (size.height - insets.y1).max(insets.y0));
        if self.screen.compact {
            self.drawers(ctx, room);
        } else {
            self.row(ctx, room);
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.main.id(), self.scrim.id(), self.rail.id(), self.sidebar.id(), self.panel.id()])
    }
}
