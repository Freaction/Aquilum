use aquilum_core::settings::models::{AppConfig, FontSettings};

use super::{Control, Number, Row, Setter, Value};
use crate::i18n::{t, t_with};

pub(super) const THEMES: [&str; 3] = ["system", "light", "dark"];
pub(super) const LANGUAGES: [&str; 2] = ["ru", "en"];
pub(super) const READER_FLOWS: [&str; 2] = ["paginated", "scrolled"];
pub(super) const HISTORY_DAYS: [u32; 4] = [0, 30, 90, 365];

const INTER: &str = "Inter";
const QUATTRO: &str = "iA Writer Quattro";
const MONO: &str = "iA Writer Mono";
pub(super) const UI_FAMILIES: [&str; 2] = [INTER, QUATTRO];
pub(super) const EDITOR_FAMILIES: [&str; 3] = [MONO, QUATTRO, INTER];
pub(super) const READER_FAMILIES: [&str; 3] = [QUATTRO, INTER, MONO];
pub(super) const WEIGHTS: [u32; 5] = [400, 450, 500, 550, 600];

pub(super) fn row(label: String, description: Option<String>, control: Control, setter: fn(&mut AppConfig, &Value)) -> Row {
    Row { label, description, control, setter: Some(Setter::Config(setter)), code: None }
}

pub(super) fn info(label: String, description: Option<String>, control: Control) -> Row {
    Row { label, description, control, setter: None, code: None }
}

pub(super) fn index(value: &Value) -> usize {
    match value {
        Value::Index(i) => *i,
        _ => 0,
    }
}

pub(super) fn number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => *n,
        _ => 0.0,
    }
}

pub(super) fn font_size(value: &Value, default: u32) -> u32 {
    match number(value) as u32 {
        0 => default,
        size => size,
    }
}

pub(super) fn flag(value: &Value) -> bool {
    matches!(value, Value::Bool(true))
}

pub(super) fn text(value: &Value) -> String {
    match value {
        Value::Text(s) => s.clone(),
        _ => String::new(),
    }
}

pub(super) fn interval(ms: u32) -> String {
    match ms / 1000 {
        seconds if seconds < 60 => t_with("settings.editor.seconds", &[("count", &seconds.to_string())]),
        seconds => t_with("settings.editor.minutes", &[("count", &(seconds / 60).to_string())]),
    }
}

pub(super) fn position<T: PartialEq>(options: &[T], value: &T) -> usize {
    options.iter().position(|o| o == value).unwrap_or(0)
}

fn weight_label(weight: u32) -> String {
    let key = match weight {
        400 => "settings.font.weightRegular",
        450 => "settings.font.weightText",
        500 => "settings.font.weightMedium",
        550 => "settings.font.weightDense",
        600 => "settings.font.weightSemibold",
        _ => return weight.to_string(),
    };
    t(key)
}

pub(super) fn language_index(language: &str) -> usize {
    usize::from(language != "ru")
}

pub(super) fn families(base: &[&str]) -> Vec<String> {
    base.iter().map(|name| (*name).to_owned()).chain(crate::ui::theme::installed_families(base)).collect()
}

pub(super) fn family(base: &[&str], value: &Value) -> String {
    families(base).into_iter().nth(index(value)).unwrap_or_else(|| base[0].to_owned())
}

pub(super) fn font_rows(
    font: &FontSettings,
    families: &[String],
    set_family: fn(&mut AppConfig, &Value),
    set_weight: fn(&mut AppConfig, &Value),
    set_size: fn(&mut AppConfig, &Value),
) -> Vec<Row> {
    let selected = families.iter().position(|name| *name == font.font_family).unwrap_or(0);
    vec![
        row(t("settings.font.family"), None, Control::Dropdown { options: families.to_vec(), selected }, set_family),
        row(
            t("settings.font.weight"),
            Some(t("settings.font.weightHint")),
            Control::Dropdown {
                options: WEIGHTS.iter().map(|w| weight_label(*w)).collect(),
                selected: position(&WEIGHTS, &font.font_weight),
            },
            set_weight,
        ),
        row(t("settings.font.size"), None, Control::Number(Number::font_size(font.font_size_base)), set_size),
    ]
}
