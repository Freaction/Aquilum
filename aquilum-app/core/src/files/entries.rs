use super::document::hash_bytes;
use super::error::FileCommandError;
use super::models::FileWriteResult;
use crate::search::paths::same_path;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

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

pub fn create_file_impl(
    path: &Path,
    content: &str,
) -> Result<FileWriteResult, FileCommandError> {
    create_binary_file_impl(path, content.as_bytes())
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
