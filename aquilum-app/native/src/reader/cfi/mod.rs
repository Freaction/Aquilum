mod extract;
mod format;
mod locate;
mod parse;

pub use extract::{order, text};
pub use format::join;
pub use locate::{Point, element_path, from_range, resolve};
pub use parse::parse;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub index: u32,
    pub id: Option<String>,
    pub offset: Option<u32>,
    pub temporal: Option<f64>,
    pub spatial: Vec<f64>,
    pub text: Vec<String>,
    pub side: Option<String>,
}

pub type Path = Vec<Part>;
pub type Steps = Vec<Path>;

#[derive(Clone, Debug, PartialEq)]
pub enum Cfi {
    Point(Steps),
    Range { parent: Steps, start: Steps, end: Steps },
}

fn concat(a: &Steps, b: &Steps) -> Steps {
    let Some((last, head)) = a.split_last() else { return b.clone() };
    let mut out = head.to_vec();
    let mut joined = last.clone();
    joined.extend(b.first().cloned().unwrap_or_default());
    out.push(joined);
    out.extend(b.iter().skip(1).cloned());
    out
}

impl Cfi {
    pub fn collapse(&self, to_end: bool) -> Steps {
        match self {
            Cfi::Point(steps) => steps.clone(),
            Cfi::Range { parent, start, end } => concat(parent, if to_end { end } else { start }),
        }
    }

    pub fn range(from: &Steps, to: &Steps) -> Cfi {
        let (local_from, local_to) = (from.last().cloned().unwrap_or_default(), to.last().cloned().unwrap_or_default());
        let (mut parent, mut start, mut end) = (Vec::new(), Vec::new(), Vec::new());
        let mut shared = true;
        for i in 0..local_from.len().max(local_to.len()) {
            let (a, b) = (local_from.get(i), local_to.get(i));
            let empty = |p: Option<&Part>| p.and_then(|p| p.offset).unwrap_or(0) == 0;
            shared = shared && a.map(|p| p.index) == b.map(|p| p.index) && empty(a) && empty(b);
            if shared {
                parent.extend(a.cloned());
            } else {
                start.extend(a.cloned());
                end.extend(b.cloned());
            }
        }
        let mut steps = from[..from.len().saturating_sub(1)].to_vec();
        steps.push(parent);
        Cfi::Range { parent: steps, start: vec![start], end: vec![end] }
    }

    pub fn section(&self) -> Option<usize> {
        let first = match self {
            Cfi::Point(steps) => steps.first()?,
            Cfi::Range { parent, .. } => parent.first()?,
        };
        let index = first.last()?.index;
        (index >= 2).then(|| (index / 2 - 1) as usize)
    }

    pub fn local(&self) -> Cfi {
        match self {
            Cfi::Point(steps) => Cfi::Point(steps[1.min(steps.len())..].to_vec()),
            Cfi::Range { parent, start, end } => Cfi::Range { parent: parent[1.min(parent.len())..].to_vec(), start: start.clone(), end: end.clone() },
        }
    }
}
