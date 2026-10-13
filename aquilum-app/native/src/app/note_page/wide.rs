use masonry::app::RenderRoot;

use super::super::App;
use crate::ui::note::EditorColumn;

impl App {
    pub fn toggle_wide_mode(&mut self, root: &mut RenderRoot) {
        let wide = !self.interface.wide();
        self.interface.set_wide(wide);
        if let Some(ids) = &self.note_ids {
            ids.column.edit(root, |mut column| EditorColumn::set_wide(&mut column, wide));
        }
    }
}
