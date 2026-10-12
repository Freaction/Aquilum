use super::*;

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

    pub(super) fn ordered(&self) -> Option<(Spot, Spot)> {
        let (a, b) = self.selection?;
        (a != b).then(|| if a < b { (a, b) } else { (b, a) })
    }

    pub(super) fn marks(&mut self) -> &[(Spot, Spot, String, String)] {
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

    pub(super) fn current(&self) -> u32 {
        ((self.fraction.clamp(0.0, 1.0) * f64::from(self.total)).round() as u32).min(self.total)
    }

    pub(super) fn line_height() -> f64 {
        (f64::from(theme::ui_font_settings().size) * number::FONT_LINE_HEIGHT_NORMAL).round()
    }
}
