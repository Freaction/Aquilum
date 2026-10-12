mod document;
mod embed;
mod highlight;
mod history;
mod markdown;
pub mod resolve;
mod table;
mod widget;

#[cfg(test)]
mod tests;

pub use table::Command as TableCommand;
pub use embed::media::media_line;
pub use widget::{Smart, TextEditor};

#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub range: std::ops::Range<usize>,
    pub insert: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextAction {
    Changed(String),
    Edited(Vec<Edit>),
    Entered(String),
    Cancelled,
}

#[derive(Clone, Debug, Default)]
pub struct Search {
    pub query: String,
    pub only: Option<usize>,
}

impl Search {
    pub fn ranges(&self, text: &str, seen: &mut usize) -> Vec<std::ops::Range<usize>> {
        let found = widget::find_all(text, &self.query);
        let first = *seen;
        *seen += found.len();
        match self.only {
            None => found,
            Some(k) => found.into_iter().enumerate().filter(|(i, _)| first + i == k).map(|(_, r)| r).collect(),
        }
    }
}
