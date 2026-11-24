use masonry::kurbo::Rect;

use super::Tracker;
use crate::render::layer::area_of;

const MAX_AREAS: usize = 4;

pub fn merge_closest<T: Copy>(items: &mut Vec<T>, cost: impl Fn(T, T) -> f64, union: impl Fn(T, T) -> T, enough: impl Fn(usize, T, T, f64) -> bool) {
    loop {
        let mut best: Option<(usize, usize, f64)> = None;
        for a in 0..items.len() {
            for b in a + 1..items.len() {
                let extra = cost(items[a], items[b]);
                if best.is_none_or(|(_, _, c)| extra < c) {
                    best = Some((a, b, extra));
                }
            }
        }
        let Some((a, b, extra)) = best else { return };
        if enough(items.len(), items[a], items[b], extra) {
            return;
        }
        items[a] = union(items[a], items[b]);
        items.swap_remove(b);
    }
}

impl Tracker {
    pub(super) fn areas(&self, rects: Vec<Rect>) -> Vec<(u16, u16, u16, u16)> {
        let mut areas: Vec<Rect> = Vec::new();
        for rect in rects.into_iter().map(|r| r.intersect(self.frame)).filter(|r| r.width() > 0.0 && r.height() > 0.0) {
            areas.push(rect);
            merge_closest(&mut areas, |x, y| area(x.union(y)) - area(x) - area(y), |x, y| x.union(y), |count, a, b, _| count <= MAX_AREAS && area(a.union(b)) > (area(a) + area(b)) * 1.5);
        }
        areas.into_iter().filter_map(|d| area_of(d.inflate(1.0, 1.0), self.frame)).collect()
    }
}

fn area(r: Rect) -> f64 {
    r.width() * r.height()
}
