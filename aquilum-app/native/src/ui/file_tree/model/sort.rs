use std::cmp::Reverse;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use aquilum_core::files::models::{FileItem, FileItemType};
use serde::{Deserialize, Serialize};

use super::{TreeModel, comparable};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    #[default]
    NameAsc,
    NameDesc,
    ModifiedNew,
    ModifiedOld,
    CreatedNew,
    CreatedOld,
}

impl Sort {
    pub const ALL: [Sort; 6] = [Sort::NameAsc, Sort::NameDesc, Sort::ModifiedNew, Sort::ModifiedOld, Sort::CreatedNew, Sort::CreatedOld];

    pub fn label(self) -> &'static str {
        match self {
            Sort::NameAsc => "fileTree.sortNameAsc",
            Sort::NameDesc => "fileTree.sortNameDesc",
            Sort::ModifiedNew => "fileTree.sortModifiedNew",
            Sort::ModifiedOld => "fileTree.sortModifiedOld",
            Sort::CreatedNew => "fileTree.sortCreatedNew",
            Sort::CreatedOld => "fileTree.sortCreatedOld",
        }
    }

    pub(super) fn by_time(self) -> bool {
        !matches!(self, Sort::NameAsc | Sort::NameDesc)
    }
}

impl TreeModel {
    pub fn set_sort(&mut self, sort: Sort) -> bool {
        if self.sort == sort {
            return false;
        }
        self.sort = sort;
        let mut directories = std::mem::take(&mut self.directories);
        if sort.by_time() {
            for items in directories.values() {
                self.stamp(items);
            }
        } else {
            self.times.clear();
        }
        for items in directories.values_mut() {
            self.order(items);
        }
        self.directories = directories;
        self.rebuild();
        true
    }

    pub(super) fn stamp(&mut self, items: &[FileItem]) {
        let nanos = |time: SystemTime| time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as i128);
        for item in items.iter().filter(|i| i.item_type == FileItemType::File) {
            let Ok(meta) = std::fs::metadata(&item.id) else { continue };
            let modified = meta.modified().map_or(0, nanos);
            let created = meta.created().map_or(modified, nanos);
            self.times.insert(comparable(Path::new(&item.id)), (modified, created));
        }
    }

    pub(super) fn order(&self, items: &mut [FileItem]) {
        let file = |i: &FileItem| i.item_type == FileItemType::File;
        let name = |i: &FileItem| i.name.to_lowercase();
        match self.sort {
            Sort::NameAsc => items.sort_by_cached_key(|i| (file(i), name(i))),
            Sort::NameDesc => items.sort_by_cached_key(|i| (file(i), Reverse(name(i)))),
            sort => {
                let newest = matches!(sort, Sort::ModifiedNew | Sort::CreatedNew);
                let created = matches!(sort, Sort::CreatedNew | Sort::CreatedOld);
                items.sort_by_cached_key(|i| {
                    let time = self.times.get(&comparable(Path::new(&i.id))).map_or(0, |t| if created { t.1 } else { t.0 });
                    let time = if !file(i) { 0 } else if newest { -time } else { time };
                    (file(i), time, name(i))
                });
            }
        }
    }
}
