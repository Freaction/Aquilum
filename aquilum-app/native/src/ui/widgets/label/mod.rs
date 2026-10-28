mod select;
mod shape;
#[cfg(test)]
mod tests;
mod widget;

use masonry::core::{BrushIndex, NewWidget};
use masonry::parley::{Layout, Selection};
use masonry::peniko::Color;

use crate::ui::tokens::{Theme, size};

pub type Paint = fn(&Theme) -> Color;

#[derive(Clone, Copy, Debug)]
pub enum Leading {
    Px(f32),
    Times(f32),
    Editor,
}

#[derive(Clone, Copy, Debug)]
pub enum Scale {
    Base,
    Small,
    Px(f32),
}

#[derive(Clone, Copy, Debug)]
pub enum Font {
    Ui,
    Editor,
    Mono,
}

pub struct Label {
    text: String,
    paint: Paint,
    background: Option<Paint>,
    weight: Option<f32>,
    leading: Leading,
    scale: Scale,
    font: Font,
    centered: bool,
    wrap: bool,
    selectable: bool,
    selection: Selection,
    cached: Option<(Option<u32>, Layout<BrushIndex>)>,
}

impl Label {
    pub fn new(text: &str, paint: Paint) -> Self {
        Label {
            text: text.to_owned(),
            paint,
            background: None,
            weight: None,
            leading: Leading::Px(size::SIZE_20 as f32),
            scale: Scale::Base,
            font: Font::Ui,
            centered: false,
            wrap: true,
            selectable: false,
            selection: Selection::default(),
            cached: None,
        }
    }

    #[cfg(test)]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(this: &mut masonry::core::WidgetMut<'_, Self>, text: &str) {
        if this.widget.text != text {
            text.clone_into(&mut this.widget.text);
            this.widget.cached = None;
            this.widget.selection = Selection::default();
            this.ctx.request_layout();
            this.ctx.request_accessibility_update();
        }
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn leading(mut self, leading: Leading) -> Self {
        self.leading = leading;
        self
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = scale;
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub fn nowrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    pub fn background(mut self, paint: Paint) -> Self {
        self.background = Some(paint);
        self
    }
}

impl From<Label> for NewWidget<Label> {
    fn from(label: Label) -> Self {
        NewWidget::new(label)
    }
}
