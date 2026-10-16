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

pub fn appended(last: Option<u8>, quote: &str) -> String {
    let gap = if last.is_some_and(|byte| byte != b'\n') { "\n\n" } else { "\n" };
    format!("{gap}{quote}")
}
