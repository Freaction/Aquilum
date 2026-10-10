//! Всплывающее меню — `src/components/Common/Menu.{tsx,css}`: слой над окном у точки щелчка,
//! пункты подсвечиваются акцентом при наведении. Закрывается щелчком мимо меню, Escape или
//! потерей фокуса окна; выбор пункта приходит окну действием `MenuChoice`.

use masonry::accesskit::{Action, Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget,
    PaintCtx, PointerButton, PointerEvent, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx,
    TextEvent, Update, UpdateCtx, Widget, WidgetPod, render_text,
};
use masonry::imaging::{BlurredRoundedRect, Composite, Painter};
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LayoutSize, LenReq, Length};
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::scroll::ScrollArea;
use super::tokens::{Shadow, size};
use super::{icons, text, theme};

/// Выбор в меню или его закрытие без выбора.
#[derive(Debug)]
pub enum MenuChoice {
    Item(usize),
    Dismissed,
}

/// Пункт меню (`.q-menu__item`); у списка выбора отмеченный пункт — с галочкой.
pub struct Entry<'a> {
    pub label: &'a str,
    pub icon: Option<icons::Icon>,
    pub disabled: bool,
}

struct MenuItem {
    index: usize,
    label: String,
    icon: Option<icons::Icon>,
    disabled: bool,
    checked: bool,
    line: text::Line,
    down: bool,
}

impl MenuItem {
    fn height() -> f64 {
        size::PADDING_2XS * 2.0 + size::SIZE_20
    }
}

