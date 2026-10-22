use masonry::core::{NewWidget, PropertySet};
use masonry::layout::Length;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::Scene;
use crate::ui::icons;
use crate::ui::snapshot::open;
use crate::ui::tokens::size;
use crate::ui::widgets::{ButtonSize, TextButton, Variant};

pub(super) fn snapshot(scene: &Scene) {
    let gap = || PropertySet::new().with(Gap::new(Length::px(size::SPACE_8)));
    let mut sheet = Flex::column().cross_axis_alignment(masonry::properties::types::CrossAxisAlignment::Start);
    for button in [ButtonSize::M, ButtonSize::S, ButtonSize::Xs] {
        for icons in [false, true] {
            let mut row = Flex::row();
            for variant in [Variant::Primary, Variant::Secondary, Variant::Ghost, Variant::Link] {
                for pressed in [false, true] {
                    let widget = TextButton::new("Button", variant).size(button).pressed(pressed);
                    let widget = if icons { widget.icon_before(icons::FOLDER).icon_after(icons::ARROW_UP_RIGHT) } else { widget };
                    row = row.with_fixed(NewWidget::new(widget));
                }
            }
            sheet = sheet.with_fixed(NewWidget::new(row).with_props(gap()));
        }
    }
    let sheet = NewWidget::new(sheet).with_props(gap().with(Padding::all(Length::px(size::SPACE_20))));
    let mut root = open(sheet.erased(), 1.0, 1.0);
    scene.save(&mut root, "buttons");
}
