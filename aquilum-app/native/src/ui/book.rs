use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerButton, PointerEvent,
    PropertiesMut, PropertiesRef, PropertySet, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::imaging::{ClipRef, Composite, Filter, GroupRef, PaintSink, Painter};
use masonry::kurbo::{Affine, Axis, BezPath, Point, Rect, RoundedRect, Shape, Size, Stroke, Vec2};
use masonry::layout::{LenDef, LenReq, Length, SizeDef};
use masonry::peniko::{BlendMode, Color, ColorStop, Compose, Fill, Gradient, ImageBrush, Mix};
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::Gap;
use masonry::widgets::Flex;

use super::handle::Handle;
use super::icons;
use super::menu::box_shadow;
use super::theme;
use super::tokens::size;
use super::widgets::{ButtonSize, IconButton, TextButton, Variant};
use crate::cover;
use crate::i18n::t;

const CONTENT_GAP: f64 = 56.0;
const CONTROLS_TOP: f64 = 16.0;
const CONTROLS_RIGHT: f64 = 12.0;
const RAYS_WIDTH: f64 = 311.0;

pub enum Visual {
    Pattern(&'static str),
    Image(ImageBrush),
}

pub struct BookView {
    pub cover: ImageBrush,
    pub has_file: bool,
}

pub struct HeroView {
    pub cover: Option<Visual>,
    pub position: f64,
    pub book: Option<BookView>,
    pub viewport: f64,
    pub motion_enabled: bool,
}

pub struct HeroIds {
    pub cover: Option<Handle<PageCover>>,
    pub replace: Option<WidgetId>,
    pub random: Option<WidgetId>,
    pub reposition: Option<Handle<IconButton>>,
    pub remove: Option<WidgetId>,
    pub thumbnail: Option<Handle<Thumbnail>>,
    pub replace_book: Option<WidgetId>,
    pub book_button: Option<Handle<TextButton>>,
}

#[derive(Debug)]
pub enum ThumbAction {
    Remove,
    Paste,
}

pub fn cover_height(viewport: f64) -> f64 {
    (viewport * 0.207).clamp(186.0, 240.0)
}

fn thumb_height(cover: f64) -> f64 {
    184.0 + (cover - 186.0) * 36.0 / 54.0
}

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
        let divider = || super::widgets::Rule::new(Axis::Vertical, || theme::current().book_actions_divider);
        let mut bar = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(replace).with_fixed(divider()).with_fixed(random);
        if matches!(visual, Visual::Image(_)) {
            let reposition = NewWidget::new(IconButton::mode(icons::UNFOLD_VERTICAL, t("book.coverPosition"), false));
            ids.reposition = Some(Handle::of(&reposition));
            bar = bar.with_fixed(divider()).with_fixed(reposition);
        }
        bar = bar.with_fixed(divider()).with_fixed(remove);
        let bar = NewWidget::new(ActionBar { row: NewWidget::new(bar).with_props(PropertySet::new().with(Gap::new(Length::px(size::SPACE_4)))).to_pod() });
        let widget = PageCover::new(visual, view.position, view.motion_enabled, bar, button.take());
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
        let widget = NewWidget::new(Thumbnail { image: book.cover, height: 0.0, replace: replace.to_pod(), hovered: false, baked: None });
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
            self.column = Some(super::note::ch_width(ctx));
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

pub struct ActionBar {
    row: WidgetPod<Flex>,
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

pub struct PageCover {
    visual: Visual,
    position: f64,
    repositioning: bool,
    drag: Option<(f64, f64)>,
    actions: WidgetPod<ActionBar>,
    book: Option<WidgetPod<TextButton>>,
    show_actions: bool,
    baked: Option<(BakeKey, ImageBrush)>,
    clock: f64,
    motion_enabled: bool,
}

type BakeKey = (u16, u16, usize, u64, i64);

fn theme_id() -> usize {
    std::ptr::from_ref(theme::current()) as usize
}

fn bake(size: Size, scale: f64, draw: impl FnOnce(&mut Painter<'_>)) -> Option<ImageBrush> {
    let mut scene = Scene::new();
    {
        let mut recorder = Painter::new(&mut scene);
        draw(&mut recorder.as_dyn());
    }
    let (dw, dh) = device(Rect::from_origin_size(Point::ORIGIN, size), scale);
    crate::render::offscreen(dw, dh, |painter| replay_transformed(&scene, painter.sink_mut(), Affine::scale(scale)))
}

fn cover_body(painter: &mut Painter<'_>, bounds: Rect, visual: &Visual, position: f64, pattern: Option<&ImageBrush>, scale: f64) {
    let tm = theme::current();
    let shape = RoundedRect::from_rect(bounds, size::BOOK_COVER_RADIUS);
    painter.with_fill_clip(shape, |painter| {
        painter.fill(bounds, tm.bg_sunken).draw();
        match visual {
            Visual::Pattern(id) => {
                painter.fill(bounds, cover::base_color(id)).draw();
                if let Some(image) = pattern {
                    painter.draw_image(image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale(1.0 / scale));
                }
            }
            Visual::Image(image) => {
                let (w, h) = (f64::from(image.image.width), f64::from(image.image.height));
                let k = (bounds.width() / w).max(bounds.height() / h);
                let (dw, dh) = (w * k, h * k);
                let x = bounds.x0 + (bounds.width() - dw) / 2.0;
                let y = bounds.y0 + (bounds.height() - dh) * position / 100.0;
                painter.draw_image(image, Affine::translate((x, y)) * Affine::scale(k));
            }
        }
        if !theme::is_dark() {
            paint_rays(painter, Rect::new(bounds.x1 - RAYS_WIDTH, bounds.y0, bounds.x1, bounds.y1), scale);
        }
    });
}

impl PageCover {
    fn new(visual: Visual, position: f64, motion_enabled: bool, actions: NewWidget<ActionBar>, book: Option<NewWidget<TextButton>>) -> NewWidget<Self> {
        NewWidget::new(PageCover { visual, position, repositioning: false, drag: None, actions: actions.to_pod(), book: book.map(NewWidget::to_pod), show_actions: false, baked: None, clock: 0.0, motion_enabled })
    }

    pub fn refresh(this: &mut WidgetMut<'_, Self>) {
        this.ctx.request_paint_only();
    }

    pub fn position(&self) -> f64 {
        self.position
    }

    pub fn repositioning(&self) -> bool {
        self.repositioning
    }

    pub fn set_repositioning(this: &mut WidgetMut<'_, Self>, on: bool, position: Option<f64>) {
        this.widget.repositioning = on;
        if let Some(position) = position {
            this.widget.position = position;
        }
        this.widget.drag = None;
        let show = on || this.ctx.is_hovered() || this.ctx.has_hovered();
        this.widget.show_actions = show;
        this.ctx.set_stashed(&mut this.widget.actions, !show);
        this.ctx.request_paint_only();
        this.ctx.request_layout();
    }

    fn overflow(&self, size_: Size) -> f64 {
        let Visual::Image(image) = &self.visual else { return 0.0 };
        let (w, h) = (f64::from(image.image.width), f64::from(image.image.height));
        if w <= 0.0 || h <= 0.0 {
            return 0.0;
        }
        (h * (size_.width / w).max(size_.height / h) - size_.height).max(0.0)
    }

    fn motion(&self) -> Option<&'static str> {
        if !self.motion_enabled {
            return None;
        }
        match &self.visual {
            Visual::Pattern(id) if cover::animated(id) => Some(id),
            _ => None,
        }
    }

    fn sync_actions(&mut self, ctx: &mut UpdateCtx<'_>) {
        let show = self.repositioning || ctx.is_hovered() || ctx.has_hovered();
        if show != self.show_actions {
            self.show_actions = show;
            ctx.set_stashed(&mut self.actions, !show);
            ctx.request_layout();
        }
    }
}

impl Widget for PageCover {
    type Action = masonry::core::NoAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if !self.repositioning {
            return;
        }
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.drag = Some((ctx.local_position(e.state.position).y, self.position));
                ctx.capture_pointer();
            }
            PointerEvent::Move(update) => {
                if let Some((start, origin)) = self.drag {
                    let overflow = self.overflow(ctx.content_box().size());
                    if overflow > 0.0 {
                        let dy = ctx.local_position(update.current.position).y - start;
                        self.position = (origin - dy / overflow * 100.0).clamp(0.0, 100.0);
                        ctx.request_paint_only();
                    }
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => self.drag = None,
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                ctx.set_stashed(&mut self.actions, true);
                if self.motion().is_some() {
                    ctx.request_anim_frame();
                }
            }
            Update::HoveredChanged(_) | Update::ChildHoveredChanged(_) => self.sync_actions(ctx),
            _ => {}
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        if self.motion().is_some() {
            self.clock += interval as f64 / 1e9;
            ctx.request_paint_only();
            ctx.request_anim_frame();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.actions);
        if let Some(book) = &mut self.book {
            ctx.register_child(book);
        }
    }

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        _ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let row = size::SIZE_32 + size::SPACE_3 * 2.0;
        let mut right = size_.width - CONTROLS_RIGHT;
        let auto = SizeDef::new(LenDef::MaxContent, LenDef::MaxContent);
        if let Some(book) = &mut self.book {
            let button = ctx.compute_size(book, auto, size_.into());
            ctx.run_layout(book, button);
            ctx.place_child(book, Point::new(right - button.width, CONTROLS_TOP + (row - button.height) / 2.0));
            right -= button.width + size::SPACE_8;
        }
        if self.show_actions {
            let bar = ctx.compute_size(&mut self.actions, auto, size_.into());
            ctx.run_layout(&mut self.actions, bar);
            ctx.place_child(&mut self.actions, Point::new(right - bar.width, CONTROLS_TOP + (row - bar.height) / 2.0));
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let bounds = ctx.content_box();
        let scale = ctx.scale_factor();
        let pattern = match &self.visual {
            Visual::Pattern(id) => {
                let width = ((bounds.width() * scale / 128.0).ceil() * 128.0) as u32;
                let height = (bounds.height() * scale).ceil() as u32;
                let key = cover::Key { id, width: width.max(1), height: height.max(1), scale: (scale * 1000.0).round() as u32 };
                Some(cover::image(key))
            }
            Visual::Image(_) => None,
        };
        let ready = !matches!(pattern, Some(None));
        let pattern = pattern.flatten();
        if self.repositioning || !ready {
            cover_body(painter, bounds, &self.visual, self.position, pattern.as_ref(), scale);
        } else {
            let (dw, dh) = device(bounds, scale);
            let source = match &self.visual {
                Visual::Pattern(id) => id.as_ptr() as u64,
                Visual::Image(image) => image.image.data.id(),
            };
            let key = (dw, dh, theme_id(), source, (self.position * 10.0).round() as i64);
            if self.baked.as_ref().map(|(k, _)| *k) != Some(key) {
                let local = Rect::from_origin_size(Point::ORIGIN, bounds.size());
                let (visual, position) = (&self.visual, self.position);
                self.baked = bake(bounds.size(), scale, |painter| cover_body(painter, local, visual, position, pattern.as_ref(), scale)).map(|image| (key, image));
            }
            match &self.baked {
                Some((_, image)) => painter.draw_image(image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale(1.0 / scale)),
                None => cover_body(painter, bounds, &self.visual, self.position, pattern.as_ref(), scale),
            }
        }
        if let Some(id) = self.motion()
            && ready
        {
            let viewport = ctx.window_size().height;
            painter.with_fill_clip(RoundedRect::from_rect(bounds, size::BOOK_COVER_RADIUS), |painter| cover::paint_motion(painter, id, bounds, self.clock, viewport));
        }
        if self.repositioning {
            let frame = RoundedRect::from_rect(bounds.inset(-0.5), size::BOOK_COVER_RADIUS);
            painter.stroke(&frame, &Stroke::new(size::BORDER_1), tm.border_accent.multiply_alpha(0.5)).draw();
        }
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        if self.repositioning { CursorIcon::NsResize } else { CursorIcon::Default }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = vec![self.actions.id()];
        ids.extend(self.book.as_ref().map(WidgetPod::id));
        ChildrenIds::from_slice(&ids)
    }
}

fn linear(from: Point, to: Point, stops: &[(f32, Color)]) -> Gradient {
    Gradient::new_linear(from, to).with_stops(stops.iter().map(|(o, c)| ColorStop::from((*o, *c))).collect::<Vec<_>>().as_slice())
}

fn cached(kind: u8, width: u32, height: u32, make: impl FnOnce() -> Option<ImageBrush>) -> Option<ImageBrush> {
    static CACHE: std::sync::Mutex<Vec<((u8, u32, u32), ImageBrush)>> = std::sync::Mutex::new(Vec::new());
    let key = (kind, width, height);
    if let Some((_, image)) = CACHE.lock().ok()?.iter().find(|(k, _)| *k == key) {
        return Some(image.clone());
    }
    let image = make()?;
    let mut cache = CACHE.lock().ok()?;
    if cache.len() >= 8 {
        cache.remove(0);
    }
    cache.push((key, image.clone()));
    Some(image)
}

fn device(rect: Rect, scale: f64) -> (u16, u16) {
    ((rect.width() * scale).ceil().clamp(1.0, 8192.0) as u16, (rect.height() * scale).ceil().clamp(1.0, 8192.0) as u16)
}

fn paint_rays(painter: &mut Painter<'_, impl PaintSink + ?Sized>, rect: Rect, scale: f64) {
    let transform = Affine::translate(rect.origin().to_vec2()) * Affine::scale_non_uniform(rect.width() / 311.0, rect.height() / 186.0);
    let white = Color::WHITE;
    let overlay = Composite::new(BlendMode::new(Mix::Overlay, Compose::SrcOver), 0.5);
    painter.with_group(GroupRef::new().with_composite(overlay), |painter| {
        let gradient = linear(Point::new(311.0, 93.0), Point::new(0.0, 93.0), &[(0.0, white), (1.0, white.with_alpha(0.0))]);
        painter.fill(Rect::new(0.0, 0.0, 311.0, 186.0), &gradient).transform(transform).draw();
    });
    let (dw, dh) = device(rect, scale);
    let Some(rays) = cached(0, u32::from(dw), u32::from(dh), || {
        crate::render::offscreen(dw, dh, |painter| {
            let local = Affine::scale_non_uniform(f64::from(dw) / 311.0, f64::from(dh) / 186.0);
            let gray = (Color::from_rgb8(0x83, 0x83, 0x83), Color::from_rgb8(0x4d, 0x4d, 0x4d));
            let light = (Color::WHITE, Color::from_rgb8(0x8c, 0x8c, 0x8c));
            let rays: [(&str, f32, f32, (f64, f64, f64, f64), (Color, Color)); 7] = [
                ("M394.487 -2.94443L237.769 -19.0234L189.579 186.356L394.487 -2.94443Z", 30.0, 1.0, (324.26, -47.1334, 360.017, 133.647), gray),
                ("M298.919 -11.7232L266.071 -63.7713L75.1033 167.198L298.919 -11.7232Z", 20.0, 1.0, (316.004, -78.4749, 325.394, 134.579), gray),
                ("M285.505 -41.0939L270.508 -95.0064L6.92272 126.088L285.505 -41.0939Z", 20.0, 1.0, (324.323, -107.045, 322.222, 107.479), gray),
                ("M353.567 -17.1869L300.139 -18.4095L180.048 181.563L353.567 -17.1869Z", 18.7118, 1.0, (314.1, -51.9018, 349.819, 129.038), light),
                ("M392.156 -33.344L369.785 -44.0036L290.1 175.465L392.156 -33.344Z", 9.0, 0.6, (380.534, -81.8194, 411.882, 135.464), light),
                ("M309.19 -44.1356L294.329 -56.8787L86.597 152.525L309.19 -44.1356Z", 10.0, 0.6, (327.289, -93.2525, 336.494, 120.126), light),
                ("M306.296 -73.0126L295.939 -86.5797L23.0478 112.006L306.296 -73.0126Z", 7.5, 0.6, (340.278, -121.243, 337.971, 93.6527), light),
            ];
            for (path, blur, opacity, (x1, y1, x2, y2), (a, b)) in rays {
                let Ok(path) = BezPath::from_svg(path) else { continue };
                let filters = [Filter::blur(blur * (f64::from(dh) / 186.0) as f32)];
                let group = GroupRef::new().with_filters(&filters).with_composite(Composite::new(BlendMode::default(), opacity));
                painter.with_group(group, |painter| {
                    let gradient = linear(Point::new(x1, y1), Point::new(x2, y2), &[(0.0, a), (1.0, b)]);
                    painter.fill(&path, &gradient).transform(local).draw();
                });
            }
        })
    }) else {
        return;
    };
    let dodge = Composite::new(BlendMode::new(Mix::ColorDodge, Compose::SrcOver), 0.7);
    painter.with_group(GroupRef::new().with_composite(dodge).with_clip(ClipRef::fill(rect)), |painter| {
        painter.draw_image(&rays, Affine::translate(rect.origin().to_vec2()) * Affine::scale(1.0 / scale));
    });
}

const STAR_PATH: &str = "M26.7326 32.142C34.8108 26.881 36.851 0 36.851 0C36.851 0 38.6803 24.7488 46 30C54.5 36.0979 96 38.5621 96 38.5621C96 38.5621 53.7749 39.0195 45.6578 44.4024C37.5407 49.7852 36.851 63 36.851 63C36.851 63 35.1929 49.7069 26.7326 44.4024C18.7456 39.3946 0 38.5621 0 38.5621C0 38.5621 18.8107 37.3012 26.7326 32.142Z";

pub struct Star;

impl Widget for Star {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        let scale = ctx.scale_factor();
        let (dw, dh) = device(bounds, scale);
        let Some(image) = cached(1, u32::from(dw), u32::from(dh), || {
            crate::render::offscreen(dw, dh, |painter| {
                let Ok(star) = BezPath::from_svg(STAR_PATH) else { return };
                let k = f64::from(dw) / 96.0;
                let star = Affine::scale(k) * star;
                painter.fill(&star, Color::WHITE.with_alpha(0.4)).draw();
                let shadows = [(Vec2::new(-1.93416, -1.93416), 1.93416, 0.5), (Vec2::new(1.93416, 11.0), 3.86832, 0.3)];
                let frame = Rect::new(0.0, 0.0, f64::from(dw), f64::from(dh));
                painter.with_fill_clip(&star, |painter| {
                    for (offset, blur, alpha) in shadows {
                        let mut outside = frame.inflate(frame.width(), frame.height()).to_path(0.1);
                        outside.extend(Affine::translate(offset * k) * star.clone());
                        let filters = [Filter::blur((blur * k) as f32)];
                        painter.with_group(GroupRef::new().with_filters(&filters), |painter| {
                            painter.fill(&outside, Color::WHITE.with_alpha(alpha)).fill_rule(Fill::EvenOdd).draw();
                        });
                    }
                });
            })
        }) else {
            return;
        };
        painter.draw_image(&image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale(1.0 / scale));
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct Thumbnail {
    image: ImageBrush,
    height: f64,
    replace: WidgetPod<IconButton>,
    hovered: bool,
    baked: Option<((u16, u16, usize), ImageBrush)>,
}

const THUMB_MARGIN: f64 = 48.0;

fn thumbnail_body(painter: &mut Painter<'_>, bounds: Rect, image: &ImageBrush) {
    let tm = theme::current();
    let shape = RoundedRect::from_rect(bounds, size::BOOK_THUMB_RADIUS);
    box_shadow(painter, shape, tm.book_thumb_shadow);
    painter.with_fill_clip(shape, |painter| {
        painter.fill(bounds, tm.bg_surface_subtle).draw();
        let h = f64::from(image.image.height);
        if h > 0.0 {
            let k = bounds.height() / h;
            painter.draw_image(image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale(k));
        }
        let overlay = Composite::new(BlendMode::new(Mix::Overlay, Compose::SrcOver), 1.0);
        if !theme::is_dark() {
            painter.with_group(GroupRef::new().with_composite(overlay), |painter| {
                let diagonal = bounds.width().max(bounds.height());
                let gradient = linear(
                    Point::new(bounds.x1, bounds.y0),
                    Point::new(bounds.x1 - diagonal * 0.7071, bounds.y0 + diagonal * 0.7071),
                    &[(0.0, Color::WHITE), (1.0, Color::WHITE.with_alpha(0.0))],
                );
                painter.fill(bounds, &gradient).draw();
            });
        }
        let wash = Composite::new(BlendMode::new(Mix::Overlay, Compose::SrcOver), 0.5);
        painter.with_group(GroupRef::new().with_composite(wash), |painter| {
            let view = Affine::translate(bounds.origin().to_vec2()) * Affine::scale_non_uniform(bounds.width() / 130.0, bounds.height() / 166.0);
            painter.fill(Rect::new(0.0, 0.0, 130.0, 166.0), Color::WHITE).transform(view).draw();
            let radial = Gradient::new_radial(Point::ORIGIN, 1.0).with_stops(
                [
                    ColorStop::from((0.052_083_3, Color::from_rgba8(0x00, 0xff, 0x84, 84))),
                    ColorStop::from((0.447_931, Color::from_rgba8(0x00, 0x75, 0xff, 97))),
                    ColorStop::from((1.0, Color::WHITE.with_alpha(0.0))),
                ]
                .as_slice(),
            );
            let brush = Affine::translate((65.0, 0.0)) * Affine::rotate(std::f64::consts::FRAC_PI_2) * Affine::scale_non_uniform(166.0, 178.164);
            painter.fill(Rect::new(0.0, 0.0, 130.0, 166.0), &radial).transform(view).brush_transform(Some(brush)).draw();
        });
    });
    let border = RoundedRect::from_rect(bounds.inset(-0.5), size::BOOK_THUMB_RADIUS - 0.5);
    painter.stroke(&border, &Stroke::new(size::BORDER_1), tm.book_thumb_border).draw();
}

impl Widget for Thumbnail {
    type Action = ThumbAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let PointerEvent::Down(_) = event {
            ctx.request_focus();
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        let TextEvent::Keyboard(key) = event else { return };
        if key.state != KeyState::Down {
            return;
        }
        let action = key.modifiers.ctrl() || key.modifiers.meta();
        match &key.key {
            Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
                ctx.submit_action::<ThumbAction>(ThumbAction::Remove);
                ctx.set_handled();
            }
            Key::Named(NamedKey::Escape) => {
                ctx.resign_focus();
                ctx.set_handled();
            }
            Key::Character(c) if action && c.eq_ignore_ascii_case("v") => {
                ctx.submit_action::<ThumbAction>(ThumbAction::Paste);
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => ctx.set_stashed(&mut self.replace, true),
            Update::HoveredChanged(_) | Update::ChildHoveredChanged(_) => {
                let hovered = ctx.is_hovered() || ctx.has_hovered();
                if hovered != self.hovered {
                    self.hovered = hovered;
                    ctx.set_stashed(&mut self.replace, !hovered);
                    ctx.request_layout();
                }
            }
            Update::FocusChanged(_) => ctx.request_paint_only(),
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.replace);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        self.height = size_.height;
        if self.hovered {
            let button = Size::new(size::SIZE_32, size::SIZE_32);
            ctx.run_layout(&mut self.replace, button);
            let inset = size::BOOK_THUMB_ACTION_INSET;
            ctx.place_child(&mut self.replace, Point::new(size_.width - inset - button.width, inset));
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let bounds = ctx.content_box();
        let scale = ctx.scale_factor();
        if ctx.is_focus_target() {
            box_shadow(painter, RoundedRect::from_rect(bounds, size::BOOK_THUMB_RADIUS), tm.shadow_focus_ring);
        }
        let canvas = bounds.size() + Size::new(THUMB_MARGIN * 2.0, THUMB_MARGIN * 2.0);
        let (dw, dh) = device(Rect::from_origin_size(Point::ORIGIN, canvas), scale);
        let key = (dw, dh, theme_id());
        if self.baked.as_ref().map(|(k, _)| *k) != Some(key) {
            let local = Rect::from_origin_size(Point::new(THUMB_MARGIN, THUMB_MARGIN), bounds.size());
            let image = &self.image;
            self.baked = bake(canvas, scale, |painter| thumbnail_body(painter, local, image)).map(|image| (key, image));
        }
        match &self.baked {
            Some((_, image)) => {
                let origin = bounds.origin() - Vec2::new(THUMB_MARGIN, THUMB_MARGIN);
                painter.draw_image(image, Affine::translate(origin.to_vec2()) * Affine::scale(1.0 / scale));
            }
            None => thumbnail_body(painter, bounds, &self.image),
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(t("book.coverAria"));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.replace.id()])
    }
}
