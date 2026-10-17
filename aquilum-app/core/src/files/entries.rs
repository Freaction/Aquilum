use super::error::FileCommandError;
use super::models::FileWriteResult;
use crate::search::paths::same_path;
use std::fs;
use crate::text::content::{Content, HashedWriter};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const FREE_NAME_ATTEMPTS: usize = 1000;

pub fn first_free<T>(candidate: impl Fn(usize) -> PathBuf, mut op: impl FnMut(&Path) -> Result<T, FileCommandError>) -> Result<PathBuf, FileCommandError> {
    for index in 0..FREE_NAME_ATTEMPTS {
        let path = candidate(index);
        match op(&path) {
            Ok(_) => return Ok(path),
            Err(FileCommandError::AlreadyExists { .. }) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(taken(&candidate(0)))
}

fn taken(path: &Path) -> FileCommandError {
    FileCommandError::AlreadyExists { path: path.to_string_lossy().into_owned() }
}

fn create_new(path: &Path) -> Result<fs::File, FileCommandError> {
    fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|error| match error.kind() {
        ErrorKind::AlreadyExists => taken(path),
        _ => error.into(),
    })
}

pub fn create_content_file(path: &Path, content: &dyn Content) -> Result<FileWriteResult, FileCommandError> {
    let file = create_new(path)?;
    let mut out = HashedWriter::new(&file);
    let written = out.put(content).and_then(|()| out.finish());
    match written.and_then(|hash| file.sync_all().map(|()| hash)) {
        Ok(hash) => Ok(FileWriteResult { hash }),
        Err(error) => {
            drop(file);
            let _ = fs::remove_file(path);
            Err(error.into())
        }
    }
}

pub fn copy_file_impl(from: &Path, to: &Path) -> Result<(), FileCommandError> {
    if to.exists() {
        return Err(taken(to));
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
            return Err(taken(new_path));
        }
        fs::rename(old_path, new_path)?;
        return Ok(());
    }

    match fs::hard_link(old_path, new_path) {
        Ok(()) => return remove_renamed_source(old_path, new_path),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => return Err(taken(new_path)),
        Err(_) => {}
    }

    let mut source = fs::File::open(old_path)?;
    let mut destination = create_new(new_path)?;

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
