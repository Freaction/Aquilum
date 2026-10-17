use super::delta::{self, EXTENSION};
use crate::files::content::FileContent;
use crate::text::content::Content;
use crate::text::pieces::Pieces;
use std::io;
use std::path::Path;

pub type Rebuilt = Pieces<FileContent>;

pub fn is_delta(id: &str) -> bool {
    id.ends_with(EXTENSION)
}

pub fn rebuild(dir: &Path, id: &str) -> io::Result<Rebuilt> {
    let mut chain = vec![id.to_owned()];
    while is_delta(chain.last().map_or("", String::as_str)) {
        let delta = delta::read(&dir.join(chain.last().map_or("", String::as_str)))?;
        let Some(base) = delta.base else { break };
        if !dir.join(&base).is_file() || chain.contains(&base) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "нет версии, от которой записаны правки"));
        }
        chain.push(base);
    }
    let mut rebuilt = Pieces::new(Vec::new());
    for id in chain.iter().rev() {
        let path = dir.join(id);
        let file = FileContent::open(&path)?;
        let len = file.byte_len();
        let file = rebuilt.source(file);
        if !is_delta(id) {
            rebuilt.take();
            rebuilt.push(file, 0..len);
            continue;
        }
        let delta = delta::read(&path)?;
        let previous = rebuilt.take();
        let (mut cursor, mut offset) = (0, delta.body as usize);
        for region in &delta.regions {
            previous.copy(cursor..region.from, &mut rebuilt);
            rebuilt.push(file, offset..offset + region.len);
            offset += region.len;
            cursor = region.to;
        }
        previous.copy(cursor..previous.byte_len(), &mut rebuilt);
    }
    Ok(rebuilt)
}
