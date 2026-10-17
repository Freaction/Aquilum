pub mod char_edits;
pub mod line_diff;
pub mod merge;

pub use char_edits::{apply_edits, TextEdit};
pub use merge::merge_external_change;

#[cfg(test)]
mod random_tests;
#[cfg(test)]
mod tests;
