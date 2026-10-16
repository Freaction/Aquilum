use std::path::PathBuf;

use masonry::kurbo::{Point, Rect};
use masonry::parley::Alignment;
use masonry::peniko::ImageBrush;

use super::super::metrics::{IMAGE_PAD_X, IMAGE_PAD_Y};
use super::{BAR_GAP, BAR_PAD, BUTTON, EmbedLine, FULL, INSET, MIN_HEIGHT, MIN_WIDTH, MediaHit, SEPARATOR, VIDEO_MIN_HEIGHT};
use crate::ui::editor::resolve::{Fetch, Media};

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
        let (image, natural, video, path) = match fetched {
            Fetch::Ready(media) => (media.image.clone(), media.natural, media.video, media.path.clone()),
            Fetch::Pending if crate::images::kind(&line.src) == Some(crate::images::MediaKind::Video) => (None, (0, 0), true, None),
            _ => return MediaView { height: IMAGE_PAD_Y * 2.0, line, image: None, video: false, path: None, frame: Rect::ZERO, room, uncropped },
        };
        let crop = if uncropped { FULL } else { line.params.crop.unwrap_or(FULL) };
        let [l, t, r, b] = crop;
        let (sw, sh) = ((r - l) / 100.0, (b - t) / 100.0);
        let (nw, nh) = (f64::from(natural.0), f64::from(natural.1));
        let aspect = if image.is_some() { nw * sw / (nh * sh).max(1.0) } else { 16.0 / 9.0 };
        let natural = if image.is_some() { nw * sw } else { room };
        let w = line.params.width.unwrap_or(if video { room } else { natural }).min(room);
        let h = line.params.height.zip(line.params.width).map_or(w / aspect, |(height, width)| height * w / width);
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
        let bottom = self.frame.y1 - INSET;
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
}