impl Widget for MenuItem {
    type Action = MenuChoice;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(_) if self.disabled => {}
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.down = true;
                ctx.capture_pointer();
            }
            PointerEvent::Up(e) if e.button == Some(PointerButton::Primary) && self.down => {
                self.down = false;
                if ctx.is_hovered() {
                    ctx.submit_action::<MenuChoice>(MenuChoice::Item(self.index));
                }
            }
            _ => {}
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click && !self.disabled {
            ctx.submit_action::<MenuChoice>(MenuChoice::Item(self.index));
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => ctx.request_paint_only(),
            // Шрифты регистрируются после создания окна: текст надо разложить и измерить заново.
            Update::FontsChanged => {
                self.line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(Self::height()),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => {
                let width = text::measure(ctx, &self.label, text::Style::ui(size::SIZE_20));
                let icon = if self.icon.is_some() { size::SIZE_20 + size::GAP_MD } else { 0.0 };
                Length::px(width.ceil() + size::PADDING_XS * 2.0 + icon)
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered() && !self.disabled;
        if hovered {
            painter.fill(RoundedRect::from_rect(bounds, size::RADIUS_LG), t.bg_accent).draw();
        }
        let lead = if self.icon.is_some() { size::SIZE_20 + size::GAP_MD } else { 0.0 };
        if let Some(icon) = self.icon {
            let color = if self.disabled {
                t.text_disabled
            } else if hovered {
                t.icon_on_accent
            } else {
                t.icon_muted
            };
            icon.draw(painter, Point::new(bounds.x0 + size::PADDING_XS, bounds.y0 + size::PADDING_2XS), size::SIZE_20, 1.2, color);
        }
        let check = if self.checked { size::SIZE_20 + size::GAP_MD } else { 0.0 };
        let available = (bounds.width() - size::PADDING_XS * 2.0 - check - lead).max(0.0);
        let layout = self.line.layout(ctx, &self.label, available, text::Style::ui(size::SIZE_20));
        let color = if self.disabled {
            t.text_disabled
        } else if hovered {
            t.text_on_accent
        } else {
            t.text_secondary
        };
        let origin = Affine::translate((bounds.x0 + size::PADDING_XS + lead, bounds.y0 + size::PADDING_2XS));
        render_text(painter, origin, layout, &[color.into()], true);
        if self.checked {
            let at = Point::new(bounds.x1 - size::PADDING_XS - size::SIZE_20, bounds.y0 + size::PADDING_2XS);
            let color = if hovered { t.icon_on_accent } else { t.icon_muted };
            icons::CHECK.draw(painter, at, size::SIZE_20, 1.6, color);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::MenuItem
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        if self.checked {
            node.set_selected(true);
        }
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// Слой меню (`.q-menu`): фон, тени, колонка пунктов.
pub struct Menu {
    child: WidgetPod<ScrollArea<Flex>>,
    label: String,
    /// Ширина списка выбора — по кнопке, которая его открыла.
    width: Option<f64>,
    max_height: f64,
    exclude: Option<Rect>,
}

impl Menu {
    /// Меню из пунктов `items`; `label` — название для доступности («Действия с файлом»).
    pub fn new(label: &str, items: &[&str]) -> NewWidget<Self> {
        let entries: Vec<Entry<'_>> = items.iter().map(|label| Entry { label, icon: None, disabled: false }).collect();
        Self::build(label, &entries, None, None)
    }

    /// Меню из пунктов с иконками; неактивные пункты серые и не выбираются.
    pub fn entries(label: &str, entries: &[Entry<'_>]) -> NewWidget<Self> {
        Self::build(label, entries, None, None)
    }

    /// Список выбора (`Dropdown`) под кнопкой `trigger`: пункт `checked` с галочкой.
    pub fn list(label: &str, items: &[&str], checked: usize, trigger: Rect) -> NewWidget<Self> {
        let entries: Vec<Entry<'_>> = items.iter().map(|label| Entry { label, icon: None, disabled: false }).collect();
        let mut menu = Self::build(label, &entries, Some(checked), Some(trigger.width()));
        menu.widget.max_height = Self::LIST_MAX_HEIGHT;
        menu.widget.exclude = Some(trigger);
        menu
    }

    pub const LIST_MAX_HEIGHT: f64 = 256.0;

    pub fn list_origin(trigger: Rect, items: usize, window: Size) -> Point {
        let height = Self::estimated_size(items).height.min(Self::LIST_MAX_HEIGHT);
        let below = trigger.y1 + size::SPACE_4;
        let y = if below + height > window.height - size::SPACE_8 { (trigger.y0 - size::SPACE_4 - height).max(size::SPACE_8) } else { below };
        Point::new(trigger.x0, y)
    }

    fn build(label: &str, items: &[Entry<'_>], checked: Option<usize>, width: Option<f64>) -> NewWidget<Self> {
        let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, item) in items.iter().enumerate() {
            column = column.with_fixed(NewWidget::new(MenuItem {
                index,
                label: item.label.to_owned(),
                icon: item.icon,
                disabled: item.disabled,
                checked: checked == Some(index),
                line: text::Line::default(),
                down: false,
            }));
        }
        let column = NewWidget::new(column).with_props(
            PropertySet::new()
                .with(Gap::new(Length::px(size::GAP_XS)))
                .with(Padding::all(Length::px(size::GAP_XS))),
        );
        let scroll = NewWidget::new(ScrollArea::new(column));
        NewWidget::new(Menu { child: scroll.to_pod(), label: label.to_owned(), width, max_height: f64::INFINITY, exclude: None })
    }

    /// Минимальная ширина меню (`min-width: 8rem`).
    const MIN_WIDTH: f64 = 128.0;

    /// Примерный размер меню из `items` пунктов — чтобы развернуть его у края окна до раскладки.
    pub fn estimated_size(items: usize) -> Size {
        let items = items as f64;
        let height = items * MenuItem::height() + (items + 1.0) * size::GAP_XS;
        Size::new(Self::MIN_WIDTH + 32.0, height)
    }
}

impl Widget for Menu {
    type Action = MenuChoice;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        let close = match event {
            TextEvent::Keyboard(k) => k.state == KeyState::Down && k.key == Key::Named(NamedKey::Escape),
            TextEvent::WindowFocusChange(false) => true,
            _ => false,
        };
        if close {
            ctx.submit_action::<MenuChoice>(MenuChoice::Dismissed);
            ctx.remove_layer(ctx.widget_id());
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = LayoutSize::maybe(axis.cross(), cross);
        let length = ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        match (axis, self.width) {
            (Axis::Horizontal, Some(width)) => Length::px(width.max(Self::MIN_WIDTH)),
            (Axis::Horizontal, None) => Length::px(length.get().max(Self::MIN_WIDTH)),
            (Axis::Vertical, _) => Length::px(length.get().min(self.max_height)),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds, size::RADIUS_XL);
        box_shadow(painter, shape, t.elevation_float);
        painter.fill(shape, t.dialog_bg).draw();
        // `--q-shadow-ring`: кольцо в 1 px цвета границы поверх края.
        for ring in t.shadow_ring {
            painter.stroke(&shape, &Stroke::new(ring.spread), ring.color).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Menu
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for Menu {
    fn capture_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        let (position, scroll) = match event {
            PointerEvent::Down(e) => (e.state.position, false),
            PointerEvent::Scroll(e) => (e.state.position, true),
            _ => return,
        };
        let local = ctx.local_position(position);
        let window = ctx.to_window(local);
        if ctx.border_box().contains(local) || !scroll && self.exclude.is_some_and(|r| r.contains(window)) {
            return;
        }
        ctx.submit_action::<MenuChoice>(MenuChoice::Dismissed);
        ctx.remove_layer(ctx.widget_id());
    }
}

/// `box-shadow` из токенов: размытый скруглённый прямоугольник под фигурой. Размытие CSS
/// соответствует удвоенному стандартному отклонению.
pub fn box_shadow<S: masonry::imaging::PaintSink + ?Sized>(painter: &mut Painter<'_, S>, shape: RoundedRect, shadows: &[Shadow]) {
    for shadow in shadows.iter().filter(|s| s.blur > 0.0 || s.x != 0.0 || s.y != 0.0) {
        let rect = shape.rect().inflate(shadow.spread, shadow.spread) + masonry::kurbo::Vec2::new(shadow.x, shadow.y);
        painter.blurred_rounded_rect(BlurredRoundedRect {
            transform: Affine::IDENTITY,
            rect,
            color: shadow.color,
            radius: shape.radii().top_left,
            std_dev: shadow.blur / 2.0,
            composite: Composite::default(),
        });
    }
}
