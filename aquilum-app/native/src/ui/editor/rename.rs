use masonry::app::RenderRoot;
use masonry::core::NewWidget;

use super::TextEditor;
use crate::ui::handle::Handle;
use crate::ui::style;
use crate::ui::tokens::size;

pub fn rename_editor(name: &str, class: &'static str) -> NewWidget<TextEditor> {
    NewWidget::new(TextEditor::single_line(name).renaming().styled(|| style::ui_text(size::SIZE_20 as f32))).with_class(class)
}

impl Handle<TextEditor> {
    pub fn focus_all(self, root: &mut RenderRoot) {
        self.edit(root, |mut field| {
            let len = field.widget.len();
            TextEditor::select_byte_range(&mut field, 0, len);
        });
        root.focus_on(Some(self.id()));
    }
}
