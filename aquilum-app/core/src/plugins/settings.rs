use crate::settings::models::ShortcutToken;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WeeklySettings {
    pub enabled: bool,
    pub folder: String,
    pub format: String,
    pub template: String,
}

impl Default for WeeklySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            folder: String::new(),
            format: "gggg-[W]ww".to_owned(),
            template: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CalendarSettings {
    pub enabled: bool,
    pub show_week_numbers: bool,
    pub open_today_shortcut: Option<ShortcutToken>,
    pub weekly: WeeklySettings,
}

impl Default for CalendarSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_week_numbers: false,
            open_today_shortcut: None,
            weekly: WeeklySettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FolderCountsSettings {
    pub enabled: bool,
    pub show_all_files: bool,
    pub hide_zero: bool,
}

impl Default for FolderCountsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            show_all_files: false,
            hide_zero: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileColorsSettings {
    pub enabled: bool,
    pub cascade: bool,
    pub background: bool,
}

impl Default for FileColorsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            cascade: false,
            background: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EnabledSettings {
    pub enabled: bool,
}

impl Default for EnabledSettings {
    fn default() -> Self {
        Self {
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReadingModeSettings {
    pub enabled: bool,
    pub shortcut: Option<ShortcutToken>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ColoredTagsSettings {
    pub enabled: bool,
    pub mix_nested: bool,
    pub tag_colors: BTreeMap<String, String>,
}

impl Default for ColoredTagsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mix_nested: true,
            tag_colors: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CodeStylerSettings {
    pub enabled: bool,
    pub line_numbers: bool,
    pub copy_button: bool,
    pub header: bool,
}

impl Default for CodeStylerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            line_numbers: false,
            copy_button: true,
            header: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AdvancedTablesSettings {
    pub enabled: bool,
    pub format_on_leave: bool,
}

impl Default for AdvancedTablesSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            format_on_leave: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GitSyncSettings {
    pub enabled: bool,
    pub commit_message: String,
    pub commit_date_format: String,
    pub auto_backup_minutes: u32,
    pub pull_on_open: bool,
    pub push: bool,
    pub sync_method: String,
}

impl Default for GitSyncSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            commit_message: "vault backup: {{date}}".to_owned(),
            commit_date_format: "YYYY-MM-DD HH:mm:ss".to_owned(),
            auto_backup_minutes: 0,
            pull_on_open: false,
            push: true,
            sync_method: "merge".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginSettings {
    pub calendar: CalendarSettings,
    pub folder_counts: FolderCountsSettings,
    pub file_colors: FileColorsSettings,
    pub file_icons: EnabledSettings,
    pub explorer_filters: EnabledSettings,
    pub colored_tags: ColoredTagsSettings,
    pub cursor_trail: EnabledSettings,
    pub code_styler: CodeStylerSettings,
    pub reading_mode: ReadingModeSettings,
    pub advanced_tables: AdvancedTablesSettings,
    pub git_sync: GitSyncSettings,
}

impl Default for PluginSettings {
    fn default() -> Self {
        Self {
            calendar: CalendarSettings::default(),
            folder_counts: FolderCountsSettings::default(),
            file_colors: FileColorsSettings::default(),
            file_icons: EnabledSettings::default(),
            explorer_filters: EnabledSettings::default(),
            colored_tags: ColoredTagsSettings::default(),
            cursor_trail: EnabledSettings::default(),
            code_styler: CodeStylerSettings::default(),
            reading_mode: ReadingModeSettings::default(),
            advanced_tables: AdvancedTablesSettings::default(),
            git_sync: GitSyncSettings::default(),
        }
    }
}
