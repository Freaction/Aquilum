use std::path::PathBuf;

use aquilum_core::mcp::McpStatus;
use aquilum_core::settings::models::AppConfig;

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
    pub(super) fn new(label: String, action: Action) -> Self {
        Button { label, action, enabled: true }
    }

    pub(super) fn enabled(mut self, enabled: bool) -> Self {
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
    pub(super) fn new(value: f64, min: f64, max: f64, step: f64) -> Self {
        Number { value, min, max, step, stepper: false, commit: false, blank: false }
    }

    pub(super) fn font_size(value: u32) -> Self {
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

pub(super) fn round2(value: f64) -> f64 {
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
