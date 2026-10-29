use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aquilum_core::files::models::{FileItem, FileItemType};
use aquilum_core::files::workspace::read_directory_impl;

use super::comparable;

pub type Lists = HashMap<String, Vec<FileItem>>;

pub fn walk(root: &Path, mut visit: impl FnMut(String, Vec<FileItem>)) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let items = read_directory_impl(&dir).unwrap_or_default();
        pending.extend(items.iter().filter(|item| item.item_type == FileItemType::Folder).map(|item| PathBuf::from(&item.id)));
        visit(comparable(&dir), items);
    }
}

pub fn index(root: &Path) -> Lists {
    let mut lists = Lists::new();
    walk(root, |key, items| {
        lists.insert(key, items);
    });
    lists
}
