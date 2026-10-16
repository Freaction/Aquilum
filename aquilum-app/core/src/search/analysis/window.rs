use crate::files::document::text_from_bytes;
use std::io::Read;
use std::path::Path;

pub const WINDOW_BYTES: usize = 64 * 1024;

fn cut(bytes: &[u8]) -> usize {
    if bytes.len() <= WINDOW_BYTES {
        return bytes.len();
    }
    match bytes[..WINDOW_BYTES].iter().rposition(|&b| b == b'\n') {
        Some(newline) => newline + 1,
        None => (0..=WINDOW_BYTES).rev().find(|&at| bytes[at] & 0xC0 != 0x80).unwrap_or(0),
    }
}

pub fn window(text: &str) -> &str {
    &text[..cut(text.as_bytes())]
}

pub fn read_window(path: &Path) -> std::io::Result<String> {
    let mut bytes = Vec::with_capacity(WINDOW_BYTES + 1);
    std::fs::File::open(path)?.take(WINDOW_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    bytes.truncate(cut(&bytes));
    text_from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::{read_window, window, WINDOW_BYTES};

    #[test]
    fn short_text_is_taken_whole() {
        assert_eq!(window("одна строка\nвторая"), "одна строка\nвторая");
    }

    #[test]
    fn long_text_is_cut_at_the_last_line_break_inside_the_window() {
        let line = "строка из слов\n";
        let text = line.repeat(WINDOW_BYTES / line.len() + 10);
        let taken = window(&text);
        assert!(taken.len() <= WINDOW_BYTES && taken.ends_with('\n'));
        assert!(WINDOW_BYTES - taken.len() < line.len());
    }

    #[test]
    fn a_single_long_line_is_cut_at_a_char_boundary() {
        let text = "ж".repeat(WINDOW_BYTES);
        let taken = window(&text);
        assert!(taken.len() <= WINDOW_BYTES && taken.len() > WINDOW_BYTES - 2);
    }

    #[test]
    fn file_window_matches_text_window() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.md");
        let text = "слово ".repeat(WINDOW_BYTES / 5) + "\nконец";
        std::fs::write(&path, &text).unwrap();
        assert_eq!(read_window(&path).unwrap(), window(&text));
    }
}
