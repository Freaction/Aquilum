use masonry::imaging::Painter;
use masonry::kurbo::{Affine, BezPath, Circle, Point, Rect, RoundedRect, Stroke, Vec2};
use masonry::parley::Alignment;
use masonry::peniko::Color;

use super::{BUTTON, HANDLE, MediaHit, MediaOverlay, MediaView, PLAY};
use crate::ui::icons::{self, Icon};
use crate::ui::menu::box_shadow;
use crate::ui::theme;

const RADIUS: f64 = 8.0;
const BAR_RADIUS: f64 = 8.0;
const BUTTON_RADIUS: f64 = 6.0;
const ICON: f64 = 16.0;
const ICON_STROKE: f64 = 1.6;
const FOCUS: f64 = 1.0;
const RING: f64 = 4.0;

fn draw_image(painter: &mut Painter<'_>, image: &masonry::peniko::ImageBrush, frame: Rect, crop: [f64; 4]) {
    let [l, t, r, b] = crop;
    let (iw, ih) = (f64::from(image.image.width), f64::from(image.image.height));
    let (w, h) = (iw * (r - l) / 100.0, ih * (b - t) / 100.0);
    let k = (frame.width() / w).max(frame.height() / h);
    let x = frame.x0 + (frame.width() - w * k) / 2.0 - iw * l / 100.0 * k;
    let y = frame.y0 + (frame.height() - h * k) / 2.0 - ih * t / 100.0 * k;
    painter.with_fill_clip(RoundedRect::from_rect(frame, RADIUS), |painter| painter.draw_image(image, Affine::translate((x, y)) * Affine::scale(k)));
}

fn icon_for(view: &MediaView, hit: MediaHit) -> Icon {
    match hit {
        MediaHit::Zoom => icons::ZOOM_IN,
        MediaHit::Crop => icons::CROP,
        MediaHit::Source => icons::CODE_XML,
        _ => match view.line.params.align {
            Alignment::Start | Alignment::Left => icons::ALIGN_LEFT,
            Alignment::End | Alignment::Right => icons::ALIGN_RIGHT,
            _ => icons::ALIGN_CENTER,
        },
    }
}

impl MediaView {
    pub fn paint(&self, painter: &mut Painter<'_>, x: f64, y: f64, overlay: &MediaOverlay) {
        if self.frame.area() <= 0.0 {
            return;
        }
        let tm = theme::current();
        let at = Vec2::new(x, y);
        let frame = self.frame + at;
        let playing = overlay.video.as_ref().and_then(|v| v.frame.as_ref());
        match playing.or(self.image.as_ref()) {
            Some(image) => draw_image(painter, image, frame, self.crop()),
            None => painter.fill(RoundedRect::from_rect(frame, RADIUS), Color::BLACK).draw(),
        }
        if let Some(state) = overlay.video.as_ref().filter(|s| overlay.hovered || !s.playing) {
            painter.with_fill_clip(RoundedRect::from_rect(frame, RADIUS), |painter| self.paint_controls(painter, at, state));
        }
        if self.video && overlay.crop.is_none() && overlay.video.is_none() {
            let c = frame.center();
            painter.fill(Circle::new(c, PLAY / 2.0), Color::BLACK.with_alpha(0.5)).draw();
            let mut triangle = BezPath::new();
            triangle.move_to((c.x - PLAY * 0.14, c.y - PLAY * 0.2));
            triangle.line_to((c.x + PLAY * 0.22, c.y));
            triangle.line_to((c.x - PLAY * 0.14, c.y + PLAY * 0.2));
            triangle.close_path();
            painter.fill(&triangle, Color::WHITE).draw();
        }
        if overlay.selected {
            painter.stroke(RoundedRect::from_rect(frame.inflate(RING / 2.0, RING / 2.0), RADIUS + RING / 2.0), &Stroke::new(RING), tm.border_focus_ring).draw();
            painter.stroke(RoundedRect::from_rect(frame.inset(-FOCUS / 2.0), RADIUS), &Stroke::new(FOCUS), tm.border_focus).draw();
        }
        if let Some(crop) = overlay.crop {
            let inner = self.crop_rect(crop) + at;
            painter.with_fill_clip(RoundedRect::from_rect(frame, RADIUS), |painter| {
                for r in [
                    Rect::new(frame.x0, frame.y0, frame.x1, inner.y0),
                    Rect::new(frame.x0, inner.y1, frame.x1, frame.y1),
                    Rect::new(frame.x0, inner.y0, inner.x0, inner.y1),
                    Rect::new(inner.x1, inner.y0, frame.x1, inner.y1),
                ] {
                    painter.fill(r, tm.bg_overlay).draw();
                }
                painter.stroke(inner.inset(-0.5), &Stroke::new(1.0), tm.border_contrast_inverse).draw();
            });
            for h in self.handles(crop) {
                painter.fill(Circle::new(h + at, HANDLE / 2.0), tm.icon_accent).draw();
            }
        }
        if overlay.hovered || overlay.crop.is_some() {
            let bar = RoundedRect::from_rect(self.toolbar() + at, BAR_RADIUS);
            box_shadow(painter, bar, tm.shadow_action);
            painter.fill(bar, tm.bg_scrim).draw();
            painter.stroke(RoundedRect::from_rect((self.toolbar() + at).inset(-0.5), BAR_RADIUS), &Stroke::new(1.0), tm.border_contrast_inverse).draw();
            for (button, rect) in self.toolbar_items() {
                let rect = rect + at;
                match button {
                    None => painter.fill(RoundedRect::from_rect(rect, 2.0), tm.icon_quiet).draw(),
                    Some(hit) => {
                        let active = overlay.hover == Some(hit) || (hit == MediaHit::Crop && overlay.crop.is_some());
                        if active {
                            painter.fill(RoundedRect::from_rect(rect, BUTTON_RADIUS), tm.bg_surface_overlay).draw();
                        }
                        let color = if active { tm.icon_primary } else { tm.icon_secondary };
                        let origin = Point::new(rect.x0 + (BUTTON - ICON) / 2.0, rect.y0 + (BUTTON - ICON) / 2.0);
                        icon_for(self, hit).draw(painter, origin, ICON, ICON_STROKE, color);
                    }
                }
            }
            let grip = RoundedRect::from_rect(self.grip() + at, BUTTON_RADIUS);
            box_shadow(painter, grip, tm.shadow_action);
            painter.fill(grip, tm.bg_scrim).draw();
            if overlay.hover == Some(MediaHit::Grip) {
                painter.fill(grip, tm.bg_surface_overlay).draw();
            }
            let g = self.grip() + at;
            icons::MOVE_DIAGONAL.draw(painter, Point::new(g.x0 + (BUTTON - ICON) / 2.0, g.y0 + (BUTTON - ICON) / 2.0), ICON, ICON_STROKE, tm.icon_muted);
        }
    }
}
