pub mod char_edits;
pub mod line_diff;
pub mod merge;
pub mod utf16;

pub use char_edits::TextEdit;
pub use merge::merge_external_change;
pub use utf16::{utf16_byte, utf16_len};

#[cfg(test)]
mod tests;
