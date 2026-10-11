use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Code, Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, ScrollDelta, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Line as Segment, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

use super::icons;
use super::menu::box_shadow;
use super::text::{Line, Style};
use super::theme;
use super::tokens::{number, size};
use super::widgets::{ButtonSize, IconButton, TextButton, Variant};
use crate::i18n::t;
use crate::reader::page::{Item, Settings};
use crate::reader::session::{Session, Spot};

mod paint;
mod widget;

const POPOVER_WIDTH: f64 = 352.0;
const PROGRESS: f64 = 3.0;
const WHEEL_STEP: f64 = 120.0;
const TOOLBAR_WIDTH: f64 = 576.0;
const MARK_SCALE: f64 = 0.6;

#[derive(Debug)]
pub enum ReaderAction {
    Close,
    Settings,
    Moved { cfi: String, fraction: f64 },
    QuoteRef(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Prev,
    Next,
}

pub struct ReaderView {
    intro: crate::ui::widgets::Intro,
    session: Session,
    settings: Settings,
    title: String,
    total: u32,
    fraction: f64,
    report: bool,
    wheel: f64,
    settings_button: WidgetPod<IconButton>,
    close: WidgetPod<IconButton>,
    popover: Option<WidgetPod<dyn Widget>>,
    title_line: Line,
    pages_line: Line,
    header: f64,
    body: Rect,
    popover_rect: Rect,
    hover: Option<Option<Side>>,
    zones: [crate::ui::widgets::Tween; 2],
    bar: crate::ui::widgets::Tween,
    selection: Option<(Spot, Spot)>,
    selecting: bool,
    quotes: Vec<(String, String)>,
    marks: Option<(usize, Vec<(Spot, Spot, String, String)>)>,
    labels: Vec<(Rect, String)>,
    quote_button: Option<WidgetPod<TextButton>>,
    toolbar: Rect,
    preview: Rect,
    preview_line: Line,
    mark_lines: Vec<Line>,
}

pub struct ReaderIds {
    pub settings: WidgetId,
    pub close: WidgetId,
    pub quote: Option<WidgetId>,
}

fn nav_width(width: f64) -> f64 {
    (width * 0.08).clamp(40.0, 72.0)
}

impl ReaderView {
    pub fn new(session: Session, settings: Settings, title: String, total: u32, can_quote: bool) -> (NewWidget<Self>, ReaderIds) {
        let settings_button = NewWidget::new(IconButton::new(icons::SETTINGS_2, t("reader.settings")));
        let close = NewWidget::new(IconButton::new(icons::X, t("reader.close")));
        let quote_button = can_quote.then(|| NewWidget::new(TextButton::new(t("reader.quote"), Variant::Primary).size(ButtonSize::S)));
        let ids = ReaderIds { settings: settings_button.id(), close: close.id(), quote: quote_button.as_ref().map(NewWidget::id) };
        let view = ReaderView {
            intro: crate::ui::widgets::Intro::new(crate::ui::widgets::NORMAL),
            session,
            settings,
            title,
            total: total.max(1),
            fraction: 0.0,
            report: true,
            wheel: 0.0,
            settings_button: settings_button.to_pod(),
            close: close.to_pod(),
            popover: None,
            title_line: Line::default(),
            pages_line: Line::default(),
            header: size::SPACE_48,
            body: Rect::ZERO,
            popover_rect: Rect::ZERO,
            hover: None,
            bar: crate::ui::widgets::Tween::new(0.0, crate::ui::widgets::FAST, crate::ui::widgets::LINEAR),
            zones: std::array::from_fn(|_| crate::ui::widgets::Tween::new(0.0, crate::ui::widgets::FAST, crate::ui::widgets::EASE_STANDARD)),
            selection: None,
            selecting: false,
            quotes: Vec::new(),
            marks: None,
            labels: Vec::new(),
            quote_button: quote_button.map(NewWidget::to_pod),
            toolbar: Rect::ZERO,
            preview: Rect::ZERO,
            preview_line: Line::default(),
            mark_lines: Vec::new(),
        };
        (NewWidget::new(view), ids)
    }

    pub fn set_settings(this: &mut WidgetMut<'_, Self>, settings: Settings) {
        if this.widget.settings != settings {
            this.widget.settings = settings;
            this.widget.report = true;
            this.ctx.request_layout();
        }
    }

