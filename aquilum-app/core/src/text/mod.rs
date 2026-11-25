pub mod calendar;
pub mod cancel;
pub mod collate;
pub mod content;
pub mod matcher;
pub mod pieces;
pub mod utf16;
pub mod utf8;

pub fn is_cyrillic(ch: char) -> bool {
    matches!(ch, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}')
}
