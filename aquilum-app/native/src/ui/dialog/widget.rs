use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::KeyState;
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::widgets::Flex;
use masonry::layout::{LenReq, Length};

use super::{CardSize, DialogAnswer, Modal};
use crate::ui::keymap::{self, Command, Scope};
use crate::render::fade::Fade;
use crate::ui::menu::{box_shadow, shadow_reach};
use crate::ui::theme;
use crate::ui::tokens::size;

const LIFT: f64 = 8.0;
const SHRINK: f64 = 0.02;

impl Widget for Modal {
    type Action = DialogAnswer;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        if let TextEvent::Keyboard(k) = event
            && k.state == KeyState::Down
            && !self.closing
        {
            match keymap::command(&[Scope::Dialog], &k.key, k.modifiers) {
                Some(Command::Close) => ctx.submit_action::<DialogAnswer>(DialogAnswer::Cancel),
                Some(Command::Confirm) if self.enter_confirms => {
                    ctx.submit_action::<DialogAnswer>(DialogAnswer::Confirm);
                }
                _ => {}
            }
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn find_widget_under_pointer<'c>(&'c self, ctx: masonry::core::QueryCtx<'c>, pos: Point) -> Option<masonry::core::WidgetRef<'c, dyn Widget>> {
        if self.closing {
            return None;
        }
        masonry::core::find_widget_under_pointer(self, ctx, pos)
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            self.id = Some(ctx.widget_id().to_raw());
            ctx.request_anim_frame();
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        if self.intro.advance(interval) {
            ctx.request_anim_frame();
        } else if self.closing {
            ctx.submit_action::<DialogAnswer>(DialogAnswer::Closed);
        }
        ctx.request_paint_only();
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.card);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let room = Size::new((size.width - size::SPACE_16 * 2.0).max(0.0), (size.height - size::SPACE_16 * 2.0).max(0.0));
        let fit = |ctx: &mut LayoutCtx<'_>, card: &mut WidgetPod<Flex>, frame: Size| {
            let height = ctx.compute_length(card, LenReq::MaxContent.into(), size.into(), Axis::Vertical, Some(Length::px(frame.width))).get();
            Size::new(frame.width, height.min(frame.height))
        };
        let (card, anchor) = match self.size {
            CardSize::Fit(max) => {
                let card = fit(ctx, &mut self.card, Size::new(room.width.min(max), room.height));
                (card, card)
            }
            CardSize::Window(of) => {
                let card = of(room);
                (card, card)
            }
            CardSize::Anchored(of) => {
                let frame = of(room);
                (fit(ctx, &mut self.card, frame), frame)
            }
        };
        ctx.run_layout(&mut self.card, card);
        self.card_rect = Rect::from_origin_size(Self::centered(size, anchor).origin(), card);
        let fullscreen = self.card_rect.width() >= size.width - 1.0 && self.card_rect.height() >= size.height - 1.0;
        if fullscreen && self.intro.running() {
            self.intro.finish();
        }
        ctx.place_child(&mut self.card, self.card_rect.origin());
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let window = ctx.content_box();
        let fullscreen = self.card_rect.width() >= window.width() - 1.0 && self.card_rect.height() >= window.height() - 1.0;
        match self.id.filter(|_| self.intro.running() || self.closing) {
            Some(id) => {
                let v = if self.closing { self.intro.reversed() } else { self.intro.value() };
                let center = self.card_rect.center().to_vec2();
                let scale = 1.0 - SHRINK * (1.0 - v);
                let transform = Affine::translate((0.0, LIFT * (1.0 - v))) * Affine::translate(center) * Affine::scale(scale) * Affine::translate(-center);
                let bounds = self.card_rect.inflate(shadow_reach(t.dialog_shadow), shadow_reach(t.dialog_shadow));
                crate::render::fade::publish_fade(id, Fade { backdrop: t.dialog_overlay_bg.multiply_alpha(v as f32), bounds, transform, alpha: v as f32 });
            }
            None => {
                if let Some(id) = self.id {
                    crate::render::fade::publish_covers(id, fullscreen);
                }
                if !fullscreen {
                    painter.fill(window, t.dialog_overlay_bg).draw();
                }
            }
        }
        if fullscreen {
            painter.fill(self.card_rect, t.dialog_bg).draw();
        } else {
            let shape = RoundedRect::from_rect(self.card_rect, size::DIALOG_RADIUS);
            box_shadow(painter, shape, t.dialog_shadow);
            painter.fill(shape, t.dialog_bg).draw();
            let border = RoundedRect::from_rect(self.card_rect.inflate(-0.5, -0.5), size::DIALOG_RADIUS - 0.5);
            painter.stroke(&border, &Stroke::new(size::BORDER_1), t.dialog_border).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        self.role
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.title.clone());
        node.set_modal();
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.card.id()])
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for Modal {
    fn capture_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let PointerEvent::Down(e) = event
            && !self.closing
        {
            let position = ctx.local_position(e.state.position);
            if !self.card_rect.contains(position) {
                ctx.submit_action::<DialogAnswer>(DialogAnswer::Cancel);
            }
        }
    }
}
