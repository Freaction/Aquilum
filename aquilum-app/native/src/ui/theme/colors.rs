use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use masonry::core::DefaultProperties;
use masonry::peniko::Color;
use masonry::properties::{Collapsible, Gap};
use masonry::widgets::{Flex, ScrollBar};

use crate::ui::tokens::{self, Theme};

static CURRENT: RwLock<Option<Arc<Theme>>> = RwLock::new(None);
static DARK: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicUsize = AtomicUsize::new(0);
static RECOLORED: Mutex<Vec<(bool, [u8; 3], Arc<Theme>)>> = Mutex::new(Vec::new());

const RECOLORED_KEPT: usize = 4;
pub(super) const DEFAULT_PRIMARY: [u8; 3] = [0x14, 0x71, 0xeb];

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

fn base(dark: bool) -> Arc<Theme> {
    static BASE: OnceLock<(Arc<Theme>, Arc<Theme>)> = OnceLock::new();
    let (light, dark_theme) = BASE.get_or_init(|| (Arc::new(tokens::LIGHT), Arc::new(tokens::DARK)));
    Arc::clone(if dark { dark_theme } else { light })
}

pub fn generation() -> usize {
    GENERATION.load(Ordering::Relaxed)
}

pub fn current() -> Arc<Theme> {
    CURRENT.read().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_else(|| base(false))
}

pub fn parse_hex(raw: &str) -> Option<[u8; 3]> {
    let hex = raw.trim().strip_prefix('#').unwrap_or(raw.trim());
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

fn recolored(dark: bool, rgb: [u8; 3]) -> Arc<Theme> {
    let mut cache = RECOLORED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, _, theme)) = cache.iter().find(|(d, c, _)| *d == dark && *c == rgb) {
        return Arc::clone(theme);
    }
    let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
    let main = [r, g, b, 1.0];
    let darker = [r * 0.78, g * 0.78, b * 0.78, 1.0];
    let mut theme = if dark { tokens::DARK } else { tokens::LIGHT };
    tokens::recolor(&mut theme, dark, [main, main, darker]);
    let theme = Arc::new(theme);
    if cache.len() >= RECOLORED_KEPT {
        cache.remove(0);
    }
    cache.push((dark, rgb, Arc::clone(&theme)));
    theme
}

pub fn apply_setting(setting: &str, system_dark: bool, primary: &str) {
    let dark = match setting {
        "dark" => true,
        "light" => false,
        _ => system_dark,
    };
    let theme = match parse_hex(primary).filter(|rgb| *rgb != DEFAULT_PRIMARY) {
        Some(rgb) => recolored(dark, rgb),
        None => base(dark),
    };
    let mut current = CURRENT.write().unwrap_or_else(|e| e.into_inner());
    if current.as_ref().is_none_or(|old| !Arc::ptr_eq(old, &theme)) {
        GENERATION.fetch_add(1, Ordering::Relaxed);
    }
    *current = Some(theme);
    DARK.store(dark, Ordering::Relaxed);
}

pub fn default_properties() -> DefaultProperties {
    let mut properties = masonry::theme::default_property_set();
    properties.insert::<Flex, _>(Gap::ZERO);
    properties.insert::<ScrollBar, _>(Collapsible(true));
    crate::ui::style::install(&mut properties, &current());
    properties
}

pub fn background() -> Color {
    current().bg_canvas
}
