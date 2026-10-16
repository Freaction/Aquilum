use std::ops::Range;

use masonry::core::BrushIndex;
use masonry::parley::{Cluster, Layout};

use super::super::embed::Formula;

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub label: Range<usize>,
    pub target: String,
    pub wiki: bool,
}

#[derive(Default)]
pub struct Decor {
    pub x: f64,
    pub inset: f64,
    pub pad: (f64, f64),
    pub hang: Option<usize>,
    pub codes: Vec<Range<usize>>,
    pub marks: Vec<Range<usize>>,
    pub tags: Vec<(Range<usize>, Option<usize>)>,
    pub links: Vec<Link>,
    pub caret: Option<usize>,
    pub formulas: Vec<Formula>,
}

impl Decor {
    pub fn link_at(&self, layout: &Layout<BrushIndex>, x: f64, y: f64) -> Option<&Link> {
        if self.links.is_empty() || y < 0.0 || y > f64::from(layout.height()) || x < 0.0 || x > f64::from(layout.width()) {
            return None;
        }
        let (cluster, _) = Cluster::from_point_exact(layout, x as f32, y as f32)?;
        let at = cluster.text_range().start;
        self.links.iter().find(|l| l.label.start <= at && at < l.label.end)
    }
}
