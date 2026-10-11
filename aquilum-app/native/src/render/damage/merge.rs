use masonry::kurbo::Rect;

use super::Tracker;

const MAX_AREAS: usize = 4;

impl Tracker {
    pub(super) fn areas(&self, rects: Vec<Rect>) -> Vec<(u16, u16, u16, u16)> {
        let mut areas: Vec<Rect> = Vec::new();
        for rect in rects.into_iter().map(|r| r.intersect(self.frame)).filter(|r| r.width() > 0.0 && r.height() > 0.0) {
            areas.push(rect);
            while let Some((a, b)) = closest(&areas) {
                let merged = areas[a].union(areas[b]);
                if areas.len() <= MAX_AREAS && area(merged) > (area(areas[a]) + area(areas[b])) * 1.5 {
                    break;
                }
                areas[a] = merged;
                areas.swap_remove(b);
            }
        }
        areas
            .into_iter()
            .filter_map(|d| {
                let d = d.inflate(1.0, 1.0).expand();
                let d = Rect::new((d.x0 / 4.0).floor() * 4.0, (d.y0 / 4.0).floor() * 4.0, d.x1, d.y1).intersect(self.frame);
                (d.width() > 0.0 && d.height() > 0.0).then(|| (d.x0 as u16, d.y0 as u16, d.width() as u16, d.height() as u16))
            })
            .collect()
    }
}

fn area(r: Rect) -> f64 {
    r.width() * r.height()
}

fn closest(areas: &[Rect]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, f64)> = None;
    for a in 0..areas.len() {
        for b in a + 1..areas.len() {
            let cost = area(areas[a].union(areas[b])) - area(areas[a]) - area(areas[b]);
            if best.is_none_or(|(_, _, c)| cost < c) {
                best = Some((a, b, cost));
            }
        }
    }
    best.map(|(a, b, _)| (a, b))
}
