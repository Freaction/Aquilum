use super::cycle::Watch;
use super::remote::{Delta, Remote, RemoteFile, RemoteResult};
use crate::text::content::Content;
use std::io::{self, Write};
use std::ops::Range;

pub struct Halt<'a, T> {
    pub inner: T,
    pub watch: &'a dyn Watch,
}

impl<T> Halt<'_, T> {
    fn check(&self) -> io::Result<()> {
        if self.watch.stopped() { Err(io::Error::other("синхронизация остановлена")) } else { Ok(()) }
    }
}

impl Content for Halt<'_, &dyn Content> {
    fn byte_len(&self) -> usize {
        self.inner.byte_len()
    }

    fn read(&self, range: Range<usize>, visit: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        self.check()?;
        self.inner.read(range, visit)
    }
}

impl Write for Halt<'_, &mut dyn Write> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.check()?;
        self.inner.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Remote for Halt<'_, &dyn Remote> {
    fn cursor(&self) -> RemoteResult<String> {
        self.inner.cursor()
    }

    fn list(&self) -> RemoteResult<Vec<RemoteFile>> {
        self.inner.list()
    }

    fn delta(&self, cursor: &str) -> RemoteResult<Option<Delta>> {
        self.inner.delta(cursor)
    }

    fn download(&self, file: &RemoteFile, out: &mut dyn Write) -> RemoteResult<()> {
        self.inner.download(file, &mut Halt { inner: out, watch: self.watch })
    }

    fn upload(&self, path: &str, content: &dyn Content, modified: u64, replaces: Option<&RemoteFile>) -> RemoteResult<RemoteFile> {
        self.inner.upload(path, &Halt { inner: content, watch: self.watch }, modified, replaces)
    }

    fn delete(&self, file: &RemoteFile) -> RemoteResult<()> {
        self.inner.delete(file)
    }

    fn checksum(&self, content: &dyn Content) -> io::Result<String> {
        self.inner.checksum(content)
    }
}
