use std::fs::File;
use std::io::Read;
use std::path::Path;

use aquilum_core::files::document::normalize_tail;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::text::utf8::prefix;

const BLOCK: usize = 64 * 1024;

pub type Stamp = (u64, std::time::SystemTime);

pub struct Loaded {
    pub text: crop::Rope,
    pub hash: String,
    pub stamp: Option<Stamp>,
}

fn stamp_of(metadata: std::io::Result<std::fs::Metadata>) -> Option<Stamp> {
    let metadata = metadata.ok()?;
    Some((metadata.len(), metadata.modified().ok()?))
}

pub fn stamp(path: &Path) -> Option<Stamp> {
    stamp_of(std::fs::metadata(path))
}

fn fill(file: &mut File, buf: &mut Vec<u8>) -> std::io::Result<usize> {
    let start = buf.len();
    buf.resize(start + BLOCK, 0);
    let mut got = 0;
    while got < BLOCK {
        match file.read(&mut buf[start + got..])? {
            0 => break,
            n => got += n,
        }
    }
    buf.truncate(start + got);
    Ok(got)
}

pub fn read(path: &Path) -> Result<Loaded, FileCommandError> {
    let mut file = File::open(path)?;
    let stamp = stamp_of(file.metadata());
    let file_len = stamp.as_ref().map(|s| s.0 as usize).unwrap_or(0);
    let initial_cap = file_len.min(BLOCK) + 8;
    let (mut hasher, mut builder, mut buf) = (blake3::Hasher::new(), crop::RopeBuilder::new(), Vec::with_capacity(initial_cap));
    loop {
        let start = buf.len();
        let eof = fill(&mut file, &mut buf)? < BLOCK;
        hasher.update(&buf[start..]);
        let kept = normalize_tail(&mut buf, 0, eof);
        let text = prefix(&buf[..kept], eof).map_err(|error| FileCommandError::InvalidUtf8 { message: error.to_string() })?;
        builder.append(text);
        let valid = text.len();
        buf.drain(..valid);
        if eof {
            break;
        }
    }
    Ok(Loaded { text: builder.build(), hash: hasher.finalize().to_hex().to_string(), stamp })
}

#[cfg(test)]
mod tests {
    use super::read;

    #[test]
    fn a_file_reads_into_the_same_text_and_hash_as_before() {
        let dir = crate::test_dir::tempdir();
        let path = dir.path().join("a.md");
        let body = "строка\r\n".repeat(700_000) + "конец\rхвост";
        std::fs::write(&path, &body).unwrap();
        let loaded = read(&path).unwrap();
        let text = aquilum_core::files::document::read_text(&path).unwrap();
        assert_eq!(loaded.text.to_string(), text);
        assert_eq!(loaded.hash, aquilum_core::files::document::hash_file(&path).unwrap());
    }
}
