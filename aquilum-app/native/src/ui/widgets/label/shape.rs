use masonry::core::BrushIndex;
use masonry::parley::style::StyleProperty;
use masonry::parley::{Alignment, AlignmentOptions, FontContext, FontFamily, FontWeight, Layout, LayoutContext, LineHeight, TextWrapMode};

use super::{Font, Label, Leading, Scale};
use crate::ui::theme;

impl Label {
    fn style(&self) -> (FontFamily<'static>, f32, f32) {
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
        (family, size, self.weight.unwrap_or(weight))
    }

    pub(super) fn layout(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, width: Option<f32>) -> &Layout<BrushIndex> {
        let width = width.filter(|_| self.wrap || self.centered);
        let key = width.map(f32::to_bits);
        if self.cached.as_ref().is_none_or(|(k, _)| *k != key) {
            let (family, size, weight) = self.style();
            let line = match self.leading {
                Leading::Px(px) => px,
                Leading::Times(k) => (size * k).round(),
                Leading::Editor => size * theme::editor_font_settings().line_height,
            };
            let mut builder = lcx.ranged_builder(fcx, &self.text, 1.0, true);
            builder.push_default(StyleProperty::FontFamily(family));
            builder.push_default(StyleProperty::FontSize(size));
            builder.push_default(StyleProperty::FontWeight(FontWeight::new(weight)));
            builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(line)));
            if !self.wrap {
                builder.push_default(StyleProperty::TextWrapMode(TextWrapMode::NoWrap));
            }
            let mut layout = builder.build(&self.text);
            layout.break_all_lines(width);
            if self.centered {
                layout.align(Alignment::Center, AlignmentOptions::default());
            }
            self.cached = Some((key, layout));
        }
        &self.cached.as_ref().expect("раскладка построена выше").1
    }
}