    pub fn set_popover(this: &mut WidgetMut<'_, Self>, popover: Option<NewWidget<dyn Widget>>) {
        if let Some(old) = this.widget.popover.take() {
            this.ctx.remove_child(old);
        }
        this.widget.popover = popover.map(NewWidget::to_pod);
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    pub fn set_quotes(this: &mut WidgetMut<'_, Self>, quotes: Vec<(String, String)>) {
        this.widget.quotes = quotes;
        this.widget.marks = None;
        this.ctx.request_paint_only();
    }

    pub fn clear_selection(this: &mut WidgetMut<'_, Self>) {
        if this.widget.selection.take().is_some() {
            this.ctx.request_layout();
        }
    }

    pub fn quote(&self) -> Option<(String, String)> {
        let (from, to) = self.ordered()?;
        let text = self.session.text(from, to);
        let cfi = self.session.range_cfi(from, to)?;
        (!text.is_empty()).then_some((text, cfi))
    }

    fn ordered(&self) -> Option<(Spot, Spot)> {
        let (a, b) = self.selection?;
        (a != b).then(|| if a < b { (a, b) } else { (b, a) })
    }

    fn marks(&mut self) -> &[(Spot, Spot, String, String)] {
        let section = self.session.section();
        if self.marks.as_ref().is_none_or(|(s, _)| *s != section) {
            let resolved = self.quotes.iter().filter_map(|(cfi, label)| self.session.resolve(cfi).map(|(a, b)| (a, b, cfi.clone(), label.clone()))).collect();
            self.marks = Some((section, resolved));
        }
        &self.marks.as_ref().expect("метки посчитаны выше").1
    }

    #[cfg(test)]
    pub fn label_rects(&self) -> Vec<Rect> {
        self.labels.iter().map(|(r, _)| *r).collect()
    }

    pub fn has_popover(&self) -> bool {
        self.popover.is_some()
    }

    fn current(&self) -> u32 {
        ((self.fraction.clamp(0.0, 1.0) * f64::from(self.total)).round() as u32).min(self.total)
    }

    fn moved(&mut self, ctx: &mut EventCtx<'_>, changed: bool) {
        if !changed {
            return;
        }
        self.report = true;
        if self.selection.take().is_some() {
            ctx.request_layout();
        }
        if self.session.needs_layout() {
            ctx.request_layout();
        } else {
            self.emit(ctx);
            ctx.request_paint_only();
        }
    }

    fn emit(&mut self, ctx: &mut impl Submit) {
        if !std::mem::take(&mut self.report) {
            return;
        }
        if let Some((cfi, fraction)) = self.session.location() {
            self.fraction = fraction;
            if self.bar.set(f64::from(self.current()) / f64::from(self.total.max(1))) && !ctx.animate() {
                self.bar.finish();
            }
            ctx.submit(ReaderAction::Moved { cfi, fraction });
        }
    }

    fn side(&self, p: Point) -> Option<Side> {
        if !self.body.contains(p) {
            return None;
        }
        let nav = nav_width(self.body.width());
        if p.x < self.body.x0 + nav {
            Some(Side::Prev)
        } else if p.x > self.body.x1 - nav {
            Some(Side::Next)
        } else {
            None
        }
    }

    fn turn(&mut self, ctx: &mut EventCtx<'_>, side: Side) {
        let changed = match side {
            Side::Prev => self.session.prev(),
            Side::Next => self.session.next(),
        };
        self.moved(ctx, changed);
    }

    fn line_height() -> f64 {
        (f64::from(theme::ui_font_settings().size) * number::FONT_LINE_HEIGHT_NORMAL).round()
    }
}

trait Submit {
    fn submit(&mut self, action: ReaderAction);
    fn animate(&mut self) -> bool;
}

impl Submit for EventCtx<'_> {
    fn submit(&mut self, action: ReaderAction) {
        self.submit_action::<ReaderAction>(action);
    }

    fn animate(&mut self) -> bool {
        self.request_anim_frame();
        true
    }
}

impl Submit for LayoutCtx<'_> {
    fn submit(&mut self, action: ReaderAction) {
        self.submit_action::<ReaderAction>(action);
    }

    fn animate(&mut self) -> bool {
        false
    }
}

impl ReaderView {
    fn set_hover(&mut self, ctx: &mut EventCtx<'_>, hover: Option<Option<Side>>) {
        if hover == self.hover {
            return;
        }
        self.hover = hover;
        let mut moving = false;
        for (zone, side) in self.zones.iter_mut().zip([Side::Prev, Side::Next]) {
            let target = match hover {
                None => 0.0,
                Some(over) if over == Some(side) => 1.0,
                Some(_) => 0.7,
            };
            moving |= zone.set(target);
        }
        if moving {
            ctx.request_anim_frame();
        }
        ctx.request_paint_only();
    }
}
