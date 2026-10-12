use crate::i18n::t;
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
