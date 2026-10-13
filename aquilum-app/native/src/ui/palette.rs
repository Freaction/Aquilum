use masonry::peniko::Color;

use crate::i18n::t;
use crate::ui::tokens::Theme;

pub const NAMES: [&str; 8] = ["red", "amber", "green", "teal", "blue", "purple", "pink", "gray"];

pub fn name(name: &str) -> Option<&'static str> {
    NAMES.iter().copied().find(|known| *known == name)
}

pub fn color(name: &str, theme: &Theme) -> Option<Color> {
    Some(match name {
        "red" => theme.red_500,
        "amber" => theme.amber_500,
        "green" => theme.green_500,
        "teal" => theme.teal_500,
        "blue" => theme.blue_500,
        "purple" => theme.purple_500,
        "pink" => theme.pink_500,
        "gray" => theme.gray_500,
        _ => return None,
    })
}

pub fn label(name: &str) -> String {
    t(&format!("palette.{name}"))
}
