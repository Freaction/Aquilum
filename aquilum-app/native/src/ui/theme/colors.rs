use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use masonry::core::DefaultProperties;
use masonry::peniko::Color;
use masonry::properties::{Collapsible, Gap};
use masonry::widgets::{Flex, ScrollBar};

use aquilum_core::recent::Recent;

use crate::ui::tokens::{self, Theme};

static CURRENT: RwLock<Option<Arc<Theme>>> = RwLock::new(None);
static DARK: AtomicBool = AtomicBool::new(false);
static RED: AtomicBool = AtomicBool::new(false);
static PRIMARY: Mutex<[u8; 3]> = Mutex::new(DEFAULT_PRIMARY);
static GENERATION: AtomicUsize = AtomicUsize::new(0);
static RECOLORED: Mutex<Recent<(bool, bool, [u8; 3]), Arc<Theme>>> = Mutex::new(Recent::new(4, usize::MAX));
pub(super) const DEFAULT_PRIMARY: [u8; 3] = [0x14, 0x71, 0xeb];

thread_local! {
    static PAPER: std::cell::RefCell<Option<Arc<Theme>>> = const { std::cell::RefCell::new(None) };
}

fn printing_now() -> bool {
    PAPER.with_borrow(Option::is_some)
}

pub fn is_dark() -> bool {
    !printing_now() && DARK.load(Ordering::Relaxed)
}

pub fn is_red() -> bool {
    !printing_now() && RED.load(Ordering::Relaxed)
}

pub fn printing<T>(print: impl FnOnce() -> T) -> T {
    let paper = themed(false, false, *PRIMARY.lock().unwrap_or_else(|e| e.into_inner()));
    PAPER.with_borrow_mut(|current| *current = Some(paper));
    let printed = print();
    PAPER.with_borrow_mut(|current| *current = None);
    printed
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
    if let Some(paper) = PAPER.with_borrow(Clone::clone) {
        return paper;
    }
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

fn recolored(dark: bool, red: bool, rgb: [u8; 3]) -> Arc<Theme> {
    let mut cache = RECOLORED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(theme) = cache.get(&(dark, red, rgb)) {
        return Arc::clone(theme);
    }
    let mut theme = if dark { tokens::DARK } else { tokens::LIGHT };
    if rgb != DEFAULT_PRIMARY {
        let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
        let main = [r, g, b, 1.0];
        let darker = [r * 0.78, g * 0.78, b * 0.78, 1.0];
        tokens::recolor(&mut theme, dark, [main, main, darker]);
    }
    if red {
        tokens::map_colors(&mut theme, vello_cpu::red);
    }
    let theme = Arc::new(theme);
    cache.put((dark, red, rgb), Arc::clone(&theme), 0);
    theme
}

fn themed(dark: bool, red: bool, rgb: [u8; 3]) -> Arc<Theme> {
    if red || rgb != DEFAULT_PRIMARY { recolored(dark, red, rgb) } else { base(dark) }
}

pub fn apply_setting(setting: &str, system_dark: bool, primary: &str) {
    let (dark, red) = match setting {
        "dark" => (true, false),
        "light" => (false, false),
        "red" => (true, true),
        _ => (system_dark, false),
    };
    let rgb = parse_hex(primary).unwrap_or(DEFAULT_PRIMARY);
    *PRIMARY.lock().unwrap_or_else(|e| e.into_inner()) = rgb;
    let theme = themed(dark, red, rgb);
    let mut current = CURRENT.write().unwrap_or_else(|e| e.into_inner());
    if current.as_ref().is_none_or(|old| !Arc::ptr_eq(old, &theme)) {
        GENERATION.fetch_add(1, Ordering::Relaxed);
    }
    *current = Some(theme);
    DARK.store(dark, Ordering::Relaxed);
    RED.store(red, Ordering::Relaxed);
    crate::system::set_dark_theme(dark);
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
