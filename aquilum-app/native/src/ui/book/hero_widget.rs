use super::*;

pub fn hero(view: HeroView) -> (NewWidget<Hero>, HeroIds) {
    let mut ids = HeroIds {
        cover: None,
        replace: None,
        random: None,
        reposition: None,
        remove: None,
        thumbnail: None,
        replace_book: None,
        book_button: None,
    };
    let mut button = view.book.as_ref().map(|book| {
        let label = if book.has_file { t("book.read") } else { t("book.upload") };
        let widget = NewWidget::new(TextButton::new(label, Variant::Primary).size(ButtonSize::M));
        ids.book_button = Some(Handle::of(&widget));
        widget
    });
    let cover = view.cover.map(|visual| {
        let replace = NewWidget::new(IconButton::new(icons::UPLOAD, t("book.replacePageCover")));
        let random = NewWidget::new(IconButton::new(icons::SHUFFLE, t("book.randomCover")));
        let remove = NewWidget::new(IconButton::new(icons::TRASH, t("book.removePageCover")));
        ids.replace = Some(replace.id());
        ids.random = Some(random.id());
        ids.remove = Some(remove.id());
        let divider = || crate::ui::widgets::Rule::new(Axis::Vertical, || theme::current().book_actions_divider);
        let mut bar = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(replace).with_fixed(divider()).with_fixed(random);
        if matches!(visual, Visual::Image(_)) {
            let reposition = NewWidget::new(IconButton::mode(icons::UNFOLD_VERTICAL, t("book.coverPosition"), false));
            ids.reposition = Some(Handle::of(&reposition));
            bar = bar.with_fixed(divider()).with_fixed(reposition);
        }
        bar = bar.with_fixed(divider()).with_fixed(remove);
        let bar = NewWidget::new(ActionBar { row: NewWidget::new(bar).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4)))).to_pod() });
        let widget = PageCover::new(visual, view.position, bar, button.take());
        ids.cover = Some(Handle::of(&widget));
        widget
    });
    let aspect = view.book.as_ref().map_or(2.0 / 3.0, |b| {
        let (w, h) = (f64::from(b.cover.image.width), f64::from(b.cover.image.height));
        if h > 0.0 { w / h } else { 2.0 / 3.0 }
    });
    let thumbnail = view.book.map(|book| {
        let replace = NewWidget::new(IconButton::white(icons::UPLOAD, t("book.replaceBookCover")));
        ids.replace_book = Some(replace.id());
        let widget = NewWidget::new(Thumbnail { image: book.cover, height: 0.0, replace: crate::ui::widgets::Appear::new(replace, crate::ui::widgets::FAST).to_pod(), hovered: false, baked: None });
        ids.thumbnail = Some(Handle::of(&widget));
        widget
    });
    let star = thumbnail.as_ref().map(|_| NewWidget::new(Star));
    let hero = NewWidget::new(Hero {
        viewport: view.viewport,
        column: None,
        aspect,
        cover: cover.map(NewWidget::to_pod),
        star: star.map(NewWidget::to_pod),
        thumbnail: thumbnail.map(NewWidget::to_pod),
        solo: button.map(NewWidget::to_pod),
    });
    (hero, ids)
}

pub struct Hero {
    viewport: f64,
    column: Option<f64>,
    aspect: f64,
    cover: Option<WidgetPod<PageCover>>,
    star: Option<WidgetPod<Star>>,
    thumbnail: Option<WidgetPod<Thumbnail>>,
    solo: Option<WidgetPod<TextButton>>,
}

impl Hero {
    pub fn set_viewport(this: &mut WidgetMut<'_, Self>, viewport: f64) {
        if (this.widget.viewport - viewport).abs() > 0.5 {
            this.widget.viewport = viewport;
            this.ctx.request_layout();
        }
    }

    fn height(&self) -> f64 {
        let cover = cover_height(self.viewport);
        if self.thumbnail.is_some() {
            CONTENT_GAP + thumb_height(cover)
        } else {
            size::BOOK_GUTTER + cover
        }
    }
}

impl Widget for Hero {
    type Action = masonry::core::NoAction;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FontsChanged) {
            self.column = None;
            ctx.request_layout();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(c) = &mut self.cover {
            ctx.register_child(c);
        }
        if let Some(c) = &mut self.star {
            ctx.register_child(c);
        }
        if let Some(c) = &mut self.thumbnail {
            ctx.register_child(c);
        }
        if let Some(c) = &mut self.solo {
            ctx.register_child(c);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        if self.column.is_none() {
            self.column = Some(crate::ui::note::ch_width(ctx));
        }
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(self.height()),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::ZERO,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let gutter = size::BOOK_GUTTER;
        let cover_h = cover_height(self.viewport);
        if let Some(cover) = &mut self.cover {
            let rect = Size::new((size_.width - gutter * 2.0).max(0.0), cover_h);
            ctx.run_layout(cover, rect);
            ctx.place_child(cover, Point::new(gutter, gutter));
        }
        let thumb_h = thumb_height(cover_h);
        if let Some(thumbnail) = &mut self.thumbnail {
            let column = self.column.unwrap_or(size_.width).min(size_.width);
            let left = ((size_.width - column) / 2.0).round() + size::EDITOR_PADDING_X;
            let width = (thumb_h * self.aspect).round();
            ctx.run_layout(thumbnail, Size::new(width, thumb_h));
            ctx.place_child(thumbnail, Point::new(left, CONTENT_GAP));
            if let Some(star) = &mut self.star {
                let star_size = Size::new(thumb_h * 96.0 / 186.0, thumb_h * 63.0 / 186.0);
                ctx.run_layout(star, star_size);
                let right = left + width + thumb_h * 58.0 / 186.0;
                ctx.place_child(star, Point::new(right - star_size.width, CONTENT_GAP - thumb_h * 24.0 / 186.0));
            }
        }
        if let Some(solo) = &mut self.solo {
            let auto = SizeDef::new(LenDef::MaxContent, LenDef::MaxContent);
            let button = ctx.compute_size(solo, auto, size_.into());
            ctx.run_layout(solo, button);
            ctx.place_child(solo, Point::new(size_.width - gutter - button.width, gutter));
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = Vec::new();
        ids.extend(self.cover.as_ref().map(WidgetPod::id));
        ids.extend(self.star.as_ref().map(WidgetPod::id));
        ids.extend(self.thumbnail.as_ref().map(WidgetPod::id));
        ids.extend(self.solo.as_ref().map(WidgetPod::id));
        ChildrenIds::from_slice(&ids)
    }
}
