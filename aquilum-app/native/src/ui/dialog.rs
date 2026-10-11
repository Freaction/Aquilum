//! Модальные окна — `Dialog.tsx`: затемнение на всё окно и карточка по центру. Escape и щелчок
//! мимо карточки — отмена; у диалога подтверждения (`ConfirmDialog.tsx`) Enter — подтверждение.
//! Ответ приходит окну действием `DialogAnswer` от слоя.

use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, ErasedAction, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget,
    PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx, TextEvent,
    Update, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LenReq, Length};
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::menu::{box_shadow, shadow_reach};
use crate::render::fade::Fade;
use super::text::label;
use super::tokens::size;
use super::widgets::{Intro, Leading, NORMAL, Pressed, Rule, TextButton, Variant};
use super::theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogAnswer {
    Confirm,
    Cancel,
    Closed,
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
    intro: Intro,
    closing: bool,
    id: Option<u64>,
}

const LIFT: f64 = 8.0;
const SHRINK: f64 = 0.02;

impl Drop for Modal {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            crate::render::fade::forget(id);
        }
    }
}

impl Modal {
    pub fn new(card: NewWidget<Flex>, title: &str, size: CardSize, role: Role, enter_confirms: bool) -> NewWidget<Self> {
        NewWidget::new(Modal { card: card.to_pod(), title: title.to_owned(), size, role, enter_confirms, card_rect: Rect::ZERO, intro: Intro::new(NORMAL), closing: false, id: None })
    }

    pub fn close(this: &mut WidgetMut<'_, Self>) {
        if this.widget.closing {
            return;
        }
        this.widget.closing = true;
        this.widget.intro.reverse();
        this.ctx.request_anim_frame();
        this.ctx.request_paint_only();
    }

    pub fn set_card(this: &mut WidgetMut<'_, Self>, card: NewWidget<Flex>, title: &str) {
        this.ctx.remove_child(std::mem::replace(&mut this.widget.card, card.to_pod()));
        this.widget.title = title.to_owned();
        this.ctx.children_changed();
        this.ctx.request_layout();
        this.ctx.request_paint_only();
        this.ctx.request_accessibility_update();
    }

    fn centered(window: Size, card: Size) -> Rect {
        let origin = Point::new(((window.width - card.width) / 2.0).round(), ((window.height - card.height) / 2.0).round());
        Rect::from_origin_size(origin, card)
    }
}

/// Шапка модального окна (`.q-dialog__header`): заголовок и, если есть, кнопка закрытия.
pub fn header(title: &str, close: Option<NewWidget<impl Widget>>) -> NewWidget<Flex> {
    let title = label(title, |t| t.text_secondary, None, Leading::Px(size::SIZE_20 as f32));
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
        let label = |text: &str, paint| label(text, paint, None, Leading::Px(size::SIZE_20 as f32));
        let header = header(title, None::<NewWidget<Flex>>);
        let content = NewWidget::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch).with_fixed(label(description, |t| t.text_secondary)))
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

pub fn close(root: &mut masonry::app::RenderRoot, layer: WidgetId) {
    root.edit_widget(layer, |mut widget| Modal::close(&mut widget.downcast()));
}

impl DialogIds {
    pub fn of(dialog: &NewWidget<Modal>, inner: (WidgetId, WidgetId)) -> Self {
        DialogIds { layer: dialog.id(), cancel: inner.0, confirm: inner.1 }
    }

    /// Ответ диалога по действию: кнопка «Отмена» или подтверждения, либо сам слой
    /// (Escape, Enter, щелчок мимо карточки).
    pub fn answer(&self, id: WidgetId, action: &ErasedAction) -> Option<DialogAnswer> {
        if id == self.layer {
            return action.downcast_ref::<DialogAnswer>().copied().filter(|answer| *answer != DialogAnswer::Closed);
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
            && !self.closing
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

    fn find_widget_under_pointer<'c>(&'c self, ctx: masonry::core::QueryCtx<'c>, pos: Point) -> Option<masonry::core::WidgetRef<'c, dyn Widget>> {
        if self.closing {
            return None;
        }
        masonry::core::find_widget_under_pointer(self, ctx, pos)
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            self.id = Some(ctx.widget_id().to_raw());
            ctx.request_anim_frame();
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        if self.intro.advance(interval) {
            ctx.request_anim_frame();
        } else if self.closing {
            ctx.submit_action::<DialogAnswer>(DialogAnswer::Closed);
        }
        ctx.request_paint_only();
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
        match self.id.filter(|_| self.intro.running() || self.closing) {
            Some(id) => {
                let v = if self.closing { self.intro.reversed() } else { self.intro.value() };
                let center = self.card_rect.center().to_vec2();
                let scale = 1.0 - SHRINK * (1.0 - v);
                let transform = Affine::translate((0.0, LIFT * (1.0 - v))) * Affine::translate(center) * Affine::scale(scale) * Affine::translate(-center);
                let bounds = self.card_rect.inflate(shadow_reach(t.dialog_shadow), shadow_reach(t.dialog_shadow));
                crate::render::fade::publish(id, Fade { backdrop: t.dialog_overlay_bg.multiply_alpha(v as f32), bounds, transform, alpha: v as f32 });
            }
            None => {
                if let Some(id) = self.id {
                    crate::render::fade::forget(id);
                }
                painter.fill(window, t.dialog_overlay_bg).draw();
            }
        }
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
        if let PointerEvent::Down(e) = event
            && !self.closing
        {
            let position = ctx.local_position(e.state.position);
            if !self.card_rect.contains(position) {
                ctx.submit_action::<DialogAnswer>(DialogAnswer::Cancel);
            }
        }
    }
}
