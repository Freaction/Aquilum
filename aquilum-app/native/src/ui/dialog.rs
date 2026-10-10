//! Модальные окна — `Dialog.tsx`: затемнение на всё окно и карточка по центру. Escape и щелчок
//! мимо карточки — отмена; у диалога подтверждения (`ConfirmDialog.tsx`) Enter — подтверждение.
//! Ответ приходит окну действием `DialogAnswer` от слоя.

use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, ErasedAction, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget,
    PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx, TextEvent,
    Widget, WidgetId, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LenReq, Length};
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::menu::box_shadow;
use super::text::label;
use super::tokens::size;
use super::widgets::{Pressed, Rule, TextButton, Variant};
use super::theme;

#[derive(Debug, PartialEq)]
pub enum DialogAnswer {
    Confirm,
    Cancel,
}

/// Размер карточки.
#[derive(Clone, Copy)]
pub enum CardSize {
    /// Ширина `min(100%, width)`, высота по содержимому.
    Fit(f64),
    /// Размер от места в окне (поля уже вычтены) — у окна настроек.
    Window(fn(Size) -> Size),
}

/// Слой модального окна с карточкой `card`.
pub struct Modal {
    card: WidgetPod<Flex>,
    title: String,
    size: CardSize,
    role: Role,
    /// Enter подтверждает (диалог подтверждения).
    enter_confirms: bool,
    /// Где карточка лежит в окне — после раскладки.
    card_rect: Rect,
}

impl Modal {
    pub fn new(card: NewWidget<Flex>, title: &str, size: CardSize, role: Role, enter_confirms: bool) -> NewWidget<Self> {
        NewWidget::new(Modal { card: card.to_pod(), title: title.to_owned(), size, role, enter_confirms, card_rect: Rect::ZERO })
    }

    /// Карточка по центру окна размера `window`.
    fn centered(window: Size, card: Size) -> Rect {
        let origin = Point::new(((window.width - card.width) / 2.0).round(), ((window.height - card.height) / 2.0).round());
        Rect::from_origin_size(origin, card)
    }
}

/// Шапка модального окна (`.q-dialog__header`): заголовок и, если есть, кнопка закрытия.
pub fn header(title: &str, close: Option<NewWidget<impl Widget>>) -> NewWidget<Flex> {
    let t = theme::current();
    let title = label(title, t.text_secondary, None, size::SIZE_20 as f32);
    let mut row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with(title, 1.0);
    if let Some(close) = close {
        row = row.with_fixed(close);
    }
    NewWidget::new(row).with_props(
        PropertySet::new().with(Gap::new(Length::px(size::GAP_XL))).with(Padding::all(Length::px(size::DIALOG_PADDING))),
    )
}

/// Диалог подтверждения.
pub struct ConfirmDialog;

impl ConfirmDialog {
    /// Слой диалога и id его частей.
    pub fn new(title: &str, description: &str, cancel_label: &str, confirm_label: &str) -> (NewWidget<Modal>, DialogIds) {
        let t = theme::current();
        let label = |text: &str, color| label(text, color, None, size::SIZE_20 as f32);
        let header = header(title, None::<NewWidget<Flex>>);
        let content = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(label(description, t.text_secondary)))
            .with_props(PropertySet::new().with(Padding::all(Length::px(size::SPACE_24))));
        let cancel = NewWidget::new(TextButton::new(cancel_label, Variant::Ghost));
        let confirm = NewWidget::new(TextButton::new(confirm_label, Variant::Primary));
        let (cancel_id, confirm_id) = (cancel.id(), confirm.id());
        let footer = NewWidget::new(
            Flex::row().main_axis_alignment(MainAxisAlignment::End).with_fixed(cancel).with_fixed(confirm),
        )
        .with_props(
            PropertySet::new()
                .with(Gap::new(Length::px(size::DIALOG_GAP)))
                .with(Padding::all(Length::px(size::DIALOG_PADDING))),
        );
        let border = || Rule::new(Axis::Horizontal, || theme::current().border_default);
        let card = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(header)
            .with_fixed(border())
            .with_fixed(content)
            .with_fixed(border())
            .with_fixed(footer);
        let dialog = Modal::new(NewWidget::new(card), title, CardSize::Fit(size::CONFIRM_DIALOG_WIDTH), Role::AlertDialog, true);
        let ids = DialogIds::of(&dialog, (cancel_id, confirm_id));
        (dialog, ids)
    }
}

