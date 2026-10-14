use masonry::peniko::Color;

use crate::i18n::t;
use crate::ui::tokens::Theme;

pub const NAMES: [&str; 8] = ["red", "amber", "green", "teal", "blue", "purple", "pink", "gray"];

pub const TAG_COLORS: usize = 7;

pub fn tag(name: &str) -> usize {
    let mut hash: u32 = 2_166_136_261;
    for byte in name.to_lowercase().bytes() {
        hash = (hash ^ u32::from(byte)).wrapping_mul(16_777_619);
    }
    hash as usize % TAG_COLORS
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_always_gets_the_same_color_whatever_its_case() {
        assert_eq!(tag("работа"), tag("Работа"));
        assert_eq!(tag("work"), tag("WORK"));
        assert!((0..100).all(|i| tag(&format!("тег{i}")) < TAG_COLORS));
    }

    #[test]
    fn tags_spread_over_the_whole_palette_without_the_gray() {
        let used: std::collections::BTreeSet<usize> = (0..200).map(|i| tag(&format!("тег{i}"))).collect();
        assert_eq!(used.len(), TAG_COLORS);
        assert!(NAMES[..TAG_COLORS].iter().all(|name| *name != "gray"));
    }
}
