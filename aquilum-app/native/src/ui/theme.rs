//! Тема окна: токены светлой или тёмной темы, шрифты фронтенда и стеки шрифтов.
//!
//! Тема берётся из настроек ядра (`ui.theme`: `light`, `dark` или `system` — тогда по теме
//! системы), как в рабочей версии.

use std::borrow::Cow;
use std::sync::{Arc, Mutex, RwLock};

use masonry::app::RenderRoot;
use masonry::core::DefaultProperties;
use masonry::properties::{Collapsible, Gap};
use masonry::widgets::{Flex, ScrollBar};
use masonry::parley::FontFamily;
use masonry::peniko::{Blob, Color};

use super::tokens::{self, Theme, number, size};

static CURRENT: RwLock<&'static Theme> = RwLock::new(&tokens::LIGHT);
static DARK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn is_dark() -> bool {
    DARK.load(std::sync::atomic::Ordering::Relaxed)
}
static RECOLORED: Mutex<Vec<(bool, [u8; 3], &'static Theme)>> = Mutex::new(Vec::new());
const DEFAULT_PRIMARY: [u8; 3] = [0x14, 0x71, 0xeb];

/// Токены текущей темы.
pub fn current() -> &'static Theme {
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

pub fn parse_hex(raw: &str) -> Option<[u8; 3]> {
    let hex = raw.trim().strip_prefix('#').unwrap_or(raw.trim());
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

fn recolored(dark: bool, rgb: [u8; 3]) -> &'static Theme {
    let mut cache = RECOLORED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, _, theme)) = cache.iter().find(|(d, c, _)| *d == dark && *c == rgb) {
        return theme;
    }
    let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
    let main = [r, g, b, 1.0];
    let darker = [r * 0.78, g * 0.78, b * 0.78, 1.0];
    let mut theme = if dark { tokens::DARK } else { tokens::LIGHT };
    tokens::recolor(&mut theme, dark, [main, main, darker]);
    let theme: &'static Theme = Box::leak(Box::new(theme));
    cache.push((dark, rgb, theme));
    theme
}

/// Выбирает тему по настройке `ui.theme` (`system` — по `system_dark`) и основному цвету
/// `ui.primaryColor`, как `applyPrimaryColor`.
pub fn apply_setting(setting: &str, system_dark: bool, primary: &str) {
    let dark = match setting {
        "dark" => true,
        "light" => false,
        _ => system_dark,
    };
    let theme = match parse_hex(primary).filter(|rgb| *rgb != DEFAULT_PRIMARY) {
        Some(rgb) => recolored(dark, rgb),
        None if dark => &tokens::DARK,
        None => &tokens::LIGHT,
    };
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = theme;
    DARK.store(dark, std::sync::atomic::Ordering::Relaxed);
}

/// Свойства виджетов Masonry по умолчанию с поправками под дизайн-систему: у `Flex` нет
/// своего промежутка (отступы задаются токенами явно), полосы прокрутки узкие.
pub fn default_properties() -> DefaultProperties {
    let mut properties = masonry::theme::default_property_set();
    properties.insert::<Flex, _>(Gap::ZERO);
    properties.insert::<ScrollBar, _>(Collapsible(true));
    properties
}

/// Фон окна под интерфейсом.
pub fn background() -> Color {
    current().bg_canvas
}

mod fonts {
    include!(concat!(env!("OUT_DIR"), "/fonts.rs"));
}

/// Регистрирует шрифты фронтенда (Inter, iA Writer) в текстовом стеке окна.
pub fn register_fonts(root: &mut RenderRoot) {
    for data in fonts::FONTS {
        root.register_fonts(Blob::new(Arc::new(*data)));
    }
}

/// Шрифт интерфейса из настроек ядра (`ui.fontFamily`, `ui.fontWeight`, `ui.fontSizeBase`),
/// как во фронтенде переопределяет `--q-font-family-ui` и `--q-font-size-ui-base`.
#[derive(Debug)]
pub struct UiFont {
    pub family: String,
    pub size: f32,
    pub weight: f32,
}

static UI_FONT: RwLock<Option<Arc<UiFont>>> = RwLock::new(None);

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

#[derive(Clone, Debug, PartialEq)]
pub struct EditorFont {
    pub family: String,
    pub size: f32,
    pub weight: f32,
    pub line_height: f32,
    pub max_width_ch: u32,
}

static EDITOR_FONT: RwLock<Option<Arc<EditorFont>>> = RwLock::new(None);

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

pub fn editor_font(family: &str) -> FontFamily<'static> {
    match family {
        "Inter" => ui_font("Inter"),
        "iA Writer Quattro" => FontFamily::Source(Cow::Borrowed("\"iA Writer Quattro\", sans-serif")),
        _ => FontFamily::Source(Cow::Borrowed("\"iA Writer Mono\", monospace")),
    }
}

/// Стек шрифта интерфейса для семейства из настроек (`ui.fontFamily`, по умолчанию Inter).
/// Кириллица Inter зарегистрирована отдельным семейством — см. `build.rs`.
pub fn ui_font(family: &str) -> FontFamily<'static> {
    let stack = match family {
        "Inter" | "" => "Inter, \"Inter Cyrillic\", sans-serif".to_owned(),
        other => format!("\"{other}\", Inter, \"Inter Cyrillic\", sans-serif"),
    };
    FontFamily::Source(Cow::Owned(stack))
}

#[cfg(test)]
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
