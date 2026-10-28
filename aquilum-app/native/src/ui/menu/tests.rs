use masonry::app::RenderRoot;
use masonry::core::{Widget, WidgetRef};

use super::{Entry, Menu};
use crate::ui::tokens::size;
use crate::ui::widgets::Rule;

fn rows(divided: bool) -> Vec<(f64, bool)> {
    let entries = [
        Entry { label: "Отменить", icon: None, disabled: false, divided: false },
        Entry { label: "Копировать", icon: None, disabled: false, divided },
    ];
    let mut root = RenderRoot::new(Menu::entries("меню", &entries).erased(), |_| {}, crate::test_dir::options(300, 300, 1.0));
    let _ = root.redraw();
    let mut list: WidgetRef<'_, dyn Widget> = root.get_layer_root(0);
    while list.children().len() == 1 {
        list = list.children()[0];
    }
    list.children().iter().map(|child| (child.ctx().bounding_box().y0, child.downcast::<Rule>().is_some())).collect()
}

#[test]
fn a_divider_separates_groups_by_its_own_height() {
    let plain = rows(false);
    let divided = rows(true);
    assert_eq!(divided.iter().map(|&(_, rule)| rule).collect::<Vec<_>>(), [false, true, false], "линия между пунктами");
    let shift = divided[2].0 - plain[1].0;
    assert!((shift - (size::BORDER_1 + 2.0 * size::SPACE_4 + size::GAP_XS)).abs() < 0.5, "второй пункт сдвинут на {shift}");
}