/// id частей диалога, по которым окно разбирает действия.
pub struct DialogIds {
    pub layer: WidgetId,
    cancel: WidgetId,
    confirm: WidgetId,
}

impl DialogIds {
    pub fn of(dialog: &NewWidget<Modal>, inner: (WidgetId, WidgetId)) -> Self {
        DialogIds { layer: dialog.id(), cancel: inner.0, confirm: inner.1 }
    }

    /// Ответ диалога по действию: кнопка «Отмена» или подтверждения, либо сам слой
    /// (Escape, Enter, щелчок мимо карточки).
    pub fn answer(&self, id: WidgetId, action: &ErasedAction) -> Option<DialogAnswer> {
        if id == self.layer {
            return match action.downcast_ref::<DialogAnswer>()? {
                DialogAnswer::Confirm => Some(DialogAnswer::Confirm),
                DialogAnswer::Cancel => Some(DialogAnswer::Cancel),
            };
        }
        action.downcast_ref::<Pressed>()?;
        if id == self.confirm {
            Some(DialogAnswer::Confirm)
        } else if id == self.cancel {
            Some(DialogAnswer::Cancel)
        } else {
            None
        }
    }
}

impl Widget for Modal {
    type Action = DialogAnswer;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        if let TextEvent::Keyboard(k) = event
            && k.state == KeyState::Down
        {
            match k.key {
                Key::Named(NamedKey::Escape) => ctx.submit_action::<DialogAnswer>(DialogAnswer::Cancel),
                Key::Named(NamedKey::Enter) if self.enter_confirms => {
                    ctx.submit_action::<DialogAnswer>(DialogAnswer::Confirm);
                }
                _ => {}
            }
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.card);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        // Затемнение — на всё окно.
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // Поля затемнения — 16 px с каждой стороны.
        let room = Size::new((size.width - size::SPACE_16 * 2.0).max(0.0), (size.height - size::SPACE_16 * 2.0).max(0.0));
        let card = match self.size {
            CardSize::Fit(max) => {
                let width = room.width.min(max);
                // Высота — по содержимому при этой ширине, но не больше окна.
                let height = ctx
                    .compute_length(&mut self.card, LenReq::MaxContent.into(), size.into(), Axis::Vertical, Some(Length::px(width)))
                    .get()
                    .min(room.height);
                Size::new(width, height)
            }
            CardSize::Window(of) => of(room),
        };
        ctx.run_layout(&mut self.card, card);
        self.card_rect = Self::centered(size, card);
        ctx.place_child(&mut self.card, self.card_rect.origin());
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let window = ctx.content_box();
        painter.fill(window, t.dialog_overlay_bg).draw();
        let shape = RoundedRect::from_rect(self.card_rect, size::DIALOG_RADIUS);
        box_shadow(painter, shape, t.dialog_shadow);
        painter.fill(shape, t.dialog_bg).draw();
        // Граница 1 px внутри карточки: обводка по линии в полпикселя от края.
        let border = RoundedRect::from_rect(self.card_rect.inflate(-0.5, -0.5), size::DIALOG_RADIUS - 0.5);
        painter.stroke(&border, &Stroke::new(size::BORDER_1), t.dialog_border).draw();
    }

    fn accessibility_role(&self) -> Role {
        self.role
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.title.clone());
        node.set_modal();
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.card.id()])
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for Modal {
    fn capture_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let PointerEvent::Down(e) = event {
            let position = ctx.local_position(e.state.position);
            if !self.card_rect.contains(position) {
                ctx.submit_action::<DialogAnswer>(DialogAnswer::Cancel);
            }
        }
    }
}
