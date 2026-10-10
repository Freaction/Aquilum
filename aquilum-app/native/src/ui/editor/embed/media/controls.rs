use masonry::imaging::{PaintSink, Painter};
use masonry::kurbo::{BezPath, Cap, Point, Rect, RoundedRect, Stroke, Vec2};
use masonry::peniko::Color;

use super::{MediaHit, MediaView};
use crate::video::State;

const BAR: f64 = 40.0;
const SIDE: f64 = 12.0;
const ICON: f64 = 16.0;
const TRACK: f64 = 4.0;
const KNOB: f64 = 12.0;
const SHADE: f32 = 0.55;

impl MediaView {
    pub fn controls(&self) -> Rect {
        Rect::new(self.frame.x0, self.frame.y1 - BAR, self.frame.x1, self.frame.y1)
    }

    fn toggle_button(&self) -> Rect {
        let c = self.controls();
        Rect::new(c.x0 + SIDE, c.center().y - ICON / 2.0, c.x0 + SIDE + ICON, c.center().y + ICON / 2.0)
    }

    fn mute_button(&self) -> Rect {
        let c = self.controls();
        Rect::new(c.x1 - SIDE - ICON, c.center().y - ICON / 2.0, c.x1 - SIDE, c.center().y + ICON / 2.0)
    }

    pub fn track(&self) -> Rect {
        let c = self.controls();
        Rect::new(self.toggle_button().x1 + SIDE, c.center().y - TRACK / 2.0, self.mute_button().x0 - SIDE, c.center().y + TRACK / 2.0)
    }

    pub fn seek_fraction(&self, p: Point) -> f64 {
        let t = self.track();
        ((p.x - t.x0) / t.width().max(1.0)).clamp(0.0, 1.0)
    }

    pub(super) fn control_hit(&self, p: Point) -> Option<MediaHit> {
        if !self.controls().contains(p) {
            return None;
        }
        if self.toggle_button().inflate(6.0, 6.0).contains(p) {
            return Some(MediaHit::Toggle);
        }
        if self.mute_button().inflate(6.0, 6.0).contains(p) {
            return Some(MediaHit::Mute);
        }
        let t = self.track();
        Some(if p.x >= t.x0 - KNOB && p.x <= t.x1 + KNOB { MediaHit::Bar } else { MediaHit::Toggle })
    }

    pub(super) fn paint_controls(&self, painter: &mut Painter<'_, impl PaintSink + ?Sized>, at: Vec2, state: &State) {
        let c = self.controls() + at;
        painter.fill(Rect::new(c.x0, c.y0, c.x1, c.y1), Color::BLACK.with_alpha(SHADE)).draw();
        let white = Color::WHITE;
        let b = self.toggle_button() + at;
        if state.playing {
            let w = ICON * 0.28;
            painter.fill(RoundedRect::new(b.x0 + 2.0, b.y0 + 1.0, b.x0 + 2.0 + w, b.y1 - 1.0, 1.0), white).draw();
            painter.fill(RoundedRect::new(b.x1 - 2.0 - w, b.y0 + 1.0, b.x1 - 2.0, b.y1 - 1.0, 1.0), white).draw();
        } else {
            let mut path = BezPath::new();
            path.move_to((b.x0 + 3.0, b.y0 + 1.0));
            path.line_to((b.x1 - 1.0, b.center().y));
            path.line_to((b.x0 + 3.0, b.y1 - 1.0));
            path.close_path();
            painter.fill(&path, white).draw();
        }
        let t = self.track() + at;
        painter.fill(RoundedRect::from_rect(t, TRACK / 2.0), white.with_alpha(0.3)).draw();
        let fraction = if state.duration > 0.0 { (state.position / state.duration).clamp(0.0, 1.0) } else { 0.0 };
        let x = t.x0 + t.width() * fraction;
        painter.fill(RoundedRect::new(t.x0, t.y0, x, t.y1, TRACK / 2.0), white).draw();
        painter.fill(masonry::kurbo::Circle::new((x, t.center().y), KNOB / 2.0), white).draw();
        let m = self.mute_button() + at;
        let mut speaker = BezPath::new();
        speaker.move_to((m.x0 + 1.0, m.center().y - 3.0));
        speaker.line_to((m.x0 + 5.0, m.center().y - 3.0));
        speaker.line_to((m.x0 + 9.0, m.y0 + 1.0));
        speaker.line_to((m.x0 + 9.0, m.y1 - 1.0));
        speaker.line_to((m.x0 + 5.0, m.center().y + 3.0));
        speaker.line_to((m.x0 + 1.0, m.center().y + 3.0));
        speaker.close_path();
        painter.fill(&speaker, white).draw();
        let stroke = Stroke::new(1.6).with_caps(Cap::Round);
        let mut waves = BezPath::new();
        if state.muted {
            waves.move_to((m.x1 - 5.0, m.center().y - 3.0));
            waves.line_to((m.x1 - 1.0, m.center().y + 3.0));
            waves.move_to((m.x1 - 1.0, m.center().y - 3.0));
            waves.line_to((m.x1 - 5.0, m.center().y + 3.0));
        } else {
            waves.move_to((m.x0 + 11.5, m.center().y - 3.5));
            waves.quad_to((m.x0 + 14.0, m.center().y), (m.x0 + 11.5, m.center().y + 3.5));
        }
        painter.stroke(&waves, &stroke, white).draw();
    }
}
