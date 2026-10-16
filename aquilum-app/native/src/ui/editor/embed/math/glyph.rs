use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use masonry::peniko::{Blob, FontData};
use ratex_font::FontId;
use skrifa::{FontRef, MetadataProvider};

static KATEX: LazyLock<Mutex<HashMap<FontId, Option<FontData>>>> = LazyLock::new(Mutex::default);

pub(super) fn glyph(font: &str, code: u32) -> Option<(FontData, u32)> {
    let ch = char::from_u32(code)?;
    let katex = FontId::parse(font).and_then(|id| {
        let mut faces = KATEX.lock().unwrap_or_else(PoisonError::into_inner);
        faces.entry(id).or_insert_with(|| ratex_katex_fonts::ttf_bytes(&format!("KaTeX_{font}.ttf")).map(|bytes| FontData::new(Blob::new(Arc::new(bytes)), 0))).clone()
    });
    katex.into_iter().chain(crate::ui::theme::bundled_fonts().iter().cloned()).find_map(|face| id(&face, ch).map(|id| (face, id)))
}

fn id(face: &FontData, ch: char) -> Option<u32> {
    Some(FontRef::from_index(face.data.data(), face.index).ok()?.charmap().map(ch)?.to_u32())
}
