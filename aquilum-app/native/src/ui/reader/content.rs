use masonry::core::BrushIndex;
use masonry::kurbo::{Point, Rect, Size};
use masonry::parley::{FontContext, Layout, LayoutContext};
use masonry::peniko::ImageBrush;

use crate::reader::{
    page::{Column, Item, Settings},
    session::{Session, Spot},
};

pub(super) enum Content {
    Flow(Box<Session>),
    Pdf(Box<crate::reader::pdf::Session>),
}

impl Content {
    pub fn pdf(&self) -> Option<&crate::reader::pdf::Session> {
        if let Self::Pdf(pdf) = self { Some(pdf) } else { None }
    }
    pub fn pdf_mut(&mut self) -> Option<&mut crate::reader::pdf::Session> {
        if let Self::Pdf(pdf) = self { Some(pdf) } else { None }
    }
    fn flow(&self) -> Option<&Session> {
        if let Self::Flow(flow) = self { Some(flow) } else { None }
    }
    pub fn location(&self) -> Option<(String, f64)> {
        match self {
            Self::Flow(s) => s.location(),
            Self::Pdf(s) => s.location(),
        }
    }
    pub fn next(&mut self) -> bool {
        match self {
            Self::Flow(s) => s.next(),
            Self::Pdf(s) => s.turn(true),
        }
    }
    pub fn prev(&mut self) -> bool {
        match self {
            Self::Flow(s) => s.prev(),
            Self::Pdf(s) => s.turn(false),
        }
    }
    pub fn needs_layout(&self) -> bool {
        self.flow().is_some_and(Session::needs_layout)
    }
    pub fn ensure(&mut self, view: Size, settings: &Settings, dpi: f64, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>) {
        match self {
            Self::Flow(s) => s.ensure(view, settings, fcx, lcx),
            Self::Pdf(s) => s.ensure(view, dpi),
        }
    }
    pub fn scroll_by(&mut self, delta: f64) -> bool {
        match self {
            Self::Flow(s) => s.scroll_by(delta),
            Self::Pdf(s) => s.pan((0.0, -delta).into()),
        }
    }
    pub fn section(&self) -> usize {
        self.flow().map_or(0, Session::section)
    }
    pub fn visible(&self) -> Vec<(Rect, f64, &Column)> {
        self.flow().map_or_else(Vec::new, Session::visible)
    }
    pub fn items(&self) -> &[Item] {
        self.flow().map_or(&[], Session::items)
    }
    pub fn layout(&self, item: usize) -> Option<&Layout<BrushIndex>> {
        self.flow()?.layout(item)
    }
    pub fn image(&self, item: usize) -> Option<&ImageBrush> {
        self.flow()?.image(item)
    }
    pub fn hit(&self, at: Point, word: bool) -> Option<(Spot, Spot)> {
        self.flow()?.hit(at, word)
    }
    pub fn rects(&self, from: Spot, to: Spot) -> Vec<Rect> {
        self.flow().map_or_else(Vec::new, |s| s.rects(from, to))
    }
    pub fn text(&self, from: Spot, to: Spot) -> String {
        self.flow().map_or_else(String::new, |s| s.text(from, to))
    }
    pub fn range_cfi(&self, from: Spot, to: Spot) -> Option<String> {
        self.flow()?.range_cfi(from, to)
    }
    pub fn resolve(&self, cfi: &str) -> Option<(Spot, Spot)> {
        self.flow()?.resolve(cfi)
    }
}
