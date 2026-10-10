use std::path::{Path, PathBuf};

use aquilum_core::files::deletions::{TrashedDeletion, deletion_page_impl};
use aquilum_core::files::trash::trash_state_impl;
use aquilum_core::mcp::McpStatus;
use aquilum_core::settings::models::AppConfig;

use crate::i18n::{self, plural, t, t_with};
use crate::interface::{MAX_SCALE, MIN_SCALE, SCALE_STEP};
use crate::ui::icons::{self, Icon};

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
        Number { blank: true, ..Number::new(f64::from(value), 0.0, 32.0, 1.0) }
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
    usize::from(language == "en")
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

fn ui(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let scale = ctx.scale;
    let scale_row = Row {
        label: t("settings.ui.scale"),
        description: Some(t_with("settings.ui.scaleHint", &[("reset", "Ctrl + 0")])),
        control: Control::Number(Number {
            value: (scale * 100.0).round(),
            min: MIN_SCALE * 100.0,
            max: MAX_SCALE * 100.0,
            step: (SCALE_STEP * 100.0).round(),
            stepper: true,
            commit: false,
            blank: false,
        }),
        setter: Some(Setter::Scale),
        code: None,
    };
    let theme = row(
        t("settings.ui.theme"),
        None,
        Control::Segmented {
            options: vec![t("theme.system"), t("theme.light"), t("theme.dark")],
            selected: position(&THEMES, &config.ui.theme.as_str()),
        },
        |c, v| c.ui.theme = THEMES[index(v).min(2)].to_owned(),
    );
    let language = row(
        t("settings.ui.language"),
        None,
        Control::Dropdown { options: vec!["Русский".into(), "English".into()], selected: language_index(&config.ui.language) },
        |c, v| c.ui.language = LANGUAGES[index(v).min(1)].to_owned(),
    );
    let fonts = font_rows(
        &config.ui.font,
        &UI_FAMILIES,
        |c, v| c.ui.font.font_family = UI_FAMILIES[index(v).min(1)].to_owned(),
        |c, v| c.ui.font.font_weight = WEIGHTS[index(v).min(4)],
        |c, v| c.ui.font.font_size_base = font_size(v, 13),
    );
    let mut display = vec![scale_row, theme, language];
    if ctx.workspace.is_some() {
        display.push(Row {
            label: t("settings.ui.homePage"),
            description: Some(t("settings.ui.homePageHint")),
            control: Control::Text { value: ctx.home_page.clone(), commit: false },
            setter: Some(Setter::HomePage),
            code: None,
        });
    }
    display.push(row(t("settings.ui.primaryColor"), None, Control::Color(config.ui.primary_color.clone()), |c, v| {
        if let Some([r, g, b]) = crate::ui::theme::parse_hex(&text(v)) {
            c.ui.primary_color = format!("#{r:02x}{g:02x}{b:02x}");
        }
    }));
    display.push(row(
        t("settings.ui.animatedCovers"),
        Some(t("settings.ui.animatedCoversHint")),
        Control::Switch(config.ui.animations),
        |c, v| c.ui.animations = flag(v),
    ));
    vec![
        Block { title: t("settings.ui.display"), rows: display },
        Block { title: t("settings.font.section"), rows: fonts },
    ]
}

