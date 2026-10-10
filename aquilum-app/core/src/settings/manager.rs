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
        assert!(config.analysis.enable_bm25f);
        assert!(config.ui.animations);
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
