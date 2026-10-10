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
pub use embed::media::is_video;
pub use widget::TextEditor;
