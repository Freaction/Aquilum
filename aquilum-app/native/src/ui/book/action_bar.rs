use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoverAction {
    Add,
    Replace,
    Random,
    Reposition,
    Remove,
    UploadBook,
}

impl CoverAction {
    pub fn list(image: Option<bool>, book: bool) -> Vec<CoverAction> {
        use CoverAction::*;
        [Add, Replace, Random, Reposition, Remove, UploadBook]
            .into_iter()
            .filter(|&action| match action {
                Add => image.is_none(),
                Replace | Remove => image.is_some(),
                Reposition => image == Some(true),
                Random => true,
                UploadBook => book,
            })
            .collect()
    }

    pub fn entry(self) -> (String, icons::Icon) {
        match self {
            CoverAction::Add => (t("book.addPageCover"), icons::UPLOAD),
            CoverAction::Replace => (t("book.replacePageCover"), icons::UPLOAD),
            CoverAction::Random => (t("book.randomCover"), icons::SHUFFLE),
            CoverAction::Reposition => (t("book.coverPosition"), icons::UNFOLD_VERTICAL),
            CoverAction::Remove => (t("book.removePageCover"), icons::TRASH),
            CoverAction::UploadBook => (t("book.uploadBookCover"), icons::BOOK_OPEN),
        }
    }
}

pub struct ActionBar {
    row: WidgetPod<Flex>,
}

impl ActionBar {
    pub(super) fn new(actions: &[CoverAction], ids: &mut HeroIds) -> NewWidget<Self> {
        let mut row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
        for (index, &action) in actions.iter().enumerate() {
            let (label, icon) = action.entry();
            let reposition = action == CoverAction::Reposition;
            let button = NewWidget::new(if reposition { IconButton::mode(icon, label, false) } else { IconButton::new(icon, label) });
            if reposition {
                ids.reposition = Some(Handle::of(&button));
            }
            ids.actions.push((button.id(), action));
            if index > 0 {
                row = row.with_fixed(crate::ui::widgets::Rule::new(Axis::Vertical, || theme::current().book_actions_divider));
            }
            row = row.with_fixed(button);
        }
        let row = NewWidget::new(row).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4))));
        NewWidget::new(ActionBar { row: row.to_pod() })
    }
}

impl Widget for ActionBar {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.row);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = ctx.context_size();
        let inner = ctx.compute_length(&mut self.row, len_req.into(), context, axis, cross);
        Length::px(inner.get() + size::SPACE_3 * 2.0)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let inner = Size::new(size_.width - size::SPACE_3 * 2.0, size_.height - size::SPACE_3 * 2.0);
        ctx.run_layout(&mut self.row, inner);
        ctx.place_child(&mut self.row, Point::new(size::SPACE_3, size::SPACE_3));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let shape = RoundedRect::from_rect(ctx.content_box(), size::ROUNDED_XL);
        box_shadow(painter, shape, tm.shadow_action);
        painter.fill(shape, tm.book_actions_bg).draw();
        let inner = RoundedRect::from_rect(ctx.content_box().inset(-0.5), size::ROUNDED_XL - 0.5);
        painter.stroke(&inner, &Stroke::new(size::BORDER_1), tm.book_actions_border).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::Toolbar
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.row.id()])
    }
}
