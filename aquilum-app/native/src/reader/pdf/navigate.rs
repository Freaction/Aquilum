use masonry::kurbo::{Point, Vec2};

use super::{MAX_ZOOM, MIN_ZOOM, Session};

impl Session {
    pub fn turn(&mut self, forward: bool) -> bool {
        if self.sizes.is_empty() {
            return false;
        }
        let page = if forward { (self.position.page + 1).min(self.sizes.len() - 1) } else { self.position.page.saturating_sub(1) };
        if page == self.position.page {
            return false;
        }
        self.position.page = page;
        self.position.x = 0.0;
        self.position.y = 0.0;
        self.frame = None;
        self.clamp();
        self.request();
        true
    }

    pub fn pan(&mut self, delta: Vec2) -> bool {
        if !delta.x.is_finite() || !delta.y.is_finite() {
            return false;
        }
        let before = self.position;
        let scale = self.scale();
        self.position.x -= delta.x / scale;
        self.position.y -= delta.y / scale;
        self.clamp();
        let changed = before.x != self.position.x || before.y != self.position.y;
        if changed {
            self.request();
        }
        changed
    }

    pub fn zoom(&mut self, factor: f64, at: Point) -> bool {
        if self.sizes.is_empty() || !factor.is_finite() || factor <= 0.0 {
            return false;
        }
        let before = self.view();
        let scale = (before.scale * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if scale == before.scale {
            return false;
        }
        let anchor = before.transform().inverse() * at;
        self.position.fit = false;
        self.position.zoom = scale;
        let centered = self.view();
        self.position.x += (centered.transform() * anchor - at).x / scale;
        self.position.y += (centered.transform() * anchor - at).y / scale;
        self.clamp();
        self.request();
        true
    }

    pub fn fit(&mut self, whole: bool) {
        let Some(page) = self.sizes.get(self.position.page) else { return };
        self.position.fit = !whole;
        self.position.zoom = (self.size.width / page.width).min(self.size.height / page.height).clamp(MIN_ZOOM, MAX_ZOOM);
        self.position.x = 0.0;
        self.position.y = 0.0;
        self.clamp();
        self.request();
    }
}
