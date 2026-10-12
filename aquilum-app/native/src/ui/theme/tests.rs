use std::sync::Arc;

use masonry::peniko::{Blob, Color};

use super::colors::DEFAULT_PRIMARY;
use super::fonts::bundled;
use super::parse_hex;
use crate::ui::tokens;

#[test]
fn bundled_fonts_answer_to_the_names_styles_ask_for() {
    use masonry::parley::fontique::{Collection, CollectionOptions};
    let mut collection = Collection::new(CollectionOptions { system_fonts: false, ..CollectionOptions::default() });
    let mut names = Vec::new();
    for data in bundled::FONTS {
        for (id, infos) in collection.register_fonts(Blob::new(Arc::new(*data)), None) {
            let italic = infos.iter().any(|info| info.style() != masonry::parley::fontique::FontStyle::Normal);
            names.extend(collection.family_name(id).map(|name| (name.to_owned(), italic)));
        }
    }
    for wanted in ["Inter", "Inter Cyrillic", "iA Writer Mono", "iA Writer Quattro"] {
        for italic in [false, true] {
            assert!(names.iter().any(|(name, i)| name == wanted && *i == italic), "нет {wanted} (курсив: {italic}), есть {names:?}");
        }
    }
}

#[cfg(target_os = "windows")]
fn has_table(data: &[u8], index: u32, tag: &[u8; 4]) -> bool {
    let read = |at: usize| u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
    let base = if &data[..4] == b"ttcf" { read(12 + 4 * index as usize) } else { 0 };
    let tables = u16::from_be_bytes([data[base + 4], data[base + 5]]) as usize;
    (0..tables).any(|i| &data[base + 12 + 16 * i..base + 16 + 16 * i] == tag)
}

#[cfg(target_os = "windows")]
#[test]
fn symbols_and_emoji_find_a_font() {
    use super::{editor_font, ui_font};
    use masonry::parley::{FontContext, LayoutContext, PositionedLayoutItem, StyleProperty};
    let mut fcx = FontContext::default();
    for data in bundled::FONTS {
        fcx.collection.register_fonts(Blob::new(Arc::new(*data)), None);
    }
    let mut lcx = LayoutContext::<masonry::core::BrushIndex>::new();
    for family in [editor_font("iA Writer Mono"), editor_font("iA Writer Quattro"), ui_font("Inter")] {
        for text in ["∈ ∉ ⊂ ∪ ⇒ ⇔ ∑ ∏ ∫", "😀🚀❤️👍"] {
            let mut builder = lcx.ranged_builder(&mut fcx, text, 1.0, true);
            builder.push_default(StyleProperty::FontFamily(family.clone()));
            let mut layout = builder.build(text);
            layout.break_all_lines(None);
            for line in layout.lines() {
                for item in line.items() {
                    if let PositionedLayoutItem::GlyphRun(run) = item {
                        let font = run.run().font();
                        assert!(run.glyphs().all(|g| g.id != 0), "{text}: пустые глифы в стеке {family:?}");
                        if text.starts_with('😀') {
                            assert!(has_table(font.data.data(), font.index, b"COLR"), "эмодзи взяты из чёрно-белого шрифта");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn default_primary_recolors_to_original_tokens() {
    let close = |a: Color, b: Color| a.to_rgba8().to_u8_array().iter().zip(b.to_rgba8().to_u8_array()).all(|(x, y)| x.abs_diff(y) <= 1);
    for (dark, original) in [(false, tokens::LIGHT), (true, tokens::DARK)] {
        let [r, g, b] = DEFAULT_PRIMARY.map(|c| f32::from(c) / 255.0);
        let mut theme = original;
        tokens::recolor(&mut theme, dark, [[r, g, b, 1.0], [r, g, b, 1.0], [r * 0.78, g * 0.78, b * 0.78, 1.0]]);
        assert!(close(theme.bg_accent, original.bg_accent));
        assert!(close(theme.bg_accent_subtle, original.bg_accent_subtle));
        assert!(close(theme.text_link, original.text_link));
        assert!(close(theme.border_focus_ring, original.border_focus_ring));
    }
    let mut theme = tokens::LIGHT;
    tokens::recolor(&mut theme, false, [[1.0, 0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0], [0.78, 0.0, 0.0, 1.0]]);
    assert_eq!(theme.bg_accent.to_rgba8().to_u8_array(), [255, 0, 0, 255]);
    assert_eq!(parse_hex("#E05A2B"), Some([0xe0, 0x5a, 0x2b]));
    assert_eq!(parse_hex("e05a2"), None);
}
