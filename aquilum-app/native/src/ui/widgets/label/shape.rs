use masonry::core::BrushIndex;
use masonry::parley::style::StyleProperty;
use masonry::parley::{Alignment, AlignmentOptions, FontContext, FontFamily, Layout, LayoutContext, TextWrapMode};

use super::{Font, Label, Leading, Scale};
use crate::ui::{text, theme};

fn fit(layout: &mut Layout<BrushIndex>, width: Option<f32>, centered: bool) {
    layout.break_all_lines(width);
    if centered {
        layout.align(Alignment::Center, AlignmentOptions::default());
    }
}

impl Label {
    fn style(&self) -> (FontFamily<'static>, text::Style) {
        let (family, base, weight) = match self.font {
            Font::Ui => {
                let font = theme::ui_font_settings();
                (theme::ui_font(&font.family), font.size, font.weight)
            }
            Font::Editor => {
                let font = theme::editor_font_settings();
                (theme::editor_font(&font.family), font.size, font.weight)
            }
            Font::Mono => {
                let font = theme::ui_font_settings();
                (theme::editor_font("iA Writer Mono"), font.size, font.weight)
            }
        };
        let size = match self.scale {
            Scale::Base => base,
            Scale::Small => (base * 12.0 / 14.0).round(),
            Scale::Px(px) => px,
        };
        let line = match self.leading {
            Leading::Px(px) => px,
            Leading::Times(k) => (size * k).round(),
            Leading::Editor => size * theme::editor_font_settings().line_height,
        };
        (family, text::Style { size, weight: self.weight.unwrap_or(weight), line_height: f64::from(line) })
    }

    pub(super) fn layout(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, width: Option<f32>) -> &Layout<BrushIndex> {
        let width = width.filter(|_| self.wrap || self.centered);
        let key = width.map(f32::to_bits);
        let centered = self.centered;
        if self.cached.is_none() {
            let (family, style) = self.style();
            let mut builder = text::builder(fcx, lcx, &self.text, family, style);
            if !self.wrap {
                builder.push_default(StyleProperty::TextWrapMode(TextWrapMode::NoWrap));
            }
            let mut layout = builder.build(&self.text);
            fit(&mut layout, width, centered);
            self.cached = Some((key, layout));
        }
        let (at, layout) = self.cached.as_mut().expect("раскладка построена выше");
        if *at != key {
            fit(layout, width, centered);
            *at = key;
        }
        layout
    }
}
