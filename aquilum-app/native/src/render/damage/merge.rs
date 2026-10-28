use masonry::kurbo::Rect;

use super::Tracker;
use crate::output::closest;
use crate::render::layer::area_of;

const MAX_AREAS: usize = 4;

impl Tracker {
    pub(super) fn areas(&self, rects: Vec<Rect>) -> Vec<(u16, u16, u16, u16)> {
        let mut areas: Vec<Rect> = Vec::new();
        for rect in rects.into_iter().map(|r| r.intersect(self.frame)).filter(|r| r.width() > 0.0 && r.height() > 0.0) {
            areas.push(rect);
            while let Some((a, b, _)) = closest(&areas, |x, y| area(x.union(y)) - area(x) - area(y)) {
                let merged = areas[a].union(areas[b]);
                if areas.len() <= MAX_AREAS && area(merged) > (area(areas[a]) + area(areas[b])) * 1.5 {
                    break;
                }
                areas[a] = merged;
                areas.swap_remove(b);
            }
        }
        areas.into_iter().filter_map(|d| area_of(d.inflate(1.0, 1.0), self.frame)).collect()
    }
}

fn area(r: Rect) -> f64 {
    r.width() * r.height()
}
