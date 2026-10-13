use std::sync::Arc;

use masonry::app::RenderRoot;

use super::{options, parts};
use crate::ui::file_tree::FileTree;
use crate::ui::widgets::Slot;
use crate::ui::{ROOT_TAG, root, theme};

#[test]
fn root_can_be_rebuilt() {
    let _theme = theme::test_lock();
    let vault = crate::test_dir::tempdir();
    let data = crate::test_dir::tempdir();
    std::fs::write(vault.path().join("a.md"), "").unwrap();
    let mut tree = FileTree::new(Some(vault.path().to_path_buf()), data.path(), std::sync::Arc::new(|| {}));
    let (widget, mut chrome) = root(tree.widget(), parts("текст"));
    let slot = Slot::new(widget.erased()).with_tag(ROOT_TAG);
    let mut root_render = RenderRoot::new(slot.erased(), |_| {}, options(1.0, 1.0));
    let _ = root_render.redraw();
    for _ in 0..2 {
        let old = chrome.note_page;
        let (widget, next) = root(tree.widget(), parts("текст"));
        root_render.set_default_properties(Arc::new(theme::default_properties()));
        root_render.edit_widget_with_tag(ROOT_TAG, |mut slot| Slot::set_child(&mut slot, widget.erased()));
        let _ = root_render.redraw();
        assert!(old.get(&root_render).is_none(), "старый редактор удалён");
        chrome = next;
    }
    assert!(chrome.note_page.get(&root_render).is_some());
    for _ in 0..2 {
        tree.start_rename(&mut root_render, vault.path().join("a.md"));
        assert!(root_render.focused_widget().is_some(), "поле переименования в фокусе");
        tree.refresh(&mut root_render);
    }
}