fn editor(config: &AppConfig) -> Vec<Block> {
    let e = &config.editor;
    let mut fonts = font_rows(
        &e.font,
        &EDITOR_FAMILIES,
        |c, v| c.editor.font.font_family = EDITOR_FAMILIES[index(v).min(2)].to_owned(),
        |c, v| c.editor.font.font_weight = WEIGHTS[index(v).min(4)],
        |c, v| c.editor.font.font_size_base = font_size(v, 16),
    );
    fonts.push(row(
        t("settings.editor.lineHeight"),
        None,
        Control::Number(Number::new(f64::from(e.line_height), 1.2, 2.0, 0.05)),
        |c, v| c.editor.line_height = number(v) as f32,
    ));
    let switch = |key: &str, on: bool, set: fn(&mut AppConfig, &Value)| {
        row(t(&format!("settings.editor.{key}")), Some(t(&format!("settings.editor.{key}Hint"))), Control::Switch(on), set)
    };
    vec![
        Block { title: t("settings.font.section"), rows: fonts },
        Block {
            title: t("settings.editor.layout"),
            rows: vec![row(
                t("settings.editor.maxWidth"),
                Some(t("settings.editor.maxWidthHint")),
                Control::Number(Number::new(f64::from(e.max_width_ch), 40.0, 120.0, 1.0)),
                |c, v| c.editor.max_width_ch = number(v) as u32,
            )],
        },
        Block {
            title: t("settings.editor.tabs"),
            rows: vec![row(
                t("settings.editor.liveTabs"),
                Some(t("settings.editor.liveTabsHint")),
                Control::Number(Number::new(f64::from(e.live_tabs), 1.0, 8.0, 1.0)),
                |c, v| c.editor.live_tabs = number(v) as u32,
            )],
        },
        Block {
            title: t("settings.editor.input"),
            rows: vec![
                switch("smartDashes", e.smart_dashes, |c, v| c.editor.smart_dashes = flag(v)),
                switch("listCallouts", e.list_callouts, |c, v| c.editor.list_callouts = flag(v)),
                switch("autoLinkTitle", e.auto_link_title, |c, v| c.editor.auto_link_title = flag(v)),
            ],
        },
        Block {
            title: t("settings.editor.completion"),
            rows: vec![
                switch("linkSuggest", e.link_suggest, |c, v| c.editor.link_suggest = flag(v)),
                row(
                    t("settings.editor.minChars"),
                    Some(t("settings.editor.minCharsHint")),
                    Control::Number(Number::new(f64::from(e.link_suggest_min_chars), 1.0, 5.0, 1.0)),
                    |c, v| c.editor.link_suggest_min_chars = number(v) as u32,
                ),
            ],
        },
        Block {
            title: t("settings.editor.saving"),
            rows: vec![row(
                t("settings.editor.saveDebounce"),
                None,
                Control::Number(Number::new(f64::from(e.save_debounce_ms), 250.0, 10_000.0, 250.0)),
                |c, v| c.editor.save_debounce_ms = number(v) as u32,
            )],
        },
    ]
}

fn reader(config: &AppConfig) -> Vec<Block> {
    let r = &config.reader;
    let mut fonts = font_rows(
        &r.font,
        &READER_FAMILIES,
        |c, v| c.reader.font.font_family = READER_FAMILIES[index(v).min(2)].to_owned(),
        |c, v| c.reader.font.font_weight = WEIGHTS[index(v).min(4)],
        |c, v| c.reader.font.font_size_base = font_size(v, 18),
    );
    fonts.push(row(
        t("settings.reader.lineHeight"),
        None,
        Control::Number(Number::new(f64::from(r.line_height), 1.2, 2.4, 0.05)),
        |c, v| c.reader.line_height = number(v) as f32,
    ));
    let switch = |key: &str, on: bool, set: fn(&mut AppConfig, &Value)| {
        row(t(&format!("settings.reader.{key}")), Some(t(&format!("settings.reader.{key}Hint"))), Control::Switch(on), set)
    };
    vec![
        Block { title: t("settings.font.section"), rows: fonts },
        Block {
            title: t("settings.reader.page"),
            rows: vec![
                row(
                    t("settings.reader.flow"),
                    Some(t("settings.reader.flowHint")),
                    Control::Segmented {
                        options: vec![t("settings.reader.flowPaginated"), t("settings.reader.flowScrolled")],
                        selected: position(&READER_FLOWS, &r.flow.as_str()),
                    },
                    |c, v| c.reader.flow = READER_FLOWS[index(v).min(1)].to_owned(),
                ),
                row(
                    t("settings.reader.maxWidth"),
                    Some(t("settings.reader.maxWidthHint")),
                    Control::Number(Number::new(f64::from(r.max_width_ch), 30.0, 120.0, 1.0)),
                    |c, v| c.reader.max_width_ch = number(v) as u32,
                ),
                row(
                    t("settings.reader.margin"),
                    Some(t("settings.reader.marginHint")),
                    Control::Number(Number::new(f64::from(r.margin_px), 0.0, 160.0, 4.0)),
                    |c, v| c.reader.margin_px = number(v) as u32,
                ),
            ],
        },
        Block {
            title: t("settings.reader.typesetting"),
            rows: vec![
                switch("justify", r.justify, |c, v| c.reader.justify = flag(v)),
                switch("hyphenate", r.hyphenate, |c, v| c.reader.hyphenate = flag(v)),
            ],
        },
    ]
}

