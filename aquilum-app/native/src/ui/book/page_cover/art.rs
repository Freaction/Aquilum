//! The page cover keeps its last complete frame while a new pattern size is baking.
use super::*;

#[derive(PartialEq)]
struct Key {
    size: Size,
    scale: f64,
    pattern: Option<cover::Key>,
    theme: usize,
    source: u64,
    position: f64,
}

#[derive(Default)]
pub(super) struct Art {
    frame: Option<(Key, ImageBrush)>,
}

impl Art {
    pub(super) fn paint(&mut self, painter: &mut Painter<'_>, bounds: Rect, scale: f64, request: Option<cover::Key>, visual: &Visual, position: f64, repositioning: bool, pattern: impl FnOnce() -> Option<cover::Pattern>) {
        let source = match visual {
            Visual::Pattern(id) => id.as_ptr() as u64,
            Visual::Image(image) => image.image.data.id(),
        };
        let key = Key { size: bounds.size(), scale, pattern: request, theme: theme_id(), source, position };
        if !repositioning && self.frame.as_ref().is_some_and(|(old, _)| *old == key) {
            self.draw(painter, bounds, scale, true);
            return;
        }
        let pattern = pattern();
        let exact = matches!(visual, Visual::Image(_)) || pattern.as_ref().is_some_and(|p| p.exact);
        if !repositioning && exact {
            let local = Rect::from_origin_size(Point::ORIGIN, bounds.size());
            if let Some(image) = bake(bounds.size(), scale, |painter| body(painter, local, visual, position, pattern.as_ref().map(|p| &p.image), scale)) {
                self.frame = Some((key, image.with_quality(ImageQuality::Low)));
                self.draw(painter, bounds, scale, true);
                return;
            }
        }
        if !repositioning && self.frame.as_ref().is_some_and(|(old, _)| old.source == source) {
            self.draw(painter, bounds, scale, false);
        } else {
            body(painter, bounds, visual, position, pattern.as_ref().map(|p| &p.image), scale);
        }
    }

    fn draw(&self, painter: &mut Painter<'_>, bounds: Rect, scale: f64, exact: bool) {
        let Some((_, image)) = &self.frame else { return };
        let transform = Affine::translate(bounds.origin().to_vec2());
        if exact {
            painter.draw_image(image, transform * Affine::scale(1.0 / scale));
        } else {
            // Fill the current bounds immediately; never replace a ready cover with a blank.
            let fit = Affine::scale_non_uniform(bounds.width() / f64::from(image.image.width), bounds.height() / f64::from(image.image.height));
            painter.with_fill_clip(bounds.to_rounded_rect(size::BOOK_COVER_RADIUS), |painter| painter.draw_image(image, transform * fit));
        }
    }
}

pub(super) fn body(painter: &mut Painter<'_>, bounds: Rect, visual: &Visual, position: f64, pattern: Option<&ImageBrush>, scale: f64) {
    let tm = theme::current();
    let shape = RoundedRect::from_rect(bounds, size::BOOK_COVER_RADIUS);
    painter.with_fill_clip(shape, |painter| {
        painter.fill(bounds, tm.bg_sunken).draw();
        match visual {
            Visual::Pattern(id) => {
                painter.fill(bounds, cover::base_color(id)).draw();
                if let Some(image) = pattern {
                    painter.fill(bounds, image).brush_transform(Some(Affine::translate(bounds.origin().to_vec2()) * Affine::scale(1.0 / scale))).draw();
                }
            }
            Visual::Image(image) => {
                let (w, h) = (f64::from(image.image.width), f64::from(image.image.height));
                let k = (bounds.width() / w).max(bounds.height() / h);
                let (dw, dh) = (w * k, h * k);
                let x = bounds.x0 + (bounds.width() - dw) / 2.0;
                let y = bounds.y0 + (bounds.height() - dh) * position / 100.0;
                painter.draw_image(image, Affine::translate((x, y)) * Affine::scale(k));
            }
        }
        if !theme::is_dark() {
            paint_rays(painter, Rect::new(bounds.x1 - RAYS_WIDTH, bounds.y0, bounds.x1, bounds.y1), scale);
        }
    });
}
