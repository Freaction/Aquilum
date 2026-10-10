use super::error::PluginError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VaultPluginData {
    pub version: u32,
    pub colors: BTreeMap<String, String>,
    pub icons: BTreeMap<String, String>,
    pub recent_icons: Vec<String>,
    pub filters: Filters,
}

impl Default for VaultPluginData {
    fn default() -> Self {
        Self { version: 1, colors: BTreeMap::new(), icons: BTreeMap::new(), recent_icons: Vec::new(), filters: Filters::default() }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Filters {
    pub hide: Vec<Filter>,
    pub pin: Vec<Filter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub name: String,
    pub active: bool,
    pub kind: FilterKind,
    pub target: FilterTarget,
    pub pattern: String,
    pub pattern_type: PatternType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterKind { Path, Tag }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterTarget { Files, Folders, Both }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternType { Strict, Wildcard, Regex }

// Сериализуем чтение с переносом битого файла и изменения, чтобы хуки не теряли записи.
static DATA_IO: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_data() -> std::sync::MutexGuard<'static, ()> {
    DATA_IO.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn load(root: &Path) -> Result<VaultPluginData, PluginError> {
    let _guard = lock_data();
    load_unlocked(root)
}

fn load_unlocked(root: &Path) -> Result<VaultPluginData, PluginError> {
    let path = root.join(crate::history::AQUILUM_FOLDER).join("plugins.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(VaultPluginData::default()),
        Err(error) => return Err(error.into()),
    };
    let parsed = serde_json::from_slice::<VaultPluginData>(&bytes)
        .map_err(|error| PluginError::Invalid { message: error.to_string() })
        .and_then(|data| { validate(&data)?; Ok(data) });
    match parsed {
        Ok(data) => Ok(data),
        Err(error) => {
            let mut backup = path.with_extension("json.unreadable");
            let mut suffix = 0;
            while backup.exists() {
                suffix += 1;
                backup = path.with_extension(format!("json.unreadable.{suffix}"));
            }
            std::fs::rename(&path, &backup)?;
            eprintln!("[aquilum:plugins] {} не разобран ({error}), сохранён в {}", path.display(), backup.display());
            Ok(VaultPluginData::default())
        }
    }
}

pub fn save(root: &Path, data: &VaultPluginData) -> Result<(), PluginError> {
    let _guard = lock_data();
    save_unlocked(root, data)
}

fn save_unlocked(root: &Path, data: &VaultPluginData) -> Result<(), PluginError> {
    use std::io::Write;
    validate(data)?;
    let bytes = serde_json::to_vec_pretty(data)
        .map_err(|error| PluginError::Invalid { message: error.to_string() })?;
    let folder = root.join(crate::history::AQUILUM_FOLDER);
    std::fs::create_dir_all(&folder)?;
    let mut file = atomic_write_file::AtomicWriteFile::open(folder.join("plugins.json"))?;
    file.write_all(&bytes)?;
    file.commit()?;
    Ok(())
}

pub fn relocate(root: &Path, from_rel: &str, to_rel: &str) -> Result<(), PluginError> {
    validate_path(from_rel)?;
    validate_path(to_rel)?;
    let _guard = lock_data();
    let mut data = load_unlocked(root)?;
    let before = data.clone();
    for map in [&mut data.colors, &mut data.icons] {
        let moved: Vec<_> = map.keys().filter(|key| within(key, from_rel)).cloned().collect();
        let values: Vec<_> = moved.into_iter().map(|key| {
            let value = map.remove(&key).unwrap();
            (format!("{to_rel}{}", &key[from_rel.len()..]), value)
        }).collect();
        map.extend(values);
    }
    for filter in data.filters.hide.iter_mut().chain(&mut data.filters.pin) {
        if strict_path(filter) && within(&filter.pattern, from_rel) {
            filter.pattern = format!("{to_rel}{}", &filter.pattern[from_rel.len()..]);
        }
    }
    if data != before { save_unlocked(root, &data)?; }
    Ok(())
}

pub fn remove(root: &Path, rel: &str) -> Result<(), PluginError> {
    validate_path(rel)?;
    let _guard = lock_data();
    let mut data = load_unlocked(root)?;
    let before = data.clone();
    data.colors.retain(|key, _| !within(key, rel));
    data.icons.retain(|key, _| !within(key, rel));
    for filters in [&mut data.filters.hide, &mut data.filters.pin] {
        filters.retain(|filter| !strict_path(filter) || !within(&filter.pattern, rel));
    }
    if data != before { save_unlocked(root, &data)?; }
    Ok(())
}

fn within(path: &str, prefix: &str) -> bool {
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

fn strict_path(filter: &Filter) -> bool {
    filter.kind == FilterKind::Path && filter.pattern_type == PatternType::Strict
}

fn validate_path(path: &str) -> Result<(), PluginError> {
    if path.contains(['\\', ':']) || path.chars().any(char::is_control)
        || path.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(PluginError::Invalid { message: format!("нужен относительный путь через /: {path}") });
    }
    Ok(())
}

fn validate(data: &VaultPluginData) -> Result<(), PluginError> {
    for key in data.colors.keys().chain(data.icons.keys()) { validate_path(key)?; }
    for filter in data.filters.hide.iter().chain(&data.filters.pin).filter(|filter| strict_path(filter)) {
        validate_path(&filter.pattern)?;
    }
    for token in data.colors.values() {
        if !matches!(token.as_str(), "red" | "amber" | "green" | "teal" | "blue" | "purple" | "pink" | "gray") {
            return Err(PluginError::Invalid { message: format!("неизвестный цвет: {token}") });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use serde_json::json;

    fn sample() -> VaultPluginData {
        serde_json::from_value(json!({
            "version": 1,
            "colors": {"A": "blue", "A/B/n.md": "red", "AB": "green"},
            "icons": {"A/B": "folder", "AB": "book"},
            "recentIcons": ["folder"],
            "filters": {
                "hide": [
                    {"name":"path", "active":true, "kind":"path", "target":"both", "pattern":"A/B", "patternType":"strict"},
                    {"name":"tag", "active":false, "kind":"tag", "target":"files", "pattern":"A/B", "patternType":"strict"}
                ],
                "pin": [
                    {"name":"wildcard", "active":true, "kind":"path", "target":"files", "pattern":"A/*", "patternType":"wildcard"},
                    {"name":"regex", "active":true, "kind":"path", "target":"folders", "pattern":"A/B", "patternType":"regex"},
                    {"name":"strict", "active":false, "kind":"path", "target":"files", "pattern":"A/B/n.md", "patternType":"strict"}
                ]
            }
        })).unwrap()
    }

    #[test]
    fn a_missing_file_returns_defaults_without_creating_it() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(serde_json::to_value(load(dir.path()).unwrap()).unwrap(),
            json!({"version":1, "colors":{}, "icons":{}, "recentIcons":[], "filters":{"hide":[], "pin":[]}}));
        assert!(!dir.path().join(".aquilum").exists());
    }

    #[test]
    fn unreadable_data_is_moved_aside_before_defaults_are_returned() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join(".aquilum");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("plugins.json"), "{broken").unwrap();
        assert_eq!(load(dir.path()).unwrap(), VaultPluginData::default());
        assert_eq!(fs::read_to_string(folder.join("plugins.json.unreadable")).unwrap(), "{broken");
        assert!(!folder.join("plugins.json").exists());
    }

    #[test]
    fn saved_data_round_trips_with_relative_slash_keys() {
        let dir = tempfile::tempdir().unwrap();
        let data = sample();
        save(dir.path(), &data).unwrap();
        assert_eq!(load(dir.path()).unwrap(), data);
        let json: serde_json::Value = serde_json::from_slice(&fs::read(dir.path().join(".aquilum/plugins.json")).unwrap()).unwrap();
        assert_eq!(json["colors"]["A/B/n.md"], "red");
        assert_eq!(json["recentIcons"], json!(["folder"]));
        assert_eq!(fs::read_dir(dir.path().join(".aquilum")).unwrap().count(), 1);
    }

    #[test]
    fn relocating_a_folder_moves_descendants_and_only_strict_path_filters() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &sample()).unwrap();
        relocate(dir.path(), "A", "Archive/A").unwrap();
        let data = load(dir.path()).unwrap();
        assert_eq!(serde_json::to_value(&data.colors).unwrap(), json!({"Archive/A":"blue", "Archive/A/B/n.md":"red", "AB":"green"}));
        assert_eq!(serde_json::to_value(&data.icons).unwrap(), json!({"Archive/A/B":"folder", "AB":"book"}));
        assert_eq!(data.filters.hide[0].pattern, "Archive/A/B");
        assert_eq!(data.filters.hide[1].pattern, "A/B");
        assert_eq!(data.filters.pin[0].pattern, "A/*");
        assert_eq!(data.filters.pin[1].pattern, "A/B");
        assert_eq!(data.filters.pin[2].pattern, "Archive/A/B/n.md");
        assert_eq!(data.recent_icons, ["folder"]);
    }

