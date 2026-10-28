use std::ops::Range;

use masonry::app::RenderRoot;
use masonry::core::{CollectionWidget, NewWidget, WidgetMut};
use masonry::layout::Length;
use masonry::widgets::{Flex, SizedBox};

use super::row::TreeRow;
use super::{FileTree, SPAN, step};
use crate::ui::scroll::ScrollArea;
use crate::ui::tokens::size;

fn height(rows: usize) -> Length {
    Length::px(rows as f64 * step() - size::SPACE_2)
}

pub(super) fn spacer(rows: usize) -> NewWidget<SizedBox> {
    NewWidget::new(SizedBox::empty().height(height(rows)))
}

fn edge(list: &mut WidgetMut<'_, Flex>, at: usize, old: usize, new: usize) {
    match (old > 0, new > 0) {
        (true, true) => SizedBox::set_height(&mut Flex::get_mut(list, at).downcast(), height(new)),
        (true, false) => Flex::remove(list, at),
        (false, true) => Flex::insert_fixed(list, at, spacer(new)),
        (false, false) => {}
    }
}

impl FileTree {
    pub(super) fn slide(&mut self, root: &mut RenderRoot, first: usize) {
        let count = self.model.rows().len();
        let span = |from: usize| from..(from + SPAN).min(count);
        let (old, new) = (span(self.first), span(first));
        let kept = old.start.max(new.start)..old.end.min(new.end);
        self.first = first;
        let scroll = self.scroll.filter(|_| self.model.renaming().is_none() && !kept.is_empty() && self.row_ids.len() == old.len());
        let Some(scroll) = scroll else { return self.refresh(root) };
        let rows = |range: Range<usize>| self.model.rows()[range].iter().map(|row| NewWidget::new(TreeRow::new(row.clone()))).collect::<Vec<_>>();
        let (above, below) = (rows(new.start..kept.start), rows(kept.end..new.end));
        let stay = self.row_ids[kept.start - old.start..kept.end - old.start].iter().copied();
        self.row_ids = above.iter().map(NewWidget::id).chain(stay).chain(below.iter().map(NewWidget::id)).collect();
        scroll.edit(root, |mut area| {
            let mut list = ScrollArea::child_mut(&mut area);
            let top = usize::from(old.start > 0);
            for _ in old.start..kept.start {
                Flex::remove(&mut list, top);
            }
            for _ in kept.end..old.end {
                Flex::remove(&mut list, top + kept.len());
            }
            for (at, row) in above.into_iter().enumerate() {
                Flex::insert_fixed(&mut list, top + at, row);
            }
            for (at, row) in below.into_iter().enumerate() {
                Flex::insert_fixed(&mut list, top + kept.end - new.start + at, row);
            }
            edge(&mut list, 0, old.start, new.start);
            edge(&mut list, usize::from(new.start > 0) + new.len(), count - old.end, count - new.end);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use masonry::app::RenderRoot;
    use masonry::core::WidgetId;

    use super::super::FileTree;
    use super::super::row::TreeRow;

    fn list(root: &mut RenderRoot, tree: &FileTree) -> (f64, Vec<WidgetId>) {
        let _ = root.redraw();
        let scroll = tree.scroll.and_then(|scroll| scroll.get(root)).expect("прокрутка дерева");
        let list = scroll.children().into_iter().next().expect("список строк");
        (list.ctx().border_box().height(), list.children().iter().map(|child| child.id()).collect())
    }

    #[test]
    fn sliding_the_window_keeps_rows_in_order_and_the_list_height() {
        let vault = crate::test_dir::tempdir();
        let data = crate::test_dir::tempdir();
        for i in 0..400 {
            std::fs::write(vault.path().join(format!("{i:04}.md")), "").unwrap();
        }
        let mut tree = FileTree::new(Some(vault.path().to_path_buf()), data.path(), Arc::new(|| {}));
        if let Some(counting) = tree.counting.take() {
            let _ = counting.recv();
        }
        let mut root = RenderRoot::new(tree.widget().erased(), |_| {}, crate::test_dir::options(300, 600, 1.0));
        let (height, _) = list(&mut root, &tree);
        for first in [30, 90, 60, 0, 200, 170, 240, 0] {
            tree.slide(&mut root, first);
            let shown: Vec<String> = tree.row_ids.iter().filter_map(|&id| Some(root.get_widget(id)?.downcast::<TreeRow>()?.inner().name().to_owned())).collect();
            assert_eq!(shown, tree.names()[tree.built_rows()], "строки окна с {first}");
            let (now, children) = list(&mut root, &tree);
            let rows: Vec<WidgetId> = children.iter().copied().filter(|id| tree.row_ids.contains(id)).collect();
            assert_eq!(rows, tree.row_ids, "порядок строк окна с {first}");
            assert_eq!(children.len() - rows.len(), usize::from(first > 0) + usize::from(tree.built_rows().end < 400));
            assert!((now - height).abs() < 0.01, "высота списка {now} вместо {height}");
        }
    }
}
