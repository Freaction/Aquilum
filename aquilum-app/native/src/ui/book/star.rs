use super::*;

const STAR_PATH: &str = "M26.7326 32.142C34.8108 26.881 36.851 0 36.851 0C36.851 0 38.6803 24.7488 46 30C54.5 36.0979 96 38.5621 96 38.5621C96 38.5621 53.7749 39.0195 45.6578 44.4024C37.5407 49.7852 36.851 63 36.851 63C36.851 63 35.1929 49.7069 26.7326 44.4024C18.7456 39.3946 0 38.5621 0 38.5621C0 38.5621 18.8107 37.3012 26.7326 32.142Z";

pub struct Star;

impl Widget for Star {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        let scale = ctx.scale_factor();
        let (dw, dh) = device(bounds, scale);
        let Some(image) = cached(1, u32::from(dw), u32::from(dh), || {
            crate::render::offscreen(dw, dh, |painter| {
                let Ok(star) = BezPath::from_svg(STAR_PATH) else { return };
                let k = f64::from(dw) / 96.0;
                let star = Affine::scale(k) * star;
                painter.fill(&star, Color::WHITE.with_alpha(0.4)).draw();
                let shadows = [(Vec2::new(-1.93416, -1.93416), 1.93416, 0.5), (Vec2::new(1.93416, 11.0), 3.86832, 0.3)];
                let frame = Rect::new(0.0, 0.0, f64::from(dw), f64::from(dh));
                painter.with_fill_clip(&star, |painter| {
                    for (offset, blur, alpha) in shadows {
                        let mut outside = frame.inflate(frame.width(), frame.height()).to_path(0.1);
                        outside.extend(Affine::translate(offset * k) * star.clone());
                        let filters = [Filter::blur((blur * k) as f32)];
                        painter.with_group(GroupRef::new().with_filters(&filters), |painter| {
                            painter.fill(&outside, Color::WHITE.with_alpha(alpha)).fill_rule(Fill::EvenOdd).draw();
                        });
                    }
                });
            })
        }) else {
            return;
        };
        painter.draw_image(&image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale(1.0 / scale));
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
