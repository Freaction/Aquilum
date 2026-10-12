use std::collections::HashMap;
use std::sync::Arc;

use krilla::text::{Font, Tag};
use masonry::peniko;
use skrifa::MetadataProvider;

pub struct Fonts {
    fonts: HashMap<(u64, u32, Vec<i16>), (Font, Arc<HashMap<u32, char>>)>,
}

impl Fonts {
    pub fn new() -> Self {
        Fonts { fonts: HashMap::new() }
    }

    pub(super) fn get(&mut self, data: &peniko::FontData, coords: &[i16]) -> Option<(Font, Arc<HashMap<u32, char>>)> {
        let key = (data.data.id(), data.index, coords.to_vec());
        if let Some(found) = self.fonts.get(&key) {
            return Some(found.clone());
        }
        let bytes: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(data.data.clone());
        let parsed = skrifa::FontRef::from_index(data.data.data(), data.index).ok()?;
        let variations: Vec<(Tag, f32)> = parsed
            .axes()
            .iter()
            .zip(coords)
            .map(|(axis, &n)| {
                let n = f32::from(n) / 16384.0;
                let value = if n >= 0.0 { axis.default_value() + n * (axis.max_value() - axis.default_value()) } else { axis.default_value() + n * (axis.default_value() - axis.min_value()) };
                (Tag::new(&axis.tag().to_be_bytes()), value)
            })
            .collect();
        let mut chars = HashMap::new();
        for (code, glyph) in parsed.charmap().mappings() {
            if let Some(c) = char::from_u32(code) {
                chars.entry(glyph.to_u32()).or_insert(c);
            }
        }
        let font = Font::new_variable(krilla::Data::from(bytes), data.index, &variations)?;
        let entry = (font, Arc::new(chars));
        self.fonts.insert(key, entry.clone());
        Some(entry)
    }
}
