mod controls;
mod markdown;
mod paint;

use std::path::PathBuf;

use masonry::kurbo::{Point, Rect};
use masonry::parley::Alignment;
use masonry::peniko::ImageBrush;

pub use markdown::{EmbedLine, format, is_video, parse};

use super::super::resolve::{Fetch, Media};
use super::{IMAGE_PAD_X, IMAGE_PAD_Y};

const BUTTON: f64 = 24.0;
const BAR_PAD: f64 = 3.0;
const BAR_GAP: f64 = 4.0;
const SEPARATOR: f64 = 1.0;
const INSET: f64 = 8.0;
const PLAY: f64 = 56.0;
const HANDLE: f64 = 10.0;
const HANDLE_HIT: f64 = 8.0;
pub const MIN_WIDTH: f64 = 96.0;
const MIN_HEIGHT: f64 = 48.0;
const VIDEO_MIN_HEIGHT: f64 = 120.0;
pub const MIN_CROP: f64 = 5.0;
const FULL: [f64; 4] = [0.0, 0.0, 100.0, 100.0];

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
}

pub struct MediaView {
    pub height: f64,
    pub line: EmbedLine,
    pub image: Option<ImageBrush>,
    pub video: bool,
    pub path: Option<PathBuf>,
    pub frame: Rect,
    pub room: f64,
    uncropped: bool,
}

impl MediaView {
    pub fn new(fetched: &Fetch<Media>, line: EmbedLine, available: f64, uncropped: bool) -> MediaView {
        let room = (available - IMAGE_PAD_X * 2.0).max(1.0);
        let (image, video, path) = match fetched {
            Fetch::Ready(media) => (media.image.clone(), media.video, media.path.clone()),
            Fetch::Pending if is_video(&line.src) => (None, true, None),
            _ => return MediaView { height: IMAGE_PAD_Y * 2.0, line, image: None, video: false, path: None, frame: Rect::ZERO, room, uncropped },
        };
        let crop = if uncropped { FULL } else { line.params.crop.unwrap_or(FULL) };
        let [l, t, r, b] = crop;
        let (sw, sh) = ((r - l) / 100.0, (b - t) / 100.0);
        let aspect = image.as_ref().map_or(16.0 / 9.0, |i| f64::from(i.image.width) * sw / (f64::from(i.image.height) * sh).max(1.0));
        let natural = image.as_ref().map_or(room, |i| f64::from(i.image.width) * sw);
        let w = line.params.width.unwrap_or(if video { room } else { natural }).min(room);
        let h = w / aspect;
        let x = IMAGE_PAD_X
            + match line.params.align {
                Alignment::Start | Alignment::Left => 0.0,
                Alignment::End | Alignment::Right => room - w,
                _ => (room - w) / 2.0,
            };
        let frame = Rect::new(x, IMAGE_PAD_Y, x + w, IMAGE_PAD_Y + h);
        MediaView { height: h + IMAGE_PAD_Y * 2.0, line, image, video, path, frame, room, uncropped }
    }

    pub fn crop(&self) -> [f64; 4] {
        if self.uncropped { FULL } else { self.line.params.crop.unwrap_or(FULL) }
    }

    pub fn aspect(&self) -> f64 {
        self.frame.width() / self.frame.height().max(1.0)
    }

    pub fn min_width(&self) -> f64 {
        MIN_WIDTH.max(self.toolbar_width() + INSET * 2.0).max(((if self.video { VIDEO_MIN_HEIGHT } else { MIN_HEIGHT }) * self.aspect()).round())
    }

    fn buttons(&self) -> Vec<Option<MediaHit>> {
        let mut out = Vec::new();
        if !self.video {
            out.extend([Some(MediaHit::Zoom), None]);
        }
        out.extend([Some(MediaHit::Align), None, Some(MediaHit::Crop), None, Some(MediaHit::Source)]);
        out
    }

    fn toolbar_width(&self) -> f64 {
        let items = self.buttons();
        let widths: f64 = items.iter().map(|b| if b.is_some() { BUTTON } else { SEPARATOR }).sum();
        widths + BAR_GAP * (items.len() - 1) as f64 + BAR_PAD * 2.0
    }

    pub fn toolbar(&self) -> Rect {
        let w = self.toolbar_width();
        Rect::new(self.frame.x1 - INSET - w, self.frame.y0 + INSET, self.frame.x1 - INSET, self.frame.y0 + INSET + BUTTON + BAR_PAD * 2.0)
    }

    pub fn toolbar_items(&self) -> Vec<(Option<MediaHit>, Rect)> {
        let bar = self.toolbar();
        let mut x = bar.x0 + BAR_PAD;
        let y = bar.y0 + BAR_PAD;
        self.buttons()
            .into_iter()
            .map(|b| {
                let w = if b.is_some() { BUTTON } else { SEPARATOR };
                let rect = if b.is_some() { Rect::new(x, y, x + w, y + BUTTON) } else { Rect::new(x, bar.center().y - 8.0, x + w, bar.center().y + 8.0) };
                x += w + BAR_GAP;
                (b, rect)
            })
            .collect()
    }

    pub fn grip(&self) -> Rect {
        let bottom = if self.video { self.controls().y0 } else { self.frame.y1 } - INSET;
        Rect::new(self.frame.x1 - INSET - BUTTON, bottom - BUTTON, self.frame.x1 - INSET, bottom)
    }

    pub fn crop_rect(&self, crop: [f64; 4]) -> Rect {
        let f = self.frame;
        Rect::new(f.x0 + f.width() * crop[0] / 100.0, f.y0 + f.height() * crop[1] / 100.0, f.x0 + f.width() * crop[2] / 100.0, f.y0 + f.height() * crop[3] / 100.0)
    }

    pub fn handles(&self, crop: [f64; 4]) -> [Point; 8] {
        let r = self.crop_rect(crop);
        let (cx, cy) = (r.center().x, r.center().y);
        [
            Point::new(r.x0, r.y0),
            Point::new(cx, r.y0),
            Point::new(r.x1, r.y0),
            Point::new(r.x1, cy),
            Point::new(r.x1, r.y1),
            Point::new(cx, r.y1),
            Point::new(r.x0, r.y1),
            Point::new(r.x0, cy),
        ]
    }

    pub fn hit(&self, p: Point, overlay: &MediaOverlay) -> MediaHit {
        if self.frame.area() <= 0.0 {
            return MediaHit::Outside;
        }
        if let Some(crop) = overlay.crop
            && let Some(i) = self.handles(crop).iter().position(|h| (*h - p).hypot() <= HANDLE_HIT)
        {
            return MediaHit::Handle(i as u8);
        }
        if let Some(state) = &overlay.video
            && (overlay.hovered || !state.playing)
            && let Some(hit) = self.control_hit(p)
        {
            return hit;
        }
        if overlay.hovered || overlay.crop.is_some() {
            if let Some((Some(b), _)) = self.toolbar_items().into_iter().find(|(b, r)| b.is_some() && r.contains(p)) {
                return b;
            }
            if self.grip().contains(p) {
                return MediaHit::Grip;
            }
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
