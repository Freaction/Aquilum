use std::sync::OnceLock;

use ttf_parser::Face;

use crate::unicode_width::{Cluster, consume_cluster, is_cjk_wide_char};

static FACES: OnceLock<Vec<Face<'static>>> = OnceLock::new();

pub fn use_fonts(fonts: &[&'static [u8]]) {
    let _ = FACES.set(fonts.iter().filter_map(|data| Face::parse(data, 0).ok()).collect());
}

pub fn measure_text_width(text: &str, font_size: f32, _font_family: &str) -> Option<f32> {
    if text.is_empty() || font_size <= 0.0 {
        return Some(0.0);
    }
    let faces = FACES.get().filter(|faces| !faces.is_empty())?;
    let chars: Vec<char> = text.chars().collect();
    let mut width = 0.0f32;
    let mut idx = 0usize;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch == '\n' {
            idx += 1;
            continue;
        }
        if let Some((kind, next)) = consume_cluster(&chars, idx) {
            width += match kind {
                Cluster::Wide => font_size,
                Cluster::ZeroWidth => 0.0,
            };
            idx = next;
            continue;
        }
        let ch = if ch == '\t' { ' ' } else { ch };
        let advance = faces.iter().find_map(|face| {
            let glyph = face.glyph_index(ch)?;
            Some(f32::from(face.glyph_hor_advance(glyph)?) * font_size / f32::from(face.units_per_em().max(1)))
        });
        width += match advance {
            Some(advance) if chars[idx] == '\t' => advance * 4.0,
            Some(advance) => advance,
            None if is_cjk_wide_char(ch) => font_size,
            None => font_size * 0.56,
        };
        idx += 1;
    }
    Some(width)
}