pub fn reader_quick(config: &AppConfig) -> Vec<Row> {
    let r = &config.reader;
    vec![
        row(
            t("reader.flow"),
            None,
            Control::Segmented { options: vec![t("reader.flowPaginated"), t("reader.flowScrolled")], selected: position(&READER_FLOWS, &r.flow.as_str()) },
            |c, v| c.reader.flow = READER_FLOWS[index(v).min(1)].to_owned(),
        ),
        row(t("reader.fontSize"), None, Control::Number(Number::font_size(r.font.font_size_base)), |c, v| {
            c.reader.font.font_size_base = font_size(v, 18)
        }),
        row(t("reader.lineHeight"), None, Control::Number(Number::new(f64::from(r.line_height), 1.2, 2.4, 0.05)), |c, v| c.reader.line_height = number(v) as f32),
        row(t("reader.columnWidth"), None, Control::Number(Number::new(f64::from(r.max_width_ch), 30.0, 120.0, 1.0)), |c, v| c.reader.max_width_ch = number(v) as u32),
    ]
}

fn search(config: &AppConfig) -> Vec<Block> {
    let s = &config.search;
    vec![Block {
        title: t("settings.search.section"),
        rows: vec![
            row(
                t("settings.search.candidatePool"),
                None,
                Control::Number(Number::new(s.candidate_pool_size as f64, 32.0, 2048.0, 32.0)),
                |c, v| c.search.candidate_pool_size = number(v) as usize,
            ),
            row(
                t("settings.search.maxQueryTerms"),
                None,
                Control::Number(Number::new(s.max_query_terms as f64, 8.0, 128.0, 1.0)),
                |c, v| c.search.max_query_terms = number(v) as usize,
            ),
        ],
    }]
}

fn files(config: &AppConfig) -> Vec<Block> {
    vec![Block {
        title: t("settings.files.section"),
        rows: vec![row(
            t("settings.files.folder"),
            Some(t("settings.files.folderHint")),
            Control::Text { value: config.files.folder.clone(), commit: false },
            |c, v| c.files.folder = text(v),
        )],
    }]
}

fn analysis(config: &AppConfig) -> Vec<Block> {
    let a = &config.analysis;
    let methods = Block {
        title: t("settings.analysis.methods"),
        rows: vec![
            row(t("settings.analysis.bm25f"), None, Control::Switch(a.enable_bm25f), |c, v| c.analysis.enable_bm25f = flag(v)),
            row(t("settings.analysis.adamicAdar"), None, Control::Switch(a.enable_adamic_adar), |c, v| {
                c.analysis.enable_adamic_adar = flag(v);
            }),
            row(t("settings.analysis.wiki"), None, Control::Switch(a.enable_wikixiv), |c, v| c.analysis.enable_wikixiv = flag(v)),
        ],
    };
    if !a.enable_bm25f {
        return vec![methods];
    }
    let p = &a.bm25f_params;
    let param = |label: String, value: f32, min, max, step, set: fn(&mut AppConfig, &Value)| {
        row(label, None, Control::Number(Number::new(round2(f64::from(value)), min, max, step)), set)
    };
    let params = Block {
        title: t("settings.analysis.bm25fParams"),
        rows: vec![
            param("k1".into(), p.k1, 0.1, 3.0, 0.1, |c, v| c.analysis.bm25f_params.k1 = number(v) as f32),
            param("k3".into(), p.k3, 1.0, 20.0, 0.5, |c, v| c.analysis.bm25f_params.k3 = number(v) as f32),
            param(t("settings.analysis.bTitle"), p.b_title, 0.0, 1.0, 0.05, |c, v| c.analysis.bm25f_params.b_title = number(v) as f32),
            param(t("settings.analysis.bBody"), p.b_body, 0.0, 1.0, 0.05, |c, v| c.analysis.bm25f_params.b_body = number(v) as f32),
            param(t("settings.analysis.titleWeight"), p.title_weight, 0.5, 10.0, 0.1, |c, v| {
                c.analysis.bm25f_params.title_weight = number(v) as f32;
            }),
        ],
    };
    vec![methods, params]
}

