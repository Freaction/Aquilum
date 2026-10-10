use crate::files::document::write_file_atomic_impl;
use crate::settings::models::AppConfig;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub struct SettingsManager {
    config_path: PathBuf,
    current_config: RwLock<AppConfig>,
}

impl SettingsManager {
    pub fn new(app_data_dir: &Path) -> Self {
        let config_path = app_data_dir.join("settings.json");
        let current_config = Self::load_or_default(&config_path);
        Self {
            config_path,
            current_config: RwLock::new(current_config),
        }
    }

    fn load_or_default(path: &Path) -> AppConfig {
        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => return config,
                Err(error) => set_aside_unreadable(path, &error),
            },
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => eprintln!("[aquilum:settings] не удалось прочитать {}: {error}", path.display()),
        }
        let default_config = AppConfig::default();
        if let Err(error) = write_config(path, &default_config) {
            eprintln!("[aquilum:settings] не удалось записать настройки по умолчанию: {error}");
        }
        default_config
    }

    pub fn get_config(&self) -> AppConfig {
        self.current_config.read().unwrap().clone()
    }

    pub fn update_config(&self, new_config: AppConfig) -> Result<(), String> {
        write_config(&self.config_path, &new_config)?;
        *self.current_config.write().unwrap() = new_config;
        Ok(())
    }
}

