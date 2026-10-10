use super::error::FileCommandError;
use super::models::{FileSnapshot, FileWriteResult};
use crate::search::paths::same_path;
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

pub fn normalize_line_endings(content: String) -> String {
    if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content
    }
}

pub fn read_text(path: &Path) -> std::io::Result<String> {
    fs::read_to_string(path).map(normalize_line_endings)
}

fn snapshot_of(bytes: Vec<u8>) -> Result<FileSnapshot, FileCommandError> {
    let mut snapshot = raw_snapshot_of(bytes)?;
    snapshot.content = normalize_line_endings(snapshot.content);
    Ok(snapshot)
}

fn raw_snapshot_of(bytes: Vec<u8>) -> Result<FileSnapshot, FileCommandError> {
    let hash = hash_bytes(&bytes);
    let raw = String::from_utf8(bytes).map_err(|error| FileCommandError::InvalidUtf8 {
        message: error.to_string(),
    })?;
    let text_hash = hash_bytes(normalize_line_endings(raw.clone()).as_bytes());

    Ok(FileSnapshot {
        content: raw,
        hash,
        text_hash,
    })
}

pub fn read_file_snapshot_impl(path: &Path) -> Result<FileSnapshot, FileCommandError> {
    snapshot_of(fs::read(path)?)
}

pub(crate) fn read_raw_file_snapshot_impl(path: &Path) -> Result<FileSnapshot, FileCommandError> {
    raw_snapshot_of(fs::read(path)?)
}

pub fn read_file_hash_impl(path: &Path) -> Result<String, FileCommandError> {
    Ok(hash_bytes(&fs::read(path)?))
}

pub fn read_file_stat_impl(path: &Path) -> Result<super::models::FileStat, FileCommandError> {
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

pub fn create_binary_file_impl(
    path: &Path,
    bytes: &[u8],
) -> Result<FileWriteResult, FileCommandError> {
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Err(FileCommandError::AlreadyExists {
                path: path.to_string_lossy().into_owned(),
            });
        }
        Err(error) => return Err(error.into()),
    };

    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error.into());
    }

    Ok(FileWriteResult {
        hash: hash_bytes(bytes),
    })
}

pub fn create_file_impl(path: &Path, content: &str) -> Result<FileWriteResult, FileCommandError> {
    create_binary_file_impl(path, content.as_bytes())
}

pub fn ensure_directory_impl(path: &Path) -> Result<(), FileCommandError> {
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn copy_file_impl(from: &Path, to: &Path) -> Result<(), FileCommandError> {
    if to.exists() {
        return Err(FileCommandError::AlreadyExists {
            path: to.to_string_lossy().into_owned(),
        });
    }
    if from.is_dir() {
        return copy_directory_impl(from, to);
    }
    fs::copy(from, to)?;
    Ok(())
}

fn copy_directory_impl(from: &Path, to: &Path) -> Result<(), FileCommandError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory_impl(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub fn rename_file_impl(old_path: &Path, new_path: &Path) -> Result<(), FileCommandError> {
    if old_path == new_path {
        return Ok(());
    }

    if same_path(old_path, new_path) {
        fs::rename(old_path, new_path)?;
        return Ok(());
    }

    if old_path.is_dir() {
        if new_path.exists() {
            return Err(FileCommandError::AlreadyExists {
                path: new_path.to_string_lossy().into_owned(),
            });
        }
        fs::rename(old_path, new_path)?;
        return Ok(());
    }

    match fs::hard_link(old_path, new_path) {
        Ok(()) => return remove_renamed_source(old_path, new_path),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Err(FileCommandError::AlreadyExists {
                path: new_path.to_string_lossy().into_owned(),
            });
        }
        Err(_) => {}
    }

    let mut source = fs::File::open(old_path)?;
    let mut destination = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(new_path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Err(FileCommandError::AlreadyExists {
                path: new_path.to_string_lossy().into_owned(),
            });
        }
        Err(error) => return Err(error.into()),
    };

    if let Err(error) = std::io::copy(&mut source, &mut destination)
        .and_then(|_| destination.sync_all())
        .and_then(|_| fs::remove_file(old_path))
    {
        drop(destination);
        let _ = fs::remove_file(new_path);
        return Err(error.into());
    }

    Ok(())
}

fn remove_renamed_source(old_path: &Path, new_path: &Path) -> Result<(), FileCommandError> {
    if let Err(error) = fs::remove_file(old_path) {
        let _ = fs::remove_file(new_path);
        return Err(error.into());
    }
    Ok(())
}