fn history(config: &AppConfig) -> Vec<Block> {
    let options = vec![
        t("settings.history.forever"),
        t_with("settings.history.days", &[("count", "30")]),
        t_with("settings.history.days", &[("count", "90")]),
        t("settings.history.year"),
    ];
    vec![Block {
        title: t("settings.history.section"),
        rows: vec![row(
            t("settings.history.retention"),
            Some(t("settings.history.retentionHint")),
            Control::Dropdown { options, selected: position(&HISTORY_DAYS, &config.history.retention_days) },
            |c, v| c.history.retention_days = HISTORY_DAYS[index(v).min(3)],
        )],
    }]
}

fn shortcuts() -> Vec<Block> {
    const GROUPS: &[(&str, &[(&str, &str)])] = &[
        (
            "global",
            &[
                ("newTab", "Ctrl + N"),
                ("newFromTemplate", "Ctrl + U"),
                ("globalSearch", "Ctrl + O"),
                ("focusMode", "Ctrl + Shift + F"),
                ("pageSearch", "Ctrl + F"),
                ("zoomIn", "Ctrl + +"),
                ("zoomOut", "Ctrl + -"),
                ("zoomReset", "Ctrl + 0"),
            ],
        ),
        (
            "editor",
            &[
                ("bold", "Ctrl + B"),
                ("italic", "Ctrl + I"),
                ("strikethrough", "Ctrl + Shift + S"),
                ("indent", "Tab"),
                ("outdent", "Shift + Tab"),
                ("lineBreak", "Shift + ↵"),
            ],
        ),
        (
            "searchWindow",
            &[
                ("searchNext", "↓"),
                ("searchPrevious", "↑"),
                ("searchOpen", "↵"),
                ("searchOpenNewPane", "Ctrl + ↵"),
                ("searchCreate", "Shift + ↵"),
            ],
        ),
        ("pageSearchWindow", &[("searchNext", "↵"), ("searchPrevious", "Shift + ↵")]),
        ("reader", &[("nextPage", "→"), ("previousPage", "←")]),
    ];
    GROUPS
        .iter()
        .map(|(group, items)| Block {
            title: t(&format!("settings.shortcuts.{group}")),
            rows: items
                .iter()
                .map(|(label, keys)| Row {
                    label: t(&format!("settings.shortcuts.{label}")),
                    description: None,
                    control: Control::Kbd((*keys).to_owned()),
                    setter: None,
                    code: None,
                })
                .collect(),
        })
        .collect()
}

pub const DEFAULT_TEMPLATES_FOLDER: &str = "Templates";

pub fn templates_path(workspace: &Path, folder: &str) -> PathBuf {
    let value = folder.trim();
    let drive = value.len() > 2 && value.as_bytes()[0].is_ascii_alphabetic() && value.as_bytes()[1] == b':';
    let absolute = value.starts_with('/') || value.starts_with("\\\\") || drive && value[2..].starts_with(['/', '\\']);
    if absolute {
        return PathBuf::from(value.trim_end_matches(['/', '\\']));
    }
    let normalized = value.trim_matches(['/', '\\']);
    workspace.join(if normalized.is_empty() { DEFAULT_TEMPLATES_FOLDER } else { normalized })
}

fn templates(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let mut rows = vec![row(
        t("settings.templates.folder"),
        Some(t("settings.templates.folderHint")),
        Control::Text { value: config.templates.folder.clone(), commit: false },
        |c, v| c.templates.folder = text(v),
    )];
    if let Some(workspace) = &ctx.workspace
        && !templates_path(workspace, &config.templates.folder).is_dir()
    {
        let folder = config.templates.folder.trim();
        let folder = if folder.is_empty() { DEFAULT_TEMPLATES_FOLDER } else { folder };
        rows.push(info(
            t("settings.templates.create"),
            Some(t_with("settings.templates.createHint", &[("folder", folder)])),
            Control::Buttons(vec![Button::new(t("settings.templates.createButton"), Action::CreateTemplates)]),
        ));
    }
    vec![Block { title: t("settings.templates.section"), rows }]
}

