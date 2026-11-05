use crate::system::DESKTOP;
use crate::ui::text;
use crate::ui::tokens::{number, size};

pub const BAR_HEIGHT: f64 = if DESKTOP { size::TABS_HEIGHT } else { size::TABS_L_HEIGHT };
pub const RAIL_WIDTH: f64 = if DESKTOP { size::SIDEBAR_RAIL_WIDTH } else { size::SIDEBAR_RAIL_L_WIDTH };

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlSize {
    L,
    M,
    S,
    Xs,
}

impl ControlSize {
    pub const fn fit(self) -> Self {
        self.touch(ControlSize::L)
    }

    pub const fn touch(self, mobile: ControlSize) -> Self {
        if DESKTOP { self } else { mobile }
    }

    pub fn height(self) -> f64 {
        match self {
            ControlSize::L => size::BUTTON_L_HEIGHT,
            ControlSize::M => size::BUTTON_HEIGHT,
            ControlSize::S => size::BUTTON_S_HEIGHT,
            ControlSize::Xs => size::BUTTON_XS_HEIGHT,
        }
    }

    pub fn padding(self) -> f64 {
        match self {
            ControlSize::L => size::BUTTON_L_PADDING_X,
            ControlSize::M => size::BUTTON_PADDING_X,
            ControlSize::S => size::BUTTON_S_PADDING_X,
            ControlSize::Xs => size::BUTTON_XS_PADDING_X,
        }
    }

    pub fn radius(self) -> f64 {
        match self {
            ControlSize::L => size::BUTTON_L_RADIUS,
            ControlSize::M => size::BUTTON_RADIUS,
            ControlSize::S => size::BUTTON_S_RADIUS,
            ControlSize::Xs => size::BUTTON_XS_RADIUS,
        }
    }

    pub fn icon(self) -> f64 {
        if self == ControlSize::L { size::BUTTON_L_ICON_SIZE } else { size::BUTTON_ICON_SIZE }
    }

    pub fn text(self) -> text::Style {
        let line_height = match self {
            ControlSize::L => size::BUTTON_L_LINE_HEIGHT,
            ControlSize::M => size::BUTTON_LINE_HEIGHT,
            ControlSize::S => size::BUTTON_S_LINE_HEIGHT,
            ControlSize::Xs => size::BUTTON_XS_LINE_HEIGHT,
        };
        text::Style { size: size::BUTTON_FONT_SIZE as f32, weight: number::FONT_WEIGHT_UI_BASE as f32, line_height }
    }

    pub fn chrome(self, icons: usize) -> f64 {
        (self.padding() + size::BUTTON_LABEL_PADDING_X) * 2.0 + icons as f64 * self.icon()
    }

    pub fn glyph(self, mode: bool) -> (f64, f64) {
        if mode || matches!(self, ControlSize::L | ControlSize::M) {
            (self.icon(), number::BUTTON_ICON_STROKE)
        } else {
            (size::ICON_BUTTON_ICON_SIZE, number::ICON_BUTTON_ICON_STROKE)
        }
    }

    pub fn field_padding(self) -> (f64, f64) {
        match self {
            ControlSize::L => (size::INPUT_L_PADDING_X, size::INPUT_L_PADDING_Y),
            ControlSize::M | ControlSize::S => (size::INPUT_PADDING_X, size::INPUT_PADDING_Y),
            ControlSize::Xs => (size::INPUT_XS_PADDING_X, size::INPUT_XS_PADDING_Y),
        }
    }

    pub fn field_content(self) -> f64 {
        if self == ControlSize::L { size::INPUT_L_CONTENT_HEIGHT } else { size::INPUT_CONTENT_HEIGHT }
    }

    pub fn field_icon(self) -> f64 {
        if self == ControlSize::L { size::INPUT_L_ICON_SIZE } else { size::INPUT_ICON_SIZE }
    }
}

#[cfg(test)]
mod tests {
    use super::ControlSize::{L, M, S, Xs};
    use super::*;

    #[test]
    fn size_l_follows_the_mobile_layout() {
        assert_eq!((L.height(), L.padding(), L.icon(), L.text().line_height), (40.0, 8.0, 24.0, 24.0), "кнопка с текстом");
        assert_eq!((L.glyph(false).0, M.glyph(false).0), (24.0, 20.0), "иконка в кнопке-иконке L и M");
        let (x, y) = L.field_padding();
        assert_eq!((x, y * 2.0 + L.field_content(), L.field_icon()), (8.0, 40.0, 24.0), "поле и список");
    }

    #[test]
    fn the_desktop_keeps_its_sizes() {
        assert_eq!([L, M, S, Xs].map(ControlSize::fit), [L, M, S, Xs]);
        assert_eq!([M, S, Xs].map(ControlSize::height), [36.0, 32.0, 24.0]);
        assert_eq!((S.glyph(false), S.glyph(true).0, M.field_content()), ((16.0, 1.6), 20.0, 20.0));
        assert_eq!([M, Xs].map(|s| s.field_padding().1 * 2.0 + s.field_content()), [32.0, 28.0]);
        assert_eq!((BAR_HEIGHT, RAIL_WIDTH), (40.0, 40.0));
    }
}