    #[test]
    fn removing_a_folder_removes_descendants_and_strict_path_filters() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &sample()).unwrap();
        remove(dir.path(), "A").unwrap();
        let data = load(dir.path()).unwrap();
        assert_eq!(serde_json::to_value(data.colors).unwrap(), json!({"AB":"green"}));
        assert_eq!(serde_json::to_value(data.icons).unwrap(), json!({"AB":"book"}));
        assert_eq!(data.filters.hide.len(), 1);
        assert_eq!(data.filters.hide[0].kind, FilterKind::Tag);
        assert_eq!(data.filters.pin.len(), 2);
        assert_eq!(data.recent_icons, ["folder"]);
    }

    #[test]
    fn invalid_paths_and_colors_do_not_overwrite_saved_data() {
        let dir = tempfile::tempdir().unwrap();
        let original = sample();
        save(dir.path(), &original).unwrap();
        for key in ["/A", "../A", "A/../B", "A\\B", "C:/A", "A//B", "A/"] {
            let mut data = original.clone();
            data.colors.insert(key.to_owned(), "blue".to_owned());
            assert!(matches!(save(dir.path(), &data), Err(PluginError::Invalid { .. })), "{key}");
        }
        let mut data = original.clone();
        data.colors.insert("A".to_owned(), "#ffffff".to_owned());
        assert!(matches!(save(dir.path(), &data), Err(PluginError::Invalid { .. })));
        assert!(relocate(dir.path(), "../A", "B").is_err());
        assert!(remove(dir.path(), "").is_err());
        assert_eq!(load(dir.path()).unwrap(), original);
    }

    #[test]
    fn read_and_write_errors_are_reported_without_replacing_data() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".aquilum"), "blocked").unwrap();
        assert!(matches!(load(dir.path()), Err(PluginError::Io { .. })));
        assert!(matches!(save(dir.path(), &sample()), Err(PluginError::Io { .. })));
    }

    #[test]
    fn an_existing_unreadable_backup_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join(".aquilum");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("plugins.json"), "{new broken").unwrap();
        fs::write(folder.join("plugins.json.unreadable"), "old broken").unwrap();
        assert_eq!(load(dir.path()).unwrap(), VaultPluginData::default());
        assert_eq!(fs::read_to_string(folder.join("plugins.json.unreadable.1")).unwrap(), "{new broken");
        assert_eq!(fs::read_to_string(folder.join("plugins.json.unreadable")).unwrap(), "old broken");
    }

    #[test]
    fn concurrent_relocations_keep_both_updates() {
        let dir = tempfile::tempdir().unwrap();
        let mut data = VaultPluginData::default();
        data.colors.insert("A".to_owned(), "blue".to_owned());
        data.colors.insert("B".to_owned(), "red".to_owned());
        save(dir.path(), &data).unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| relocate(dir.path(), "A", "C").unwrap());
            scope.spawn(|| relocate(dir.path(), "B", "D").unwrap());
        });
        assert_eq!(serde_json::to_value(load(dir.path()).unwrap().colors).unwrap(), json!({"C":"blue", "D":"red"}));
    }
}