pub fn new_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn snippets(config: &AppConfig, executable: &str) -> Vec<(&'static str, String, String)> {
    let mcp = &config.mcp;
    let url = format!("http://127.0.0.1:{}/mcp", mcp.port);
    let bearer = format!("Bearer {}", mcp.token);
    let gemini = serde_json::json!({
        "mcpServers": { "aquilum": { "httpUrl": url, "headers": { "Authorization": bearer } } }
    });
    vec![
        (
            "Claude Code",
            t("settings.mcp.claudeHint"),
            format!("claude mcp add --transport http aquilum {url} --header \"Authorization: {bearer}\""),
        ),
        (
            "Codex CLI",
            t("settings.mcp.codexHint"),
            format!(
                "[mcp_servers.aquilum]\ncommand = \"{}\"\nargs = [\"--mcp-stdio\"]",
                executable.replace('\\', "\\\\")
            ),
        ),
        ("Gemini CLI", t("settings.mcp.geminiHint"), serde_json::to_string_pretty(&gemini).unwrap_or_default()),
    ]
}

fn mcp(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let m = &config.mcp;
    let status = match &ctx.mcp {
        Some(McpStatus { error: Some(error), .. }) => Control::Status { text: error.clone(), tone: Tone::Danger },
        Some(McpStatus { running: true, port, .. }) => {
            Control::Status { text: t_with("settings.mcp.running", &[("port", &port.to_string())]), tone: Tone::Secondary }
        }
        _ => Control::Status { text: t("settings.mcp.off"), tone: Tone::Muted },
    };
    let mut port = Number::new(f64::from(m.port), 1024.0, 65535.0, 1.0);
    port.commit = true;
    let mut blocks = vec![Block {
        title: t("settings.mcp.section"),
        rows: vec![
            row(t("settings.mcp.access"), Some(t("settings.mcp.accessHint")), Control::Switch(m.enabled), |c, v| {
                c.mcp.enabled = flag(v);
                if c.mcp.token.trim().is_empty() {
                    c.mcp.token = new_token();
                }
            }),
            info(t("settings.mcp.status"), None, status),
            row(t("settings.mcp.port"), None, Control::Number(port), |c, v| c.mcp.port = number(v) as u16),
            row(
                t("settings.mcp.token"),
                Some(t("settings.mcp.tokenHint")),
                Control::Text { value: m.token.clone(), commit: true },
                |c, v| c.mcp.token = text(v),
            ),
            info(
                t("settings.mcp.newToken"),
                Some(t("settings.mcp.newTokenHint")),
                Control::Buttons(vec![Button::new(t("settings.mcp.generate"), Action::NewToken)]),
            ),
            row(t("settings.mcp.allowWrite"), Some(t("settings.mcp.allowWriteHint")), Control::Switch(m.allow_write), |c, v| {
                c.mcp.allow_write = flag(v);
            }),
        ],
    }];
    let executable = ctx.mcp.as_ref().map_or(String::new(), |s| s.executable.clone());
    let rows = snippets(config, &executable)
        .into_iter()
        .enumerate()
        .map(|(i, (title, hint, code))| {
            let label = if ctx.copied == Some(i) { t("settings.mcp.copied") } else { t("settings.mcp.copy") };
            let mut row = info(title.to_owned(), Some(hint), Control::Buttons(vec![Button::new(label, Action::Copy(i, code.clone()))]));
            row.code = Some(code);
            row
        })
        .collect();
    blocks.push(Block { title: t("settings.mcp.connection"), rows });
    blocks
}

const TRASH_DAYS: [u32; 5] = [0, 7, 30, 90, 365];
pub const TRASH_PAGE_SIZE: usize = 20;

fn format_locale(value: f64, digits: usize) -> String {
    let text = format!("{value:.digits$}");
    let text = if digits > 0 { text.trim_end_matches('0').trim_end_matches('.').to_owned() } else { text };
    let (whole, fraction) = text.split_once('.').map_or((text.as_str(), None), |(w, f)| (w, Some(f)));
    let russian = i18n::is_russian();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(if russian { '\u{a0}' } else { ',' });
        }
        grouped.push(ch);
    }
    match fraction {
        Some(f) => format!("{grouped}{}{f}", if russian { ',' } else { '.' }),
        None => grouped,
    }
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        return t_with("settings.trash.bytes", &[("value", &format_locale(bytes as f64, 0))]);
    }
    if bytes < 1024 * 1024 {
        return t_with("settings.trash.kilobytes", &[("value", &format_locale((bytes as f64 / 1024.0).round(), 0))]);
    }
    t_with("settings.trash.megabytes", &[("value", &format_locale(bytes as f64 / (1024.0 * 1024.0), 1))])
}


