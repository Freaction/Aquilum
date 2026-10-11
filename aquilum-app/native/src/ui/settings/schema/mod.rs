use std::path::{Path, PathBuf};

use aquilum_core::files::deletions::{TrashedDeletion, deletion_page_impl};
use aquilum_core::files::trash::trash_state_impl;
use aquilum_core::mcp::McpStatus;
use aquilum_core::settings::models::AppConfig;

use crate::i18n::{self, plural, t, t_with};
use crate::interface::{MAX_SCALE, MIN_SCALE, SCALE_STEP};
use crate::ui::icons::{self, Icon};

mod appearance;
mod data;
mod mcp;
mod system;

pub use appearance::reader_quick;
pub use data::templates_path;
pub use mcp::new_token;
use appearance::*;
use data::*;
use mcp::*;
use system::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Ui,
    Editor,
    Reader,
    Search,
    Templates,
    Files,
    Analysis,
    Mcp,
    History,
    Trash,
    System,
    Shortcuts,
}

impl Section {
    pub const ALL: [Section; 12] = [
        Section::Ui,
        Section::Editor,
        Section::Reader,
        Section::Search,
        Section::Templates,
        Section::Files,
        Section::Analysis,
        Section::Mcp,
        Section::History,
        Section::Trash,
        Section::System,
        Section::Shortcuts,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Section::Ui => "ui",
            Section::Editor => "editor",
            Section::Reader => "reader",
            Section::Search => "search",
            Section::Templates => "templates",
            Section::Files => "files",
            Section::Analysis => "analysis",
            Section::Mcp => "mcp",
            Section::History => "history",
            Section::Trash => "trash",
            Section::System => "system",
            Section::Shortcuts => "shortcuts",
        }
    }

    pub fn from_key(key: &str) -> Option<Section> {
        Section::ALL.into_iter().find(|s| s.key() == key)
    }

    pub fn label(self) -> String {
        t(&format!("settings.nav.{}", self.key()))
    }

    pub fn icon(self) -> Icon {
        match self {
            Section::Ui => icons::MONITOR,
            Section::Editor => icons::TYPE,
            Section::Reader => icons::BOOK_OPEN,
            Section::Search => icons::SEARCH,
            Section::Templates => icons::FILE_TEXT,
            Section::Files => icons::PAPERCLIP,
            Section::Analysis => icons::GRAPH_ANALYSIS,
            Section::Mcp => icons::PLUG,
            Section::History => icons::HISTORY,
            Section::Trash => icons::TRASH,
            Section::System => icons::POWER,
            Section::Shortcuts => icons::KEYBOARD,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Index(usize),
    Number(f64),
    Text(String),
}

#[derive(Clone, Copy)]
pub enum Setter {
    Config(fn(&mut AppConfig, &Value)),
    Scale,
    HomePage,
    Autostart,
}

pub struct Context {
    pub workspace: Option<PathBuf>,
    pub scale: f64,
    pub home_page: String,
    pub mcp: Option<McpStatus>,
    pub copied: Option<usize>,
    pub trash_page: usize,
    pub autostart: Option<bool>,
    pub update: Option<crate::updater::Status>,
}

impl Context {
    #[cfg(test)]
    pub fn new(scale: f64) -> Self {
        Context { workspace: None, scale, home_page: String::new(), mcp: None, copied: None, trash_page: 0, autostart: None, update: None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    OpenTrash,
    Restore(String),
    TrashPage(usize),
    CreateTemplates,
    NewToken,
    Copy(usize, String),
    OpenUrl(&'static str),
    CheckUpdates,
    InstallUpdate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    pub label: String,
    pub action: Action,
    pub enabled: bool,
}

impl Button {
    fn new(label: String, action: Action) -> Self {
        Button { label, action, enabled: true }
    }

    fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tone {
    Muted,
    Secondary,
    Danger,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Number {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub stepper: bool,
    pub commit: bool,
    pub blank: bool,
}

impl Number {
    fn new(value: f64, min: f64, max: f64, step: f64) -> Self {
        Number { value, min, max, step, stepper: false, commit: false, blank: false }
    }

    fn font_size(value: u32) -> Self {
        Number { blank: true, ..Number::new(f64::from(value), 12.0, 32.0, 1.0) }
    }

    pub fn parse(&self, text: &str) -> Option<f64> {
        let text = text.trim();
        if text.is_empty() {
            return self.blank.then_some(0.0);
        }
        text.replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite()).map(|v| v.clamp(self.min, self.max))
    }

    pub fn step_from(&self, value: f64, direction: f64) -> f64 {
        round2(value + direction * self.step).clamp(self.min, self.max)
    }
}

pub fn format_number(value: f64) -> String {
    let text = format!("{:.2}", round2(value));
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    Switch(bool),
    Locked(bool),
    Segmented { options: Vec<String>, selected: usize },
    Dropdown { options: Vec<String>, selected: usize },
    Number(Number),
    Text { value: String, commit: bool },
    Color(String),
    Kbd(String),
    Buttons(Vec<Button>),
    Status { text: String, tone: Tone },
}

pub struct Row {
    pub label: String,
    pub description: Option<String>,
    pub control: Control,
    pub setter: Option<Setter>,
    pub code: Option<String>,
}

pub struct Block {
    pub title: String,
    pub rows: Vec<Row>,
}

fn row(label: String, description: Option<String>, control: Control, setter: fn(&mut AppConfig, &Value)) -> Row {
    Row { label, description, control, setter: Some(Setter::Config(setter)), code: None }
}

fn info(label: String, description: Option<String>, control: Control) -> Row {
    Row { label, description, control, setter: None, code: None }
}

fn index(value: &Value) -> usize {
    match value {
        Value::Index(i) => *i,
        _ => 0,
    }
}

fn number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => *n,
        _ => 0.0,
    }
}

fn font_size(value: &Value, default: u32) -> u32 {
    match number(value) as u32 {
        0 => default,
        size => size,
    }
}

fn flag(value: &Value) -> bool {
    matches!(value, Value::Bool(true))
}

fn text(value: &Value) -> String {
    match value {
        Value::Text(s) => s.clone(),
        _ => String::new(),
    }
}

fn position<T: PartialEq>(options: &[T], value: &T) -> usize {
    options.iter().position(|o| o == value).unwrap_or(0)
}

const THEMES: [&str; 3] = ["system", "light", "dark"];
const LANGUAGES: [&str; 2] = ["ru", "en"];
const READER_FLOWS: [&str; 2] = ["paginated", "scrolled"];
const HISTORY_DAYS: [u32; 4] = [0, 30, 90, 365];

const INTER: &str = "Inter";
const QUATTRO: &str = "iA Writer Quattro";
const MONO: &str = "iA Writer Mono";
const UI_FAMILIES: [&str; 2] = [INTER, QUATTRO];
const EDITOR_FAMILIES: [&str; 3] = [MONO, QUATTRO, INTER];
const READER_FAMILIES: [&str; 3] = [QUATTRO, INTER, MONO];
const WEIGHTS: [u32; 5] = [400, 450, 500, 550, 600];

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

fn language_index(language: &str) -> usize {
    usize::from(language != "ru")
}

fn font_rows(
    font: &aquilum_core::settings::models::FontSettings,
    families: &[&str],
    set_family: fn(&mut AppConfig, &Value),
    set_weight: fn(&mut AppConfig, &Value),
    set_size: fn(&mut AppConfig, &Value),
) -> Vec<Row> {
    let options = families.iter().map(|f| (*f).to_owned()).collect();
    vec![
        row(t("settings.font.family"), None, Control::Dropdown { options, selected: position(families, &font.font_family.as_str()) }, set_family),
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

pub fn blocks(section: Section, config: &AppConfig, ctx: &Context) -> Vec<Block> {
    match section {
        Section::Ui => ui(config, ctx),
        Section::Editor => editor(config),
        Section::Reader => reader(config),
        Section::Search => search(config),
        Section::Templates => templates(config, ctx),
        Section::Files => files(config),
        Section::Analysis => analysis(config),
        Section::Mcp => mcp(config, ctx),
        Section::History => history(config),
        Section::Trash => trash(config, ctx),
        Section::System => system(config, ctx),
        Section::Shortcuts => shortcuts(),
    }
}
