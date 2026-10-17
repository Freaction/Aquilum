use crate::text::content::{Content, BLOCK};
use std::fs;
use std::path::Path;

pub struct FileContent {
    file: fs::File,
    len: usize,
}

impl FileContent {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = fs::File::open(path)?;
        let len = file.metadata()?.len() as usize;
        Ok(Self { file, len })
    }
}

impl Content for FileContent {
    fn byte_len(&self) -> usize {
        self.len
    }

    fn read(&self, range: std::ops::Range<usize>, visit: &mut dyn FnMut(&[u8]) -> std::io::Result<()>) -> std::io::Result<()> {
        let mut buf = vec![0u8; BLOCK.min(range.len())];
        for from in range.clone().step_by(BLOCK) {
            let block = &mut buf[..BLOCK.min(range.end - from)];
            read_at(&self.file, block, from as u64)?;
            visit(block)?;
        }
        Ok(())
    }
}

#[cfg(unix)]
fn read_at(file: &fs::File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
}

#[cfg(windows)]
fn read_at(file: &fs::File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
    while !buf.is_empty() {
        match std::os::windows::fs::FileExt::seek_read(file, buf, offset)? {
            0 => return Err(std::io::ErrorKind::UnexpectedEof.into()),
            read => {
                buf = &mut buf[read..];
                offset += read as u64;
            }
        }
    }
    Ok(())
}