fn deletion_row(deletion: TrashedDeletion) -> Row {
    let original = Path::new(&deletion.original);
    let folder = deletion.files > 1;
    let name = if folder { original.file_name() } else { original.file_stem() };
    let parent = deletion.original.rfind(['/', '\\']).map(|cut| &deletion.original[..cut]).filter(|p| !p.is_empty());
    let parent = parent.map_or_else(|| t("settings.trash.root"), str::to_owned);
    let date = crate::dates::date_time(deletion.deleted_at_ms);
    let description = if folder {
        let files = plural("settings.trash.files", deletion.files as u64);
        t_with("settings.trash.deletedFolderAt", &[("folder", &parent), ("date", &date), ("files", &files)])
    } else {
        t_with("settings.trash.deletedAt", &[("folder", &parent), ("date", &date)])
    };
    info(
        name.map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        Some(description),
        Control::Buttons(vec![Button::new(t("settings.trash.restore"), Action::Restore(deletion.id))]),
    )
}

fn trash(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let options = vec![
        t("settings.trash.never"),
        t_with("settings.trash.days", &[("count", "7")]),
        t_with("settings.trash.days", &[("count", "30")]),
        t_with("settings.trash.days", &[("count", "90")]),
        t("settings.trash.year"),
    ];
    let state = ctx.workspace.as_deref().and_then(|w| trash_state_impl(w).ok());
    let description = match state {
        Some(state) => format!("{}, {}", plural("settings.trash.files", state.count), format_size(state.bytes)),
        None => t("settings.trash.emptyHint"),
    };
    let mut blocks = vec![Block {
        title: t("settings.trash.section"),
        rows: vec![
            row(
                t("settings.trash.retention"),
                Some(t("settings.trash.retentionHint")),
                Control::Dropdown { options, selected: position(&TRASH_DAYS, &config.trash.retention_days) },
                |c, v| c.trash.retention_days = TRASH_DAYS[index(v).min(4)],
            ),
            info(
                t("settings.trash.deleted"),
                Some(description),
                Control::Buttons(vec![Button::new(t("settings.trash.open"), Action::OpenTrash).enabled(ctx.workspace.is_some())]),
            ),
        ],
    }];
    let Some(workspace) = &ctx.workspace else { return blocks };
    let page = ctx.trash_page;
    let Ok(listing) = deletion_page_impl(workspace, page * TRASH_PAGE_SIZE, TRASH_PAGE_SIZE) else { return blocks };
    if listing.total == 0 {
        return blocks;
    }
    let first = page * TRASH_PAGE_SIZE;
    let shown = listing.deletions.len();
    let pages = listing.total.div_ceil(TRASH_PAGE_SIZE);
    let mut rows: Vec<Row> = listing.deletions.into_iter().map(deletion_row).collect();
    if pages > 1 {
        let range = t_with(
            "settings.trash.pageRange",
            &[("from", &(first + 1).to_string()), ("to", &(first + shown).to_string()), ("total", &listing.total.to_string())],
        );
        rows.push(info(
            range,
            None,
            Control::Buttons(vec![
                Button::new(t("settings.trash.previous"), Action::TrashPage(page.saturating_sub(1))).enabled(page > 0),
                Button::new(t("settings.trash.next"), Action::TrashPage(page + 1)).enabled(page + 1 < pages),
            ]),
        ));
    }
    blocks.push(Block { title: t("settings.trash.filesSection"), rows });
    blocks
}

pub const AUTHOR_URL: &str = "https://t.me/dmitriy_yiu";

fn update_row(status: &crate::updater::Status) -> Row {
    use crate::updater::Status;
    let description = match status {
        Status::Idle => t("settings.system.updatesIdle"),
        Status::Checking => t("settings.system.updatesChecking"),
        Status::Latest => t("settings.system.updatesLatest"),
        Status::Available(version) => t_with("settings.system.updatesAvailable", &[("version", version)]),
        Status::Downloading { version, .. } | Status::Ready(version) => t_with("settings.system.updatesInstalling", &[("version", version)]),
        Status::Installing => t("update.installing"),
        Status::Failed(message) => t_with("settings.system.updatesFailed", &[("message", message)]),
    };
    let button = match status {
        Status::Available(_) => Button::new(t("settings.system.installUpdate"), Action::InstallUpdate),
        Status::Checking | Status::Downloading { .. } | Status::Ready(_) | Status::Installing => Button::new(t("settings.system.checkUpdates"), Action::CheckUpdates).enabled(false),
        _ => Button::new(t("settings.system.checkUpdates"), Action::CheckUpdates),
    };
    info(t("settings.system.manualUpdate"), Some(description), Control::Buttons(vec![button]))
}

