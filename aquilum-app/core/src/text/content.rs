use std::io::{self, BufWriter, Write};
use std::ops::Range;

pub const BLOCK: usize = 64 * 1024;

pub trait Content {
    fn byte_len(&self) -> usize;
    fn read(&self, range: Range<usize>, visit: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> io::Result<()>;
}

impl Content for [u8] {
    fn byte_len(&self) -> usize {
        self.len()
    }

    fn read(&self, range: Range<usize>, visit: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        visit(&self[range])
    }
}

impl Content for str {
    fn byte_len(&self) -> usize {
        self.len()
    }

    fn read(&self, range: Range<usize>, visit: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        self.as_bytes().read(range, visit)
    }
}

impl Content for String {
    fn byte_len(&self) -> usize {
        self.len()
    }

    fn read(&self, range: Range<usize>, visit: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        self.as_str().read(range, visit)
    }
}

impl<T: Content + ?Sized> Content for &T {
    fn byte_len(&self) -> usize {
        (**self).byte_len()
    }

    fn read(&self, range: Range<usize>, visit: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        (**self).read(range, visit)
    }
}

impl<T: Content + ?Sized> Content for Box<T> {
    fn byte_len(&self) -> usize {
        (**self).byte_len()
    }

    fn read(&self, range: Range<usize>, visit: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        (**self).read(range, visit)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Splice {
    pub at: usize,
    pub removed: usize,
    pub inserted: usize,
}

pub fn read_bytes(content: &dyn Content, range: Range<usize>) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(range.len());
    content.read(range, &mut |chunk| {
        bytes.extend_from_slice(chunk);
        Ok(())
    })?;
    Ok(bytes)
}

pub fn read_string(content: &dyn Content, range: Range<usize>) -> io::Result<String> {
    Ok(String::from_utf8_lossy(&read_bytes(content, range)?).into_owned())
}

pub fn find_byte(content: &dyn Content, range: Range<usize>, backwards: bool, test: impl Fn(u8) -> bool) -> io::Result<Option<usize>> {
    let blocks = (range.start..range.end).step_by(BLOCK).map(|from| from..(from + BLOCK).min(range.end));
    let blocks: Box<dyn Iterator<Item = Range<usize>>> = if backwards { Box::new(blocks.rev()) } else { Box::new(blocks) };
    for block in blocks {
        let bytes = read_bytes(content, block.clone())?;
        let found = if backwards { bytes.iter().rposition(|byte| test(*byte)) } else { bytes.iter().position(|byte| test(*byte)) };
        if let Some(index) = found {
            return Ok(Some(block.start + index));
        }
    }
    Ok(None)
}

pub fn splice(old: &dyn Content, new: &dyn Content) -> io::Result<Splice> {
    let (old_len, new_len) = (old.byte_len(), new.byte_len());
    let shorter = old_len.min(new_len);
    let mut prefix = 0;
    while prefix < shorter {
        let end = (prefix + BLOCK).min(shorter);
        let same = common(&read_bytes(old, prefix..end)?, &read_bytes(new, prefix..end)?, false);
        let full = same == end - prefix;
        prefix += same;
        if !full {
            break;
        }
    }
    let limit = shorter - prefix;
    let mut suffix = 0;
    while suffix < limit {
        let take = BLOCK.min(limit - suffix);
        let a = read_bytes(old, old_len - suffix - take..old_len - suffix)?;
        let same = common(&a, &read_bytes(new, new_len - suffix - take..new_len - suffix)?, true);
        suffix += same;
        if same < take {
            break;
        }
    }
    Ok(Splice { at: prefix, removed: old_len - prefix - suffix, inserted: new_len - prefix - suffix })
}

fn common(a: &[u8], b: &[u8], backwards: bool) -> usize {
    match backwards {
        true => a.iter().rev().zip(b.iter().rev()).take_while(|(x, y)| x == y).count(),
        false => a.iter().zip(b).take_while(|(x, y)| x == y).count(),
    }
}

pub fn starts_char(byte: u8) -> bool {
    byte & 0xC0 != 0x80
}

pub fn char_floor(bytes: &[u8], at: usize) -> usize {
    (0..=at.min(bytes.len())).rev().find(|&at| bytes.get(at).is_none_or(|&byte| starts_char(byte))).unwrap_or(0)
}

pub fn chars_below(content: &dyn Content, ranges: impl IntoIterator<Item = Range<usize>>, limit: usize) -> io::Result<bool> {
    let mut chars = 0;
    for range in ranges {
        if range.len() >= limit * 4 {
            return Ok(false);
        }
        chars += read_bytes(content, range)?.iter().filter(|&&byte| starts_char(byte)).count();
        if chars >= limit {
            return Ok(false);
        }
    }
    Ok(true)
}

pub struct HashedWriter<W: Write> {
    out: BufWriter<W>,
    hasher: blake3::Hasher,
}

impl<W: Write> HashedWriter<W> {
    pub fn new(out: W) -> Self {
        Self { out: BufWriter::with_capacity(16 * BLOCK, out), hasher: blake3::Hasher::new() }
    }

    pub fn put(&mut self, content: &dyn Content) -> io::Result<()> {
        self.put_range(content, 0..content.byte_len())
    }

    pub fn put_range(&mut self, content: &dyn Content, range: Range<usize>) -> io::Result<()> {
        content.read(range, &mut |bytes| {
            self.hasher.update(bytes);
            self.out.write_all(bytes)
        })
    }

    pub fn finish(mut self) -> io::Result<String> {
        self.out.flush()?;
        Ok(self.hasher.finalize().to_hex().to_string())
    }
}

pub fn hash(content: &dyn Content) -> io::Result<String> {
    let mut writer = HashedWriter::new(io::sink());
    writer.put(content)?;
    writer.finish()
}

pub struct ContentReader<'a> {
    content: &'a dyn Content,
    at: usize,
}

impl<'a> ContentReader<'a> {
    pub fn new(content: &'a dyn Content) -> Self {
        Self { content, at: 0 }
    }
}

impl io::Read for ContentReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let end = (self.at + buf.len()).min(self.content.byte_len());
        let mut filled = 0;
        self.content.read(self.at..end, &mut |bytes| {
            buf[filled..filled + bytes.len()].copy_from_slice(bytes);
            filled += bytes.len();
            Ok(())
        })?;
        self.at = end;
        Ok(filled)
    }
}

#[cfg(test)]
pub fn collect(content: &dyn Content) -> String {
    read_string(content, 0..content.byte_len()).unwrap()
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
