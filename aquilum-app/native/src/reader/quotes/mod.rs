mod link;
mod scan;
#[cfg(test)]
mod tests;

use std::ops::Range;

pub use link::format;
pub use scan::find;

#[derive(Clone, Debug, PartialEq)]
pub struct Quote {
    pub line: Range<usize>,
    pub text: Range<usize>,
    pub cfi: String,
    pub book: Option<String>,
    pub label: String,
}

pub fn append(doc: &str, quote: &str) -> String {
    let gap = if !doc.is_empty() && !doc.ends_with('\n') { "\n\n" } else { "\n" };
    format!("{doc}{gap}{quote}")
}
