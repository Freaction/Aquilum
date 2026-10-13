use std::path::{Path, PathBuf};

use masonry::core::WidgetId;

use super::FileTree;

impl FileTree {
    pub fn built_rows(&self) -> std::ops::Range<usize> {
        self.first..self.first + self.row_ids.len()
    }

    pub fn scroll_id(&self) -> Option<WidgetId> {
        self.scroll.map(|s| s.id())
    }

    pub fn toolbar_ids(&self) -> Option<[WidgetId; 4]> {
        self.toolbar.as_ref().map(|t| [t.new_note, t.new_folder, t.sort, t.collapse])
    }

    pub fn row_id(&self, name: &str) -> Option<WidgetId> {
        let index = self.model.rows().iter().position(|r| r.name == name)?;
        self.row_ids.get(index.checked_sub(self.first)?).copied()
    }

    pub fn items(&self, name: &str) -> Option<u32> {
        self.model.rows().iter().find(|r| r.name == name)?.items
    }

    pub fn counted(&self) -> bool {
        self.counting.is_none()
    }

    pub fn offset(&self) -> f64 {
        self.offset
    }

    pub fn pinned_names(&self, root: &masonry::app::RenderRoot) -> Vec<String> {
        self.pinned.and_then(|pinned| pinned.get(root)).map(|pinned| pinned.inner().names()).unwrap_or_default()
    }

    pub fn names(&self) -> Vec<String> {
        self.model.rows().iter().map(|r| r.name.clone()).collect()
    }

    pub fn renaming(&self) -> Option<PathBuf> {
        self.model.renaming().map(Path::to_path_buf)
    }

    pub fn rename_field(&self) -> Option<WidgetId> {
        self.rename.map(|field| field.id())
    }
}
