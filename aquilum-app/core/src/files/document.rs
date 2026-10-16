use super::error::FileCommandError;
use super::models::{FileSnapshot, FileWriteResult};
use atomic_write_file::AtomicWriteFile;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn read_hash_if_exists(path: &Path) -> Result<Option<String>, FileCommandError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(hash_bytes(&bytes))),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
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

pub fn text_from_bytes(mut bytes: Vec<u8>) -> std::io::Result<String> {
    let kept = normalize_line_endings_in_place(&mut bytes);
    bytes.truncate(kept);
    String::from_utf8(bytes).map_err(|error| std::io::Error::new(ErrorKind::InvalidData, error))
}

pub fn normalize_line_endings(content: String) -> String {
    text_from_bytes(content.into_bytes()).expect("line endings are ASCII, so the text stays UTF-8")
}

pub fn read_text(path: &Path) -> std::io::Result<String> {
    text_from_bytes(fs::read(path)?)
}

fn snapshot_of(bytes: Vec<u8>) -> Result<FileSnapshot, FileCommandError> {
    let hash = hash_bytes(&bytes);
    let content = text_from_bytes(bytes).map_err(|error| FileCommandError::InvalidUtf8 {
        message: error.to_string(),
    })?;
    let text_hash = hash_bytes(content.as_bytes());

    Ok(FileSnapshot {
        content,
        hash,
        text_hash,
    })
}

pub fn read_file_snapshot_impl(path: &Path) -> Result<FileSnapshot, FileCommandError> {
    snapshot_of(fs::read(path)?)
}

pub fn read_file_hash_impl(path: &Path) -> Result<String, FileCommandError> {
    Ok(hash_bytes(&fs::read(path)?))
}

pub fn read_file_stat_impl(
    path: &Path,
) -> Result<super::models::FileStat, FileCommandError> {
    let meta = fs::metadata(path)?;
    if !meta.is_file() {
        return Err(FileCommandError::Io {
            message: format!("not a file: {}", path.display()),
        });
    }
    Ok(super::models::FileStat {
        byte_length: meta.len(),
    })
}

fn check_expected_hash(path: &Path, expected_hash: &str) -> Result<(), FileCommandError> {
    let actual_hash = read_hash_if_exists(path)?;
    if actual_hash.as_deref() != Some(expected_hash) {
        return Err(FileCommandError::Conflict {
            expected_hash: expected_hash.to_owned(),
            actual_hash,
        });
    }
    Ok(())
}

pub fn write_file_atomic_impl(
    path: &Path,
    content: &str,
    expected_hash: Option<&str>,
) -> Result<FileWriteResult, FileCommandError> {
    let bytes = content.as_bytes();
    let hash = hash_bytes(bytes);
    let mut file = AtomicWriteFile::open(path)?;
    file.write_all(bytes)?;

    if let Some(expected_hash) = expected_hash {
        check_expected_hash(path, expected_hash)?;
    }

    file.commit()?;
    Ok(FileWriteResult { hash })
}

pub fn ensure_directory_impl(path: &Path) -> Result<(), FileCommandError> {
    fs::create_dir_all(path)?;
    Ok(())
}