fn write_config(path: &Path, config: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    write_file_atomic_impl(path, &json, None)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn set_aside_unreadable(path: &Path, error: &serde_json::Error) {
    let backup = path.with_extension("json.unreadable");
    eprintln!(
        "[aquilum:settings] {} не разобран ({error}), копия сохранена в {}",
        path.display(),
        backup.display()
    );
    if let Err(copy_error) = fs::copy(path, &backup) {
        eprintln!("[aquilum:settings] не удалось сохранить копию: {copy_error}");
    }
}

#[cfg(test)]
mod tests {
    use super::SettingsManager;

    fn plugin_defaults() -> serde_json::Value {
        serde_json::json!({
            "calendar": {"enabled": true, "showWeekNumbers": false, "openTodayShortcut": null,
                "weekly": {"enabled": false, "folder": "", "format": "gggg-[W]ww", "template": ""}},
            "folderCounts": {"enabled": false, "showAllFiles": false, "hideZero": true},
            "fileColors": {"enabled": false, "cascade": false, "background": false},
            "fileIcons": {"enabled": false},
            "explorerFilters": {"enabled": false},
            "coloredTags": {"enabled": false, "mixNested": true, "tagColors": {}},
            "cursorTrail": {"enabled": false},
            "codeStyler": {"enabled": false, "lineNumbers": false, "copyButton": true, "header": true},
            "readingMode": {"enabled": false, "shortcut": null},
            "advancedTables": {"enabled": false, "formatOnLeave": false},
            "gitSync": {"enabled": false, "commitMessage": "vault backup: {{date}}",
                "commitDateFormat": "YYYY-MM-DD HH:mm:ss", "autoBackupMinutes": 0,
                "pullOnOpen": false, "push": true, "syncMethod": "merge"}
        })
    }

    #[test]
    fn plugin_defaults_are_complete_in_old_and_default_configs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), "{}").unwrap();
        let loaded = SettingsManager::new(dir.path()).get_config();
        assert_eq!(serde_json::to_value(loaded).unwrap()["plugins"], plugin_defaults());
        assert_eq!(serde_json::to_value(crate::settings::models::AppConfig::default()).unwrap()["plugins"], plugin_defaults());
    }

    #[test]
    fn legacy_shortcut_settings_keep_values_and_default_to_null() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{"editor":{"fullWidth":true},"plugins":{"calendar":{"showWeekNumbers":true},"readingMode":{"enabled":true}}}"#,
        ).unwrap();
        let config = SettingsManager::new(dir.path()).get_config();
        assert!(config.editor.full_width);
        assert!(config.plugins.calendar.show_week_numbers);
        assert!(config.plugins.reading_mode.enabled);
        for config in [config, crate::settings::models::AppConfig::default()] {
            let json = serde_json::to_value(config).unwrap();
            assert_eq!(json["editor"]["fullWidthShortcut"], serde_json::Value::Null);
            assert_eq!(json["plugins"]["calendar"]["openTodayShortcut"], serde_json::Value::Null);
            assert_eq!(json["plugins"]["readingMode"]["shortcut"], serde_json::Value::Null);
        }
    }

    #[test]
    fn shortcut_tokens_and_reset_survive_save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let manager = SettingsManager::new(dir.path());
        let mut json = serde_json::to_value(manager.get_config()).unwrap();
        let shortcuts = [
            serde_json::json!({"code": "KeyD", "key": "d", "primary": true, "alt": false, "shift": true}),
            serde_json::json!({"code": "KeyE", "key": "e", "primary": false, "alt": true, "shift": false}),
            serde_json::json!({"code": "KeyF", "key": "а", "primary": true, "alt": true, "shift": false}),
        ];
        json["plugins"]["calendar"]["openTodayShortcut"] = shortcuts[0].clone();
        json["plugins"]["readingMode"]["shortcut"] = shortcuts[1].clone();
        json["editor"]["fullWidthShortcut"] = shortcuts[2].clone();
        manager.update_config(serde_json::from_value(json.clone()).unwrap()).unwrap();
        let saved: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("settings.json")).unwrap(),
        ).unwrap();
        assert_eq!(saved["plugins"], json["plugins"]);
        assert_eq!(saved["editor"]["fullWidthShortcut"], json["editor"]["fullWidthShortcut"]);
        let reloaded = SettingsManager::new(dir.path());
        assert_eq!(serde_json::to_value(reloaded.get_config()).unwrap(), json);

        json["plugins"]["calendar"]["openTodayShortcut"] = serde_json::Value::Null;
        json["plugins"]["readingMode"]["shortcut"] = serde_json::Value::Null;
        json["editor"]["fullWidthShortcut"] = serde_json::Value::Null;
        reloaded.update_config(serde_json::from_value(json.clone()).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(SettingsManager::new(dir.path()).get_config()).unwrap(), json);
    }

    #[test]
    fn partial_plugin_settings_keep_defaults_and_survive_reload() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), r#"{"plugins":{"gitSync":{"enabled":true}}}"#).unwrap();
        let manager = SettingsManager::new(dir.path());
        let config = manager.get_config();
        let mut expected = plugin_defaults();
        expected["gitSync"]["enabled"] = serde_json::json!(true);
        assert_eq!(serde_json::to_value(&config).unwrap()["plugins"], expected);
        let mut json = serde_json::to_value(config).unwrap();
        json["plugins"]["calendar"]["weekly"] = serde_json::json!({"enabled": true});
        json["plugins"]["coloredTags"]["tagColors"] = serde_json::json!({"project": "blue"});
        manager.update_config(serde_json::from_value(json).unwrap()).unwrap();
        expected["calendar"]["weekly"]["enabled"] = serde_json::json!(true);
        expected["coloredTags"]["tagColors"] = serde_json::json!({"project": "blue"});
        let reloaded = SettingsManager::new(dir.path()).get_config();
        assert_eq!(serde_json::to_value(reloaded).unwrap()["plugins"], expected);
    }

    #[test]
    fn partial_settings_keep_present_values_and_fill_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{"mcp":{"token":"secret"},"editor":{"fontFamily":"iA Writer Quattro"}}"#,
        )
        .unwrap();

        let config = SettingsManager::new(dir.path()).get_config();

        assert_eq!(config.mcp.token, "secret");
        assert_eq!(config.mcp.port, 8787);
        assert_eq!(config.editor.font.font_family, "iA Writer Quattro");
        assert_eq!(config.editor.save_debounce_ms, 1000);
        assert!(config.editor.full_width_shortcut.is_none());
        assert!(config.analysis.enable_bm25f);
    }

    #[test]
    fn legacy_editor_width_survives_full_width_save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{"editor":{"maxWidthCh":80}}"#,
        ).unwrap();
        let manager = SettingsManager::new(dir.path());
        let mut config = manager.get_config();
        let initial_json = serde_json::to_value(&config).unwrap();
        assert_eq!(initial_json["editor"]["fullWidth"], false);
        assert_eq!(config.editor.max_width_ch, 80);
        let mut json = initial_json;
        json["editor"]["fullWidth"] = serde_json::json!(true);
        config = serde_json::from_value(json).unwrap();
        manager.update_config(config).unwrap();
        let reloaded = SettingsManager::new(dir.path()).get_config();
        let json = serde_json::to_value(&reloaded).unwrap();
        assert_eq!(json["editor"]["fullWidth"], true);
        assert_eq!(reloaded.editor.max_width_ch, 80);
    }

    #[test]
    fn daily_notes_defaults_and_custom_settings_survive_reload() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), "{}").unwrap();
        let manager = SettingsManager::new(dir.path());
        let mut json = serde_json::to_value(manager.get_config()).unwrap();
        assert_eq!(json["dailyNotes"]["format"], "YYYY-MM-DD");
        assert_eq!(json["dailyNotes"]["openOnStartup"], false);
        assert_eq!(json["dailyNotes"]["confirmBeforeCreate"], true);
        assert_eq!(json["dailyNotes"]["wordsPerDot"], 250);
        assert_eq!(json["dailyNotes"]["weekStart"], "locale");
        json["dailyNotes"] = serde_json::json!({
            "folder": "Daily", "format": "YYYY/MM/DD", "template": "Templates/Daily.md",
            "openOnStartup": true, "wordsPerDot": 100, "weekStart": "monday",
            "confirmBeforeCreate": false
        });
        let expected = json["dailyNotes"].clone();
        manager.update_config(serde_json::from_value(json).unwrap()).unwrap();
        let reloaded = SettingsManager::new(dir.path()).get_config();
        assert_eq!(serde_json::to_value(reloaded).unwrap()["dailyNotes"], expected);
    }

    #[test]
    fn unreadable_settings_are_set_aside_before_defaults_are_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{ broken").unwrap();

        SettingsManager::new(dir.path());

        let backup = std::fs::read_to_string(dir.path().join("settings.json.unreadable")).unwrap();
        assert_eq!(backup, "{ broken");
        assert!(std::fs::read_to_string(&path).unwrap().contains("\"mcp\""));
    }
}
