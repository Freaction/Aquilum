use std::hash::Hasher;

use masonry::kurbo::Rect;

use super::hash::H;
use super::{Level, Tracker, close};

impl Tracker {
    pub(super) fn top(&self) -> Level {
        *self.stack.last().expect("корень стека")
    }

    pub(super) fn lift(&self, region: Option<usize>) -> f64 {
        region.map_or(0.0, |r| self.regions[r].offset)
    }

    pub(super) fn push(&mut self, hash: u64, rect: Rect, faded: bool) {
        let level = self.top();
        let mut h = H::new(11);
        h.0.write_u64(level.state);
        h.0.write_u64(hash);
        let found = level.region.is_none().then(|| self.regions.iter().position(|r| !r.started && close(r.bounds, rect))).flatten();
        if let Some(found) = found {
            self.regions[found].started = true;
        }
        let clip = level.clip.intersect(found.map_or(rect, |i| self.regions[i].viewport));
        let region = level.region.or(found);
        self.stack.push(Level { state: h.finish(), clip, region, faded: level.faded || faded });
    }

    pub(super) fn pop(&mut self) {
        if self.stack.len() > 1 {
            let level = self.stack.pop().expect("уровень стека");
            if let Some(region) = level.region
                && self.top().region != Some(region)
            {
                self.regions[region].ended = true;
            }
        }
    }
}
