use std::time::Instant;

use masonry::kurbo::{Point, Vec2};

use super::scale::Scale;
use crate::fling::{Fling, Trail};
use crate::ui::keymap::gesture::{DOUBLE_TAP, DOUBLE_TAP_SLOP, FLING_MAX_VELOCITY, FLING_MIN_VELOCITY, LONG_PRESS, TAP_TIMEOUT, TOUCH_SLOP};

#[derive(Clone, Copy, Default, PartialEq)]
enum Phase {
    #[default]
    Idle,
    Pressed(Instant, u8),
    Held,
    Scrolling,
    Done,
}

pub struct Frame {
    pub scroll: Option<(Point, Vec2)>,
    pub pinch: Option<(Point, f64)>,
}

#[derive(Default)]
pub struct Touch {
    fingers: Vec<(usize, Point)>,
    phase: Phase,
    dp: f64,
    down: Point,
    focus: Point,
    travel: Vec2,
    moved: Option<Instant>,
    trail: Trail,
    scale: Scale,
    scroll: Vec2,
    zoom: f64,
    fling: Option<Fling>,
    tap: Option<(Instant, Point, u8)>,
    settled: bool,
}

impl Touch {
    pub fn down(&mut self, finger: usize, position: Point, at: Instant, dp: f64) {
        self.fingers.retain(|&(id, _)| id != finger);
        self.fingers.push((finger, position));
        if self.fingers.len() > 1 {
            self.phase = Phase::Scrolling;
        } else {
            self.dp = dp;
            self.down = position;
            self.travel = Vec2::ZERO;
            self.trail = Trail::touch();
            self.moved = Some(at);
            self.settled = false;
            let repeated = self.tap.take().filter(|&(up, from, _)| at.saturating_duration_since(up) <= DOUBLE_TAP && (position - from).hypot() < DOUBLE_TAP_SLOP * dp);
            self.phase = match (self.fling.take(), repeated) {
                (Some(_), _) => Phase::Scrolling,
                (None, Some((_, _, count))) => Phase::Pressed(at, count.saturating_add(1)),
                (None, None) => Phase::Pressed(at, 1),
            };
        }
        self.regroup();
    }

    pub fn moved(&mut self, finger: usize, position: Point, at: Instant) {
        let Some(slot) = self.fingers.iter_mut().find(|(id, _)| *id == finger) else { return };
        slot.1 = position;
        let focus = self.centroid();
        let step = focus - std::mem::replace(&mut self.focus, focus);
        self.travel += step;
        self.moved = Some(at);
        match self.phase {
            Phase::Pressed(..) => {
                let offset = focus - self.down;
                let slop = TOUCH_SLOP * self.dp;
                if offset.hypot() > slop {
                    self.phase = Phase::Scrolling;
                    self.scroll += offset * (1.0 - slop / offset.hypot());
                }
            }
            Phase::Scrolling => {
                self.scroll += step;
                self.zoom += self.scale.moved(self.span(), self.dp);
            }
            Phase::Idle | Phase::Held | Phase::Done => {}
        }
    }

    pub fn up(&mut self, finger: usize, position: Point, at: Instant) -> Option<(Point, u8)> {
        self.moved(finger, position, at);
        if !self.lift(finger) || !self.fingers.is_empty() {
            return None;
        }
        match std::mem::take(&mut self.phase) {
            Phase::Pressed(_, count) => {
                self.tap = Some((at, self.down, count));
                Some((self.down, count))
            }
            Phase::Scrolling => {
                self.fling = self.release(at);
                None
            }
            Phase::Idle | Phase::Held | Phase::Done => None,
        }
    }

    pub fn pressed(&self) -> bool {
        matches!(self.phase, Phase::Pressed(..))
    }

    pub fn hold(&mut self) {
        self.phase = Phase::Held;
    }

    pub fn held(&self) -> bool {
        self.phase == Phase::Held
    }

    pub fn cancel(&mut self, finger: usize) {
        if self.lift(finger) {
            self.phase = if self.fingers.is_empty() { Phase::Idle } else { Phase::Done };
        }
    }

    pub fn settle(&mut self, now: Instant) -> Option<Point> {
        let due = matches!(self.phase, Phase::Pressed(at, _) if !self.settled && now >= at + TAP_TIMEOUT);
        self.settled |= due;
        due.then_some(self.down)
    }

    pub fn long_press(&mut self, now: Instant) -> Option<Point> {
        matches!(self.phase, Phase::Pressed(at, _) if now >= at + LONG_PRESS).then(|| {
            self.phase = Phase::Done;
            self.down
        })
    }

    pub fn deadline(&self) -> Option<Instant> {
        match self.phase {
            Phase::Pressed(at, _) => Some(at + if self.settled { LONG_PRESS } else { TAP_TIMEOUT }),
            _ => None,
        }
    }

    pub fn active(&self) -> bool {
        self.moved.is_some() || self.fling.is_some() || self.scroll != Vec2::ZERO || self.zoom != 0.0
    }

    pub fn frame(&mut self, at: Instant) -> Frame {
        self.sample();
        let mut scroll = std::mem::take(&mut self.scroll);
        if let Some(step) = self.fling.as_mut().map(|fling| fling.advance(at)) {
            match step {
                Some((x, y)) => scroll += Vec2::new(x, y),
                None => self.fling = None,
            }
        }
        let zoom = std::mem::take(&mut self.zoom);
        Frame { scroll: (scroll != Vec2::ZERO).then_some((self.down, scroll)), pinch: (zoom != 0.0).then_some((self.focus, zoom.exp_m1())) }
    }

    fn release(&mut self, at: Instant) -> Option<Fling> {
        self.sample();
        let (max, min) = (FLING_MAX_VELOCITY * self.dp, FLING_MIN_VELOCITY * self.dp);
        let (vx, vy) = self.trail.velocity(at)?;
        let velocity = (vx.clamp(-max, max), vy.clamp(-max, max));
        (velocity.0.abs() > min || velocity.1.abs() > min).then(|| Fling::spline(at, velocity, self.dp))
    }

    fn sample(&mut self) {
        if let Some(at) = self.moved.take() {
            self.trail.push(at, self.travel.x, self.travel.y);
        }
    }

    fn lift(&mut self, finger: usize) -> bool {
        let count = self.fingers.len();
        self.fingers.retain(|&(id, _)| id != finger);
        let lifted = self.fingers.len() < count;
        if lifted && !self.fingers.is_empty() {
            self.regroup();
        }
        lifted
    }

    fn regroup(&mut self) {
        self.focus = self.centroid();
        self.scale.regroup(self.span(), self.dp);
    }

    fn centroid(&self) -> Point {
        let sum = self.fingers.iter().fold(Vec2::ZERO, |sum, &(_, p)| sum + p.to_vec2());
        (sum / self.fingers.len() as f64).to_point()
    }

    fn span(&self) -> f64 {
        let deviation = self.fingers.iter().fold(Vec2::ZERO, |sum, &(_, p)| sum + Vec2::new((p.x - self.focus.x).abs(), (p.y - self.focus.y).abs()));
        (deviation * 2.0 / self.fingers.len() as f64).hypot()
    }
}
