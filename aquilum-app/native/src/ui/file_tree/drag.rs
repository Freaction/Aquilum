use std::path::PathBuf;

use masonry::app::RenderRoot;
use masonry::core::WidgetId;
use masonry::kurbo::{Point, Vec2};

use super::row::TreeRow;
use super::{FileTree, TreeAction, ghost};

pub(super) struct Drag {
    source: PathBuf,
    sources: Vec<PathBuf>,
    ghost: WidgetId,
    over: Option<(WidgetId, PathBuf)>,
}

impl FileTree {
    pub(super) fn start_drag(&mut self, root: &mut RenderRoot, index: usize, at: Point) {
        let Some(row) = self.model.rows().get(index) else { return };
        let source = row.path.clone();
        let sources = self.model.drag_sources(index);
        let label = if sources.len() > 1 { crate::i18n::plural("fileTree.dragCount", sources.len() as u64) } else { row.name.clone() };
        let ghost = ghost::DragGhost::new(&label);
        let id = ghost.id();
        root.add_layer(ghost, at + Vec2::new(ghost::OFFSET, ghost::OFFSET));
        self.drag = Some(Drag { source, sources, ghost: id, over: None });
    }

    pub(super) fn drag_to(&mut self, root: &mut RenderRoot, at: Point) {
        let Some(drag) = &mut self.drag else { return };
        root.reposition_layer(drag.ghost, at + Vec2::new(ghost::OFFSET, ghost::OFFSET));
        let under = root.get_layer_root(0).find_widget_under_pointer(at).map(|w| w.id());
        let target = under
            .and_then(|id| self.row_ids.iter().position(|&row| row == id))
            .and_then(|index| Some((self.row_ids[index], self.model.drop_target(&drag.source, self.first + index)?)));
        let previous = drag.over.as_ref().map(|(id, _)| *id);
        let next = target.as_ref().map(|(id, _)| *id);
        if previous != next {
            for (id, on) in [(previous, false), (next, true)] {
                if let Some(id) = id {
                    root.edit_widget(id, |mut w| TreeRow::set_drop_over(&mut w.downcast::<TreeRow>(), on));
                }
            }
        }
        drag.over = target;
    }

    pub(super) fn drop(&mut self, root: &mut RenderRoot) -> Option<TreeAction> {
        let drag = self.drag.take()?;
        root.remove_layer(drag.ghost);
        let (row, target) = drag.over?;
        root.edit_widget(row, |mut w| TreeRow::set_drop_over(&mut w.downcast::<TreeRow>(), false));
        self.model.moved(&drag.sources, &target);
        self.save_expanded();
        self.save_colors();
        Some(TreeAction::Move { sources: drag.sources, target })
    }
}
