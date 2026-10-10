use masonry::kurbo::Point;
use masonry::peniko::ImageBrush;

use super::table::Command;

#[derive(Clone, Debug, PartialEq)]
pub enum Fetch<T> {
    Pending,
    Ready(T),
    Missing(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(String),
    Link { text: String, target: String },
    Progress(f64),
    Check { done: bool, target: String, line: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Table,
    List,
    Tasks,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub shape: Shape,
    pub title: Option<String>,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Vec<Part>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Book {
    pub title: String,
    pub linked: bool,
    pub author: String,
    pub cover: Option<ImageBrush>,
    pub pages: Option<(u32, u32)>,
    pub file: Option<std::path::PathBuf>,
    pub page: Option<std::path::PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Media {
    pub image: Option<ImageBrush>,
    pub video: bool,
    pub path: Option<std::path::PathBuf>,
}

#[derive(Clone, Debug)]
pub enum Request {
    TableMenu { at: Point, commands: Vec<(Command, bool)> },
    Open { target: String, wiki: bool, new_tab: bool },
    View(ImageBrush),
    ToggleTask { target: String, line: usize },
    Read { file: std::path::PathBuf, page: Option<std::path::PathBuf>, title: String },
    Suggest(Option<Suggestions>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Suggestions {
    pub editor: masonry::core::WidgetId,
    pub caret: masonry::kurbo::Rect,
    pub query: String,
    pub items: Vec<String>,
    pub selected: usize,
}

pub trait Resolver: Send + Sync {
    fn dataview(&self, query: &str) -> (Fetch<Grid>, u64);
    fn media(&self, source: &str, wiki: bool) -> (Fetch<Media>, u64);
    fn book(&self, target: &str) -> (Fetch<Book>, u64);
    fn request(&self, request: Request);
    fn sweep(&self) {}
    fn suggest(&self, _query: &str) -> Option<Vec<String>> {
        None
    }
    fn suggest_min(&self) -> Option<usize> {
        None
    }
}
