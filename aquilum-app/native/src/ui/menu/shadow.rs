use masonry::imaging::{BlurredRoundedRect, Composite, PaintSink, Painter};
use masonry::kurbo::{Affine, RoundedRect, Stroke, Vec2};

use crate::ui::tokens::Shadow;

pub fn shadow_reach(shadows: &[Shadow]) -> f64 {
    shadows.iter().map(|s| s.spread + s.blur * 1.5 + s.x.abs().max(s.y.abs())).fold(0.0, f64::max).ceil()
}

pub fn box_shadow<S: PaintSink + ?Sized>(painter: &mut Painter<'_, S>, shape: RoundedRect, shadows: &[Shadow]) {
    for shadow in shadows {
        if shadow.blur <= 0.0 && shadow.x == 0.0 && shadow.y == 0.0 {
            if shadow.spread > 0.0 {
                let half = shadow.spread / 2.0;
                let ring = RoundedRect::from_rect(shape.rect().inflate(half, half), shape.radii().top_left + half);
                painter.stroke(&ring, &Stroke::new(shadow.spread), shadow.color).draw();
            }
            continue;
        }
        let rect = shape.rect().inflate(shadow.spread, shadow.spread) + Vec2::new(shadow.x, shadow.y);
        painter.blurred_rounded_rect(BlurredRoundedRect {
            transform: Affine::IDENTITY,
            rect,
            color: shadow.color,
            radius: shape.radii().top_left,
            std_dev: shadow.blur / 2.0,
            composite: Composite::default(),
        });
    }
}
