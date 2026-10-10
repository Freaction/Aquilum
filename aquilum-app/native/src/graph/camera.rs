use super::Bounds;

const MIN_SCALE: f64 = 0.02;
const MAX_SCALE: f64 = 400.0;
const RESPONSE_SECONDS: f64 = 0.08;
const SETTLED_ZOOM: f64 = 0.0015;
const SETTLED_PIXELS: f64 = 0.05;
const LONGEST_FRAME_SECONDS: f64 = 0.1;

pub fn approach(seconds: f64, constant: f64) -> f64 {
    1.0 - (-seconds.min(LONGEST_FRAME_SECONDS) / constant).exp()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Anchor {
    offset_x: f64,
    offset_y: f64,
    world_x: f64,
    world_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub center_x: f64,
    pub center_y: f64,
    pub scale: f64,
    target_scale: f64,
    anchor: Option<Anchor>,
    target: Option<(f64, f64)>,
}

impl Default for Camera {
    fn default() -> Self {
        Camera { center_x: 0.0, center_y: 0.0, scale: 1.0, target_scale: 1.0, anchor: None, target: None }
    }
}

fn fit(bounds: Bounds, width: f64, height: f64) -> (f64, f64, f64) {
    let scale = (width / (bounds.max_x - bounds.min_x).max(1e-3)).min(height / (bounds.max_y - bounds.min_y).max(1e-3));
    ((bounds.min_x + bounds.max_x) / 2.0, (bounds.min_y + bounds.max_y) / 2.0, scale.clamp(MIN_SCALE, MAX_SCALE))
}

impl Camera {
    pub fn zoom_by(&mut self, factor: f64, offset_x: f64, offset_y: f64) {
        let (world_x, world_y) = self.to_world(offset_x, offset_y);
        self.anchor = Some(Anchor { offset_x, offset_y, world_x, world_y });
        self.target = None;
        self.target_scale = (self.target_scale * factor).clamp(MIN_SCALE, MAX_SCALE);
    }

    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        self.target = None;
        let (wx, wy) = (dx / self.scale, dy / self.scale);
        self.center_x -= wx;
        self.center_y += wy;
        if let Some(anchor) = &mut self.anchor {
            anchor.world_x -= wx;
            anchor.world_y += wy;
        }
    }

    pub fn rescale_world(&mut self, factor: f64) {
        self.center_x *= factor;
        self.center_y *= factor;
        if let Some(anchor) = &mut self.anchor {
            anchor.world_x *= factor;
            anchor.world_y *= factor;
        }
        if let Some((x, y)) = &mut self.target {
            *x *= factor;
            *y *= factor;
        }
    }

    pub fn move_to(&mut self, center_x: f64, center_y: f64, scale: f64) {
        self.center_x = center_x;
        self.center_y = center_y;
        self.scale = scale.clamp(MIN_SCALE, MAX_SCALE);
        self.target_scale = self.scale;
        self.anchor = None;
        self.target = None;
    }

    pub fn jump_to(&mut self, bounds: Bounds, width: f64, height: f64) {
        let (x, y, scale) = fit(bounds, width, height);
        self.move_to(x, y, scale);
    }

    pub fn glide_to(&mut self, bounds: Bounds, width: f64, height: f64) {
        let (x, y, scale) = fit(bounds, width, height);
        self.target_scale = scale;
        self.target = Some((x, y));
        self.anchor = None;
    }

    pub fn advance(&mut self, seconds: f64) -> bool {
        let step = approach(seconds, RESPONSE_SECONDS);
        let zooming = self.advance_scale(step);
        let gliding = self.advance_center(step);
        zooming || gliding
    }

    pub fn to_world(&self, offset_x: f64, offset_y: f64) -> (f64, f64) {
        (self.center_x + offset_x / self.scale, self.center_y - offset_y / self.scale)
    }

    fn advance_scale(&mut self, step: f64) -> bool {
        let gap = self.target_scale / self.scale;
        if gap.ln().abs() < SETTLED_ZOOM {
            let settled = self.scale != self.target_scale;
            self.scale = self.target_scale;
            self.hold_anchor();
            self.anchor = None;
            return settled;
        }
        self.scale *= gap.powf(step);
        self.hold_anchor();
        true
    }

    fn advance_center(&mut self, step: f64) -> bool {
        let Some((x, y)) = self.target else { return false };
        let (dx, dy) = (x - self.center_x, y - self.center_y);
        if dx.hypot(dy) * self.scale < SETTLED_PIXELS {
            self.center_x = x;
            self.center_y = y;
            self.target = None;
            return false;
        }
        self.center_x += dx * step;
        self.center_y += dy * step;
        true
    }

    fn hold_anchor(&mut self) {
        let Some(a) = self.anchor else { return };
        self.center_x = a.world_x - a.offset_x / self.scale;
        self.center_y = a.world_y + a.offset_y / self.scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_the_point_under_the_cursor() {
        let mut camera = Camera::default();
        let before = camera.to_world(100.0, -50.0);
        camera.zoom_by(4.0, 100.0, -50.0);
        for _ in 0..200 {
            camera.advance(1.0 / 60.0);
        }
        assert!((camera.scale - 4.0).abs() < 1e-9);
        let after = camera.to_world(100.0, -50.0);
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
    }

    #[test]
    fn glides_to_fit_bounds() {
        let mut camera = Camera::default();
        camera.glide_to(Bounds { min_x: 10.0, min_y: 0.0, max_x: 20.0, max_y: 5.0 }, 1000.0, 1000.0);
        while camera.advance(1.0 / 60.0) {}
        assert!((camera.center_x - 15.0).abs() < 0.01 && (camera.center_y - 2.5).abs() < 0.01);
        assert!((camera.scale - 100.0).abs() < 1e-9);
    }
}
