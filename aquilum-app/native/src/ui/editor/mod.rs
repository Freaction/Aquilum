mod caret;
mod change;
mod document;
mod embed;
mod highlight;
mod history;
mod markdown;
pub mod resolve;
mod table;
mod rename;
mod widget;

#[cfg(test)]
mod tests;

pub use rename::rename_editor;
pub use table::Command as TableCommand;
pub use embed::media::media_line;
pub use change::Change;
pub use widget::{Search, Smart, Status, TextEditor};

#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub range: std::ops::Range<usize>,
    pub inserted: usize,
    pub head: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextAction {
    Changed(String),
    Edited(Vec<Edit>),
    Outside(Vec<(std::ops::Range<usize>, String)>),
    Entered(String),
    Cancelled,
    Page { down: bool, extend: bool },
    Line { down: bool },
    Menu(Option<ContextMenu>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextMenu {
    pub around: masonry::kurbo::Rect,
    pub selected: bool,
    pub inserts: bool,
}
