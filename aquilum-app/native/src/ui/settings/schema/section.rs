use crate::i18n::t;
use crate::ui::icons::{self, Icon};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Ui,
    Editor,
    Reader,
    Data,
    Analysis,
    Mcp,
    History,
    Trash,
    Sync,
    System,
    Shortcuts,
}

impl Section {
    pub const ALL: [Section; 11] = [
        Section::Ui,
        Section::Editor,
        Section::Reader,
        Section::Data,
        Section::Analysis,
        Section::Mcp,
        Section::History,
        Section::Trash,
        Section::Sync,
        Section::System,
        Section::Shortcuts,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Section::Ui => "ui",
            Section::Editor => "editor",
            Section::Reader => "reader",
            Section::Data => "data",
            Section::Analysis => "analysis",
            Section::Mcp => "mcp",
            Section::History => "history",
            Section::Trash => "trash",
            Section::Sync => "sync",
            Section::System => "system",
            Section::Shortcuts => "shortcuts",
        }
    }

    pub fn from_key(key: &str) -> Option<Section> {
        let key = match key {
            "templates" | "files" => "data",
            "search" => "analysis",
            key => key,
        };
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
            Section::Data => icons::FOLDER,
            Section::Analysis => icons::GRAPH_ANALYSIS,
            Section::Mcp => icons::PLUG,
            Section::History => icons::HISTORY,
            Section::Trash => icons::TRASH,
            Section::Sync => icons::CLOUD,
            Section::System => icons::POWER,
            Section::Shortcuts => icons::KEYBOARD,
        }
    }
}
