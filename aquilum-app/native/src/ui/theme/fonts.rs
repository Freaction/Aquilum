use std::borrow::Cow;
use std::sync::{Arc, LazyLock, RwLock};

use masonry::app::RenderRoot;
use masonry::parley::FontFamily;
use masonry::peniko::{Blob, FontData};

use crate::ui::tokens::{number, size};

pub(super) mod bundled {
    include!(concat!(env!("OUT_DIR"), "/fonts.rs"));
}

static UI_FONT: RwLock<Option<Arc<UiFont>>> = RwLock::new(None);
static EDITOR_FONT: RwLock<Option<Arc<EditorFont>>> = RwLock::new(None);

#[derive(Debug)]
pub struct UiFont {
    pub family: String,
    pub size: f32,
    pub weight: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditorFont {
    pub family: String,
    pub size: f32,
    pub weight: f32,
    pub line_height: f32,
    pub max_width_ch: u32,
}

macro_rules! symbols {
    ($stack:literal) => {
        concat!($stack, ", \"Segoe UI Emoji\", \"Apple Color Emoji\", \"Noto Color Emoji\", \"Segoe UI Symbol\", \"Cambria Math\", \"Apple Symbols\", \"STIX Two Math\", \"Noto Sans Math\", \"Noto Sans Symbols 2\", \"DejaVu Sans\"")
    };
}

static BUNDLED: LazyLock<Vec<FontData>> = LazyLock::new(|| bundled::FONTS.iter().map(|data| FontData::new(Blob::new(Arc::new(*data)), 0)).collect());

pub fn bundled_fonts() -> &'static [FontData] {
    &BUNDLED
}

pub fn register_fonts(root: &mut RenderRoot) {
    for font in bundled_fonts() {
        root.register_fonts(font.data.clone());
    }
}

pub fn set_ui_font(font: UiFont) {
    *UI_FONT.write().expect("шрифт интерфейса") = Some(Arc::new(font));
}

pub fn ui_font_settings() -> Arc<UiFont> {
    UI_FONT.read().expect("шрифт интерфейса").clone().unwrap_or_else(|| {
        Arc::new(UiFont {
            family: "Inter".to_owned(),
            size: size::FONT_SIZE_UI_BASE as f32,
            weight: number::FONT_WEIGHT_UI_BASE as f32,
        })
    })
}

pub fn set_editor_font(font: EditorFont) {
    *EDITOR_FONT.write().expect("шрифт редактора") = Some(Arc::new(font));
}

pub fn editor_font_settings() -> Arc<EditorFont> {
    EDITOR_FONT.read().expect("шрифт редактора").clone().unwrap_or_else(|| {
        Arc::new(EditorFont {
            family: "iA Writer Mono".to_owned(),
            size: size::EDITOR_FONT_SIZE as f32,
            weight: number::EDITOR_FONT_WEIGHT as f32,
            line_height: number::EDITOR_LINE_HEIGHT as f32,
            max_width_ch: 65,
        })
    })
}

pub fn installed_families(known: &[&str]) -> Vec<String> {
    use masonry::parley::fontique::{Collection, CollectionOptions};
    let mut collection = Collection::new(CollectionOptions { system_fonts: true, ..CollectionOptions::default() });
    let mut names: Vec<String> = collection.family_names().filter(|name| !name.starts_with(['.', '@']) && !known.contains(name)).map(str::to_owned).collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup();
    names
}

pub fn editor_font(family: &str) -> FontFamily<'static> {
    match family {
        "Inter" => ui_font("Inter"),
        "iA Writer Quattro" => FontFamily::Source(Cow::Borrowed(symbols!("\"iA Writer Quattro\", sans-serif"))),
        "iA Writer Mono" => FontFamily::Source(Cow::Borrowed(symbols!("\"iA Writer Mono\", monospace"))),
        other => FontFamily::Source(Cow::Owned(format!("\"{other}\", {}", symbols!("\"iA Writer Quattro\", sans-serif")))),
    }
}

pub fn ui_font(family: &str) -> FontFamily<'static> {
    let base = symbols!("Inter, \"Inter Cyrillic\", sans-serif");
    let stack = match family {
        "Inter" | "" => base.to_owned(),
        other => format!("\"{other}\", {base}"),
    };
    FontFamily::Source(Cow::Owned(stack))
}
