use super::error::FileCommandError;
use super::models::FileWriteResult;
use atomic_write_file::AtomicWriteFile;
use std::fs;
use super::content::FileContent;
use crate::text::content::{self, Content, HashedWriter};
use crate::search::pages::Pages;
use std::io::ErrorKind;
use std::ops::Range;
use std::path::Path;

pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

pub fn hash_file(path: &Path) -> std::io::Result<String> {
    content::hash(&FileContent::open(path)?)
}

pub fn normalize_line_endings_in_place(bytes: &mut [u8]) -> usize {
    let Some(mut read) = memchr::memchr(b'\r', bytes) else {
        return bytes.len();
    };
    let mut write = read;
    while read < bytes.len() {
        bytes[write] = b'\n';
        write += 1;
        read += 1 + usize::from(bytes.get(read + 1) == Some(&b'\n'));
        let next = memchr::memchr(b'\r', &bytes[read..]).map_or(bytes.len(), |at| read + at);
        bytes.copy_within(read..next, write);
        write += next - read;
        read = next;
    }
    write
}

pub fn normalize_tail(buf: &mut Vec<u8>, from: usize, ended: bool) -> usize {
    let end = buf.len() - usize::from(!ended && buf.last() == Some(&b'\r'));
    let kept = normalize_line_endings_in_place(&mut buf[from..end]);
    buf.drain(from + kept..end);
    from + kept
}

pub fn text_from_bytes(mut bytes: Vec<u8>) -> std::io::Result<String> {
    let kept = normalize_line_endings_in_place(&mut bytes);
    bytes.truncate(kept);
    crate::text::utf8::string(bytes)
}

pub fn normalize_line_endings(content: String) -> String {
    text_from_bytes(content.into_bytes()).expect("line endings are ASCII, so the text stays UTF-8")
}

pub const HEAD_LIMIT: usize = 64 * 1024;

pub fn head(text: &dyn Content) -> std::io::Result<String> {
    let mut bytes = content::read_bytes(text, 0..text.byte_len().min(HEAD_LIMIT + 1))?;
    bytes.truncate(head_len(&bytes));
    text_from_bytes(bytes)
}

pub fn read_head(path: &Path) -> std::io::Result<String> {
    head(&FileContent::open(path)?)
}

fn head_len(bytes: &[u8]) -> usize {
    if bytes.len() <= HEAD_LIMIT {
        return bytes.len();
    }
    match memchr::memrchr(b'\n', &bytes[..HEAD_LIMIT]) {
        Some(newline) => newline + 1,
        None => content::char_floor(bytes, HEAD_LIMIT),
    }
}

pub fn read_text(path: &Path) -> std::io::Result<String> {
    text_from_bytes(fs::read(path)?)
}

pub fn atomic(path: &Path, expected_hash: Option<&str>, fill: impl FnOnce(&mut HashedWriter<&mut AtomicWriteFile>) -> std::io::Result<()>) -> Result<FileWriteResult, FileCommandError> {
    let mut file = AtomicWriteFile::open(path)?;
    let mut out = HashedWriter::new(&mut file);
    fill(&mut out)?;
    let hash = out.finish()?;
    if let Some(expected) = expected_hash {
        let actual = match hash_file(path) {
            Ok(hash) => Some(hash),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if actual.as_deref() != Some(expected) {
            return Err(FileCommandError::Conflict { expected_hash: expected.to_owned(), actual_hash: actual });
        }
    }
    file.commit()?;
    Ok(FileWriteResult { hash })
}

pub fn write_content_atomic(path: &Path, content: &dyn Content, expected_hash: Option<&str>) -> Result<FileWriteResult, FileCommandError> {
    atomic(path, expected_hash, |out| out.put(content))
}

pub fn write_spliced_atomic(path: &Path, changes: &[(Range<usize>, String)], expected_hash: Option<&str>) -> Result<FileWriteResult, FileCommandError> {
    let mut pages = Pages::open(path)?;
    atomic(path, expected_hash, |out| {
        let mut changes = changes.iter().peekable();
        while let Some(page) = pages.next()? {
            let end = page.start + page.text.len();
            let mut cursor = 0;
            while let Some((range, replacement)) = changes.next_if(|(range, _)| range.start < end) {
                out.put_range(&page.text, cursor..range.start - page.start)?;
                out.put(replacement)?;
                cursor = range.end - page.start;
            }
            out.put_range(&page.text, cursor..page.text.len())?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::{head, HEAD_LIMIT};

    #[test]
    fn the_head_ends_at_the_last_whole_line_or_char() {
        assert_eq!(head(&"одна строка\r\nвторая").unwrap(), "одна строка\nвторая");
        let line = "строка из слов\n";
        let lines = line.repeat(HEAD_LIMIT / line.len() + 10);
        let taken = head(&lines.as_str()).unwrap();
        assert!(taken.len() <= HEAD_LIMIT && HEAD_LIMIT - taken.len() < line.len() && taken.ends_with('\n'));
        let taken = head(&"ж".repeat(HEAD_LIMIT)).unwrap();
        assert!(taken.len() <= HEAD_LIMIT && taken.len() > HEAD_LIMIT - 2);
    }
}
