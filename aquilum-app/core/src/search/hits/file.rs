use crate::files::document::normalize_line_endings_in_place;
use crate::text::cancel::Cancel;
use crate::text::matcher::Matcher;
use crate::text::utf16::utf16_len;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub struct FirstMatch {
    pub byte: usize,
    pub utf16: usize,
}

struct NoteLines {
    reader: BufReader<File>,
    bytes: Vec<u8>,
}

impl NoteLines {
    fn open(path: &Path) -> io::Result<Self> {
        Ok(Self { reader: BufReader::new(File::open(path)?), bytes: Vec::new() })
    }

    fn next(&mut self) -> io::Result<Option<&str>> {
        self.bytes.clear();
        if self.reader.read_until(b'\n', &mut self.bytes)? == 0 {
            return Ok(None);
        }
        let kept = normalize_line_endings_in_place(&mut self.bytes);
        self.bytes.truncate(kept);
        std::str::from_utf8(&self.bytes)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

pub fn first_match(path: &Path, matcher: &Matcher, cancel: &Cancel) -> io::Result<Option<FirstMatch>> {
    let mut lines = NoteLines::open(path)?;
    let (mut byte, mut utf16) = (0, 0);
    while let Some(line) = lines.next()? {
        if let Some(found) = matcher.first(line) {
            let before = &line[..found.start];
            return Ok(Some(FirstMatch { byte: byte + before.len(), utf16: utf16 + utf16_len(before) }));
        }
        if cancel.is_cancelled() {
            return Ok(None);
        }
        (byte, utf16) = (byte + line.len(), utf16 + utf16_len(line));
    }
    Ok(None)
}

pub fn read_through(path: &Path, at: usize, after: usize) -> io::Result<(String, bool)> {
    let mut lines = NoteLines::open(path)?;
    let mut text = String::new();
    let mut lines_after = 0;
    while text.len() < at + after || lines_after < 2 {
        let Some(line) = lines.next()? else {
            return Ok((text, true));
        };
        text.push_str(line);
        if text.len() > at {
            lines_after += 1;
        }
    }
    let ended = lines.reader.fill_buf()?.is_empty();
    Ok((text, ended))
}
