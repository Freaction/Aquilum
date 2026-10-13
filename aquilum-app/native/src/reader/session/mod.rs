mod layout;
mod navigate;
mod select;

use std::collections::HashMap;
use std::path::Path;

use masonry::kurbo::Size;
use masonry::peniko::ImageBrush;

use super::cfi::{self, Point};
use super::flow::{self, Block};
use super::page::{Geometry, Laid, Settings};
use super::Book;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spot {
    pub block: usize,
    pub byte: usize,
}

pub enum Target {
    Start,
    Cfi(String),
    Fraction(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Anchor {
    Start,
    End,
    At(usize, usize),
    Fraction(f64),
}

pub struct Session {
    book: Book,
    sizes: Vec<f64>,
    total: f64,
    section: usize,
    loaded: Option<usize>,
    blocks: Vec<Block>,
    laid: Option<Laid>,
    key: Option<(Size, Settings)>,
    geometry: Geometry,
    view: Size,
    page: usize,
    scroll: f64,
    anchor: Anchor,
    placed: (usize, f64),
    images: HashMap<String, Option<ImageBrush>>,
}

impl Session {
    pub fn open(path: &Path, target: Target) -> Option<Session> {
        let book = Book::open(path)?;
        if book.sections.is_empty() {
            return None;
        }
        let sizes: Vec<f64> = (0..book.sections.len()).map(|i| book.size(i) as f64).collect();
        let total = sizes.iter().sum::<f64>().max(1.0);
        let geometry = Geometry { columns: 1, column_width: 1.0, gap: 0.0, left: 0.0, top: 0.0, height: 1.0, size: 1.0 };
        let mut session = Session {
            book,
            sizes,
            total,
            section: 0,
            loaded: None,
            blocks: Vec::new(),
            laid: None,
            key: None,
            geometry,
            view: Size::ZERO,
            page: 0,
            scroll: 0.0,
            anchor: Anchor::Start,
            placed: (0, 0.0),
            images: HashMap::new(),
        };
        session.go(target);
        Some(session)
    }

    pub fn go(&mut self, target: Target) {
        match target {
            Target::Start => self.enter(0, Anchor::Start),
            Target::Cfi(source) => match self.book.resolve(&source) {
                Some((index, start, _)) => {
                    self.enter(index, Anchor::Start);
                    if let Some((block, byte)) = self.locate(start) {
                        self.anchor = Anchor::At(block, byte);
                        self.place();
                    }
                }
                None => self.enter(0, Anchor::Start),
            },
            Target::Fraction(fraction) => {
                let goal = fraction.clamp(0.0, 1.0) * self.total;
                let mut before = 0.0;
                let last = self.sizes.len() - 1;
                for (index, &size) in self.sizes.iter().enumerate() {
                    if before + size > goal || index == last {
                        let within = if size > 0.0 { ((goal - before) / size).clamp(0.0, 1.0) } else { 0.0 };
                        self.enter(index, Anchor::Fraction(within));
                        return;
                    }
                    before += size;
                }
            }
        }
    }

    fn enter(&mut self, index: usize, anchor: Anchor) {
        self.section = index;
        if self.loaded != Some(index) {
            self.loaded = Some(index);
            self.blocks = self.book.document(index).map(flow::blocks).unwrap_or_default();
            self.images.clear();
            self.laid = None;
        }
        self.anchor = anchor;
        self.place();
    }

    fn locate(&self, point: Point) -> Option<(usize, usize)> {
        let dom = self.book.document(self.section)?;
        if let Point::At(node, offset) = point
            && dom.text_of(node).is_some()
        {
            for (index, block) in self.blocks.iter().enumerate() {
                if let Block::Text { text, origins, .. } = block
                    && origins.iter().any(|o| o.node == node)
                {
                    return flow::byte(text, origins, node, offset).map(|byte| (index, byte));
                }
            }
        }
        let (enter, exit) = cfi::order(dom);
        let target = match point {
            Point::At(node, _) if dom.text_of(node).is_some() => enter[node as usize],
            Point::At(node, offset) => dom.node(node).children.get(offset as usize).map_or(exit[node as usize], |&c| enter[c as usize]),
            Point::Before(node) => enter[node as usize],
            Point::After(node) => exit[node as usize],
        };
        for (index, block) in self.blocks.iter().enumerate() {
            match block {
                Block::Text { origins, .. } => {
                    if let Some(origin) = origins.iter().find(|o| enter[o.node as usize] >= target) {
                        return Some((index, origin.byte));
                    }
                }
                Block::Image { node, .. } if enter[*node as usize] >= target => return Some((index, 0)),
                _ => {}
            }
        }
        Some((self.blocks.len().saturating_sub(1), 0))
    }
}
