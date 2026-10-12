use std::ops::Range;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontWeight, RangedBuilder};

use super::super::brushes::SYNTAX;
use super::super::metrics::{Fonts, H1, H2};
use super::builder::{hide, push};

pub(super) fn style_heading(builder: &mut RangedBuilder<'_, BrushIndex>, text: &str, from: usize, level: u8, marker: usize, shown: bool, fonts: &Fonts) {
    let scale = match level {
        1 => H1,
        2 => H2,
        _ => 1.0,
    };
    push(builder, StyleProperty::FontSize(fonts.size * scale), from..text.len());
    push(builder, StyleProperty::FontWeight(FontWeight::BOLD), from..text.len());
    let closing = closing_hashes(text, from + marker);
    if shown {
        push(builder, StyleProperty::Brush(SYNTAX), from..from + marker);
        push(builder, StyleProperty::Brush(SYNTAX), closing);
    } else {
        hide(builder, from..from + marker, 0.0);
        hide(builder, closing, 0.0);
    }
}

fn closing_hashes(text: &str, marker: usize) -> Range<usize> {
    let body = text.trim_end();
    let hashes = body.trim_end_matches('#');
    let spaced = hashes.trim_end_matches([' ', '\t']);
    if hashes.len() == body.len() || spaced.len() == hashes.len() || spaced.len() < marker {
        return text.len()..text.len();
    }
    spaced.len()..text.len()
}

#[cfg(test)]
mod tests {
    use super::closing_hashes;

    #[test]
    fn heading_closing_hashes_are_syntax() {
        fn closing(text: &str) -> &str {
            &text[closing_hashes(text, 2)]
        }
        assert_eq!(closing("# Заголовок #"), " #");
        assert_eq!(closing("# Заголовок ###  "), " ###  ");
        assert_eq!(closing("# C#"), "");
        assert_eq!(closing("# Заголовок"), "");
        assert_eq!(closing("# #"), "");
    }
}