fn system(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let mut updates = vec![row(
        t("settings.system.autoUpdate"),
        Some(t("settings.system.autoUpdateHint")),
        Control::Switch(config.updates.auto),
        |c, v| c.updates.auto = flag(v),
    )];
    if !config.updates.auto {
        updates.push(match &ctx.update {
            Some(status) => update_row(status),
            None => info(
                t("settings.system.manualUpdate"),
                Some(t("settings.system.updatesUnavailable")),
                Control::Buttons(vec![Button::new(t("settings.system.checkUpdates"), Action::CheckUpdates).enabled(false)]),
            ),
        });
    }
    vec![
        Block { title: t("settings.system.updates"), rows: updates },
        Block {
            title: t("settings.system.autostart"),
            rows: vec![match ctx.autostart {
                Some(on) => Row { label: t("settings.system.launch"), description: Some(t("settings.system.launchHint")), control: Control::Switch(on), setter: Some(Setter::Autostart), code: None },
                None => info(t("settings.system.launch"), Some(t("settings.system.launchUnavailable")), Control::Locked(false)),
            }],
        },
        Block {
            title: t("settings.system.about"),
            rows: vec![
                info(
                    t("settings.system.author"),
                    Some(t("settings.system.authorHint")),
                    Control::Buttons(vec![Button::new(AUTHOR_URL.to_owned(), Action::OpenUrl(AUTHOR_URL))]),
                ),
                info(
                    t("settings.system.version"),
                    Some(t("settings.system.versionHint")),
                    Control::Status { text: env!("CARGO_PKG_VERSION").to_owned(), tone: Tone::Muted },
                ),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_parse_clamp_step_and_format() {
        let n = Number::new(1.6, 1.2, 2.0, 0.05);
        assert_eq!(n.parse("1,75"), Some(1.75));
        assert_eq!(n.parse("9"), Some(2.0));
        assert_eq!(n.parse(" "), None);
        assert_eq!(n.parse("abc"), None);
        assert_eq!(n.step_from(1.6, 1.0), 1.65);
        assert_eq!(n.step_from(2.0, 1.0), 2.0);
        assert_eq!(format_number(1.65), "1.65");
        assert_eq!(format_number(100.0), "100");
        assert_eq!(format_number(1.5), "1.5");
    }

    #[test]
    fn setters_write_config() {
        let mut config = AppConfig::default();
        let ctx = Context::new(1.0);
        let theme = ui(&config, &ctx).remove(0).rows.remove(1);
        let Some(Setter::Config(set)) = theme.setter else { panic!("тема пишется в настройки") };
        set(&mut config, &Value::Index(2));
        assert_eq!(config.ui.theme, "dark");
        let animations = ui(&config, &ctx).remove(0).rows.pop().unwrap();
        let Some(Setter::Config(set)) = animations.setter else { panic!("анимация пишется в настройки") };
        set(&mut config, &Value::Bool(false));
        assert!(!config.ui.animations);
        assert!(blocks(Section::Analysis, &config, &ctx).len() == 2);
        config.analysis.enable_bm25f = false;
        assert!(blocks(Section::Analysis, &config, &ctx).len() == 1);
    }

    #[test]
    fn templates_path_and_trash_size() {
        let workspace = Path::new("/kb");
        assert_eq!(templates_path(workspace, ""), workspace.join("Templates"));
        assert_eq!(templates_path(workspace, "Шаблоны/"), workspace.join("Шаблоны"));
        assert_eq!(templates_path(workspace, "/Шаблоны/"), PathBuf::from("/Шаблоны"));
        assert_eq!(templates_path(workspace, "D:\\T\\"), PathBuf::from("D:\\T"));
        i18n::set_language("ru");
        assert_eq!(format_size(512), "512 Б");
        assert_eq!(format_size(1_572_864), "1,5 МБ");
    }
}
