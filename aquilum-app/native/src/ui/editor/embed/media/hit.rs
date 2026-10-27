use masonry::kurbo::Point;

use super::{HANDLE_HIT, MIN_CROP, MediaView, PLAY};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaHit {
    Zoom,
    Align,
    Crop,
    Source,
    Grip,
    Play,
    Image,
    Handle(u8),
    CropBox,
    Toggle,
    Mute,
    Bar,
    Outside,
}

#[derive(Clone, Debug, Default)]
pub struct MediaOverlay {
    pub hovered: bool,
    pub selected: bool,
    pub hover: Option<MediaHit>,
    pub crop: Option<[f64; 4]>,
    pub video: Option<crate::video::State>,
    pub fade: f32,
}

impl MediaView {
    pub fn hit(&self, p: Point, overlay: &MediaOverlay, reach: f64) -> MediaHit {
        if self.frame.area() <= 0.0 {
            return MediaHit::Outside;
        }
        if let Some(crop) = overlay.crop
            && let Some(i) = self.handles(crop).iter().position(|h| (*h - p).hypot() <= HANDLE_HIT + reach)
        {
            return MediaHit::Handle(i as u8);
        }
        if (overlay.hovered || overlay.crop.is_some()) && self.grip().inflate(reach, reach).contains(p) {
            return MediaHit::Grip;
        }
        if let Some(state) = &overlay.video
            && (overlay.hovered || !state.playing)
            && let Some(hit) = self.control_hit(p)
        {
            return hit;
        }
        if (overlay.hovered || overlay.crop.is_some())
            && let Some((Some(b), _)) = self.toolbar_items().into_iter().find(|(b, r)| b.is_some() && r.contains(p))
        {
            return b;
        }
        if !self.frame.contains(p) {
            return MediaHit::Outside;
        }
        if overlay.crop.is_some() {
            return MediaHit::CropBox;
        }
        if overlay.video.is_some() {
            return MediaHit::Toggle;
        }
        if self.video && (p - self.frame.center()).hypot() <= PLAY / 2.0 {
            return MediaHit::Play;
        }
        MediaHit::Image
    }

    pub fn drag_crop(&self, start: [f64; 4], handle: Option<u8>, dx: f64, dy: f64) -> [f64; 4] {
        let (px, py) = (dx / self.frame.width().max(1.0) * 100.0, dy / self.frame.height().max(1.0) * 100.0);
        let [mut l, mut t, mut r, mut b] = start;
        let round = |v: f64| (v * 10.0).round() / 10.0;
        match handle {
            None => {
                let (w, h) = (r - l, b - t);
                l = (l + px).clamp(0.0, 100.0 - w);
                t = (t + py).clamp(0.0, 100.0 - h);
                r = l + w;
                b = t + h;
            }
            Some(i) => {
                if matches!(i, 0 | 6 | 7) {
                    l = (l + px).clamp(0.0, r - MIN_CROP);
                }
                if matches!(i, 2 | 3 | 4) {
                    r = (r + px).clamp(l + MIN_CROP, 100.0);
                }
                if matches!(i, 0 | 1 | 2) {
                    t = (t + py).clamp(0.0, b - MIN_CROP);
                }
                if matches!(i, 4 | 5 | 6) {
                    b = (b + py).clamp(t + MIN_CROP, 100.0);
                }
            }
        }
        [round(l), round(t), round(r), round(b)]
    }
}
