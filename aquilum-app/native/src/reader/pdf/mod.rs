//! Fixed-layout PDF session. Only viewport pixels cross the worker boundary.
#[cfg(test)]
pub(crate) mod tests;
mod worker;

use std::path::Path;
use std::sync::Arc;

use masonry::kurbo::{Affine, Point, Rect, Size, Vec2};
use masonry::peniko::ImageBrush;
use serde::{Deserialize, Serialize};

use super::session::Target;
use worker::{Frame, Worker};

pub fn is_pdf(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub page: usize,
    pub scale: f64,
    pub origin: Vec2,
    pub size: Size,
    pub dpi: f64,
}

impl View {
    pub fn transform(self) -> Affine {
        Affine::translate(self.origin) * Affine::scale(self.scale)
    }

    pub fn pixels(self) -> (u16, u16) {
        ((self.size.width * self.dpi).ceil().clamp(1.0, 8192.0) as u16, (self.size.height * self.dpi).ceil().clamp(1.0, 8192.0) as u16)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Position {
    page: usize,
    zoom: f64,
    x: f64,
    y: f64,
    fit: bool,
}

pub struct Session {
    worker: Worker,
    sizes: Vec<Size>,
    position: Position,
    initial: Option<Target>,
    size: Size,
    dpi: f64,
    requested: Option<View>,
    frame: Option<Frame>,
    pub error: Option<String>,
}

impl Session {
    #[cfg(test)]
    pub fn current_frame(&self) -> bool {
        self.frame.as_ref().is_some_and(|f| Some(f.view) == self.requested)
    }

    pub fn open(path: &Path, target: Target, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Session {
            worker: Worker::new(path.to_path_buf(), wake),
            sizes: Vec::new(),
            position: Position { page: 0, zoom: 1.0, x: 0.0, y: 0.0, fit: true },
            initial: Some(target),
            size: Size::ZERO,
            dpi: 1.0,
            requested: None,
            frame: None,
            error: None,
        }
    }

    pub fn total(&self) -> u32 {
        self.sizes.len() as u32
    }
    pub fn current(&self) -> u32 {
        self.position.page as u32 + 1
    }
    pub fn scale(&self) -> f64 {
        let width = self.sizes.get(self.position.page).map_or(1.0, |s| s.width);
        if self.position.fit { (self.size.width / width).max(0.001) } else { self.position.zoom }
    }

    pub fn location(&self) -> Option<(String, f64)> {
        if self.sizes.is_empty() {
            return None;
        }
        let position = serde_json::to_string(&self.position).ok()?;
        Some((format!("pdf:{position}"), f64::from(self.current()) / f64::from(self.total())))
    }

    pub fn poll(&mut self) -> bool {
        let inbox = self.worker.take();
        let changed = inbox.opened.is_some() || inbox.frame.is_some() || inbox.error.is_some();
        if let Some(sizes) = inbox.opened {
            self.sizes = sizes;
            match self.initial.take() {
                Some(Target::Cfi(text)) => {
                    if let Some(position) = text.strip_prefix("pdf:").and_then(|s| serde_json::from_str::<Position>(s).ok())
                        && position.zoom.is_finite()
                        && position.x.is_finite()
                        && position.y.is_finite()
                    {
                        self.position = Position { page: position.page.min(self.sizes.len() - 1), zoom: position.zoom.clamp(0.05, 16.0), ..position };
                    }
                }
                Some(Target::Fraction(f)) if f.is_finite() => {
                    self.position.page = ((f.clamp(0.0, 1.0) * f64::from(self.total())).round() as usize).saturating_sub(1).min(self.sizes.len() - 1)
                }
                _ => {}
            }
            self.clamp();
            self.request();
        }
        if let Some(frame) = inbox.frame
            && Some(frame.view) == self.requested
        {
            self.frame = Some(frame);
        }
        if let Some(error) = inbox.error {
            self.error = Some(error);
        }
        changed
    }

    pub fn trim(&mut self) {
        self.frame = None;
        self.requested = None;
        self.worker.trim();
    }

    pub fn ensure(&mut self, size: Size, dpi: f64) {
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        self.size = size;
        // Bound viewport raster memory, including very high DPI desktop windows.
        self.dpi = dpi.min(8192.0 / size.width.max(size.height)).min((4_194_304.0 / (size.width * size.height)).sqrt()).max(0.01);
        self.clamp();
        self.request();
    }

    fn view(&self) -> View {
        let scale = self.scale();
        let page = self.sizes.get(self.position.page).copied().unwrap_or(Size::ZERO);
        let origin = Vec2::new(
            ((self.size.width - page.width * scale) / 2.0).max(0.0) - self.position.x * scale,
            ((self.size.height - page.height * scale) / 2.0).max(0.0) - self.position.y * scale,
        );
        View { page: self.position.page, scale, origin, size: self.size, dpi: self.dpi }
    }

    fn request(&mut self) {
        if self.sizes.is_empty() || self.size.is_zero_area() || self.error.is_some() {
            return;
        }
        let view = self.view();
        if self.requested != Some(view) {
            self.requested = Some(view);
            self.worker.request(view);
        }
    }

    fn clamp(&mut self) {
        let Some(page) = self.sizes.get(self.position.page) else { return };
        let scale = self.scale();
        self.position.x = self.position.x.clamp(0.0, (page.width - self.size.width / scale).max(0.0));
        self.position.y = self.position.y.clamp(0.0, (page.height - self.size.height / scale).max(0.0));
    }

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
        let scale = (before.scale * factor).clamp(0.05, 16.0);
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
        self.position.zoom = (self.size.width / page.width).min(self.size.height / page.height).clamp(0.05, 16.0);
        self.position.x = 0.0;
        self.position.y = 0.0;
        self.clamp();
        self.request();
    }

    pub fn page_rect(&self) -> Rect {
        self.sizes.get(self.position.page).map_or(Rect::ZERO, |s| self.view().transform().transform_rect_bbox(s.to_rect()))
    }

    pub fn image(&self) -> Option<(&ImageBrush, Affine)> {
        let frame = self.frame.as_ref().filter(|f| f.view.page == self.position.page)?;
        Some((&frame.image, self.view().transform() * frame.view.transform().inverse() * Affine::scale(1.0 / frame.view.dpi)))
    }
}
