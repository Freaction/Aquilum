use super::error::PluginError;
use super::vault_data::VaultPluginData;
use crate::settings::models::AppConfig;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: Vec<String>,
    pub skipped: Vec<Skipped>,
    pub overwritten: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Skipped {
    pub id: String,
    pub reason: String,
}

pub struct ImportResult {
    pub config: AppConfig,
    pub data: VaultPluginData,
    pub report: ImportReport,
}

pub fn detect(root: &Path) -> bool {
    root.join(".obsidian").is_dir()
}

pub fn import(root: &Path, config: &AppConfig, data: &VaultPluginData) -> Result<ImportResult, PluginError> {
    if !detect(root) {
        return Err(PluginError::Invalid { message: "В хранилище нет .obsidian".into() });
    }
    let mut report = ImportReport::default();
    let mut config = serde_json::to_value(config)
        .map_err(|error| PluginError::Invalid { message: error.to_string() })?;
    let mut data = data.clone();
    let obsidian = root.join(".obsidian");
    if let Some(daily) = read_json(&obsidian.join("daily-notes.json"), "daily-notes", &mut report) {
        if daily.is_object() {
            copy_fields(&mut config, &daily, &[
                ("folder", "/dailyNotes/folder"), ("format", "/dailyNotes/format"),
                ("template", "/dailyNotes/template"),
            ], "daily-notes", &mut report);
            report.imported.push("daily-notes".into());
        } else { skip(&mut report, "daily-notes", "invalid_data"); }
    }
    if let Some(community) = read_json(&obsidian.join("community-plugins.json"), "community-plugins", &mut report) {
        if let Some(ids) = community.as_array() {
            let mut seen = HashSet::new();
            for value in ids {
                let Some(id) = value.as_str() else {
                    skip(&mut report, "community-plugins", "invalid_data");
                    continue;
                };
                if !seen.insert(id) { continue; }
                let plugin = match id {
                    "calendar" => "calendar",
                    "file-explorer-note-count" => "folderCounts",
                    "obsidian-file-color" => "fileColors",
                    "obsidian-icon-folder" => "fileIcons",
                    "file-explorer-plus" => "explorerFilters",
                    "colored-tags" => "coloredTags",
                    "ninja-cursor" => "cursorTrail",
                    "code-styler" => "codeStyler",
                    "toggle-all-reading" => "readingMode",
                    "table-editor-obsidian" => "advancedTables",
                    "obsidian-git" => "gitSync",
                    "obsidian-local-rest-api" | "mcp-tools-istefox" => {
                        skip(&mut report, id, "covered_by_mcp"); continue;
                    }
                    _ => { skip(&mut report, id, "unsupported_plugin"); continue; }
                };
                // Только известные id участвуют в построении пути.
                let path = obsidian.join("plugins").join(id).join("data.json");
                let failures = report.skipped.len();
                let source = read_json(&path, id, &mut report).unwrap_or_else(|| serde_json::json!({}));
                if failures != report.skipped.len() { continue; }
                if !source.is_object() { skip(&mut report, id, "invalid_data"); continue; }
                config["plugins"][plugin]["enabled"] = Value::Bool(true);
                match id {
                    "calendar" => copy_fields(&mut config, &source, &[
                        ("shouldConfirmBeforeCreate", "/dailyNotes/confirmBeforeCreate"),
                        ("weekStart", "/dailyNotes/weekStart"), ("wordsPerDot", "/dailyNotes/wordsPerDot"),
                        ("showWeeklyNote", "/plugins/calendar/showWeekNumbers"),
                        ("showWeeklyNote", "/plugins/calendar/weekly/enabled"),
                        ("weeklyNoteFolder", "/plugins/calendar/weekly/folder"),
                        ("weeklyNoteFormat", "/plugins/calendar/weekly/format"),
                        ("weeklyNoteTemplate", "/plugins/calendar/weekly/template"),
                    ], id, &mut report),
                    "colored-tags" => copy_fields(&mut config, &source,
                        &[("mixColors", "/plugins/coloredTags/mixNested")], id, &mut report),
                    "obsidian-git" => {
                        copy_fields(&mut config, &source, &[
                            ("commitMessage", "/plugins/gitSync/commitMessage"),
                            ("autoSaveInterval", "/plugins/gitSync/autoBackupMinutes"),
                            ("autoPullOnBoot", "/plugins/gitSync/pullOnOpen"),
                            ("syncMethod", "/plugins/gitSync/syncMethod"),
                        ], id, &mut report);
                        if let Some(disabled) = source.get("disablePush") {
                            if let Some(disabled) = disabled.as_bool() {
                                set_field(&mut config, "/plugins/gitSync/push", Value::Bool(!disabled), id, &mut report);
                            } else { skip(&mut report, id, "invalid_field"); }
                        }
                    }
                    "obsidian-icon-folder" => {
                        for (path, value) in source.as_object().unwrap() {
                            if path == "settings" { continue; }
                            let icon = value.as_str().or_else(|| value.get("iconName").and_then(Value::as_str));
                            if !valid_path(path) { skip(&mut report, id, "invalid_path"); continue; }
                            if let Some(icon) = icon.and_then(lucide_name) {
                                merge_value(&mut data.icons, path, icon, &mut report);
                            } else { skip(&mut report, id, "unsupported_icon"); }
                        }
                    }
                    "obsidian-file-color" => import_colors(&source, &mut data, &mut report),
                    "file-explorer-plus" => {
                        import_filters(&source["hideFilters"], &mut data.filters.hide, &mut report);
                        import_filters(&source["pinFilters"], &mut data.filters.pin, &mut report);
                    }
                    _ => {}
                }
                if !report.imported.iter().any(|item| item == id) { report.imported.push(id.into()); }
            }
        } else { skip(&mut report, "community-plugins", "invalid_data"); }
    }
    let config = serde_json::from_value(config)
        .map_err(|error| PluginError::Invalid { message: error.to_string() })?;
    Ok(ImportResult { config, data, report })
}

fn skip(report: &mut ImportReport, id: &str, reason: &str) {
    if !report.skipped.iter().any(|row| row.id == id && row.reason == reason) {
        report.skipped.push(Skipped { id: id.into(), reason: reason.into() });
    }
}

fn read_json(path: &Path, id: &str, report: &mut ImportReport) -> Option<Value> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => { skip(report, id, "read_failed"); return None; }
    };
    match serde_json::from_slice(&bytes) {
        Ok(value) => Some(value),
        Err(_) => { skip(report, id, "invalid_data"); None }
    }
}

fn overwritten(report: &mut ImportReport, item: &str) {
    if !report.overwritten.iter().any(|path| path == item) { report.overwritten.push(item.into()); }
}

fn set_field(config: &mut Value, path: &str, value: Value, id: &str, report: &mut ImportReport) {
    let target = config.pointer_mut(path).expect("поле настроек");
    if *target != value { overwritten(report, id); *target = value; }
}

fn copy_fields(config: &mut Value, source: &Value, fields: &[(&str, &str)], id: &str, report: &mut ImportReport) {
    for (key, path) in fields {
        let Some(value) = source.get(key) else { continue; };
        let target = config.pointer(path).expect("поле настроек");
        let valid = (target.is_boolean() && value.is_boolean())
            || (target.is_string() && value.is_string())
            || (target.is_number() && value.as_u64().is_some_and(|n| n <= u32::MAX as u64));
        let supported = match *key {
            "weekStart" => matches!(value.as_str(), Some("locale" | "monday" | "sunday")),
            "syncMethod" => matches!(value.as_str(), Some("merge" | "rebase")),
            _ => true,
        };
        if !valid || !supported { skip(report, id, "invalid_field"); continue; }
        // Пустой формат Obsidian означает формат по умолчанию.
        if matches!(*key, "format" | "weeklyNoteFormat") && value.as_str() == Some("") { continue; }
        let value = if matches!(*key, "template" | "weeklyNoteTemplate")
            && value.as_str().is_some_and(|path| !path.is_empty() && Path::new(path).extension().is_none()) {
            Value::String(format!("{}.md", value.as_str().unwrap()))
        } else { value.clone() };
        set_field(config, path, value, id, report);
    }
}

fn valid_path(path: &str) -> bool {
    !path.contains(['\\', ':']) && !path.chars().any(char::is_control)
        && !path.split('/').any(|part| part.is_empty() || part == "." || part == "..")
}

fn merge_value(map: &mut BTreeMap<String, String>, path: &str, value: String, report: &mut ImportReport) {
    if map.get(path).is_some_and(|old| old != &value) { overwritten(report, path); }
    map.insert(path.into(), value);
}

fn lucide_name(name: &str) -> Option<String> {
    let name = name.strip_prefix("Li")?;
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_alphanumeric()) { return None; }
    let bytes = name.as_bytes();
    let mut result = String::new();
    for (index, &byte) in bytes.iter().enumerate() {
        if index > 0 {
            let previous = bytes[index - 1];
            let boundary = byte.is_ascii_uppercase() && (previous.is_ascii_lowercase() || previous.is_ascii_digit()
                || bytes.get(index + 1).is_some_and(u8::is_ascii_lowercase))
                || byte.is_ascii_digit() && previous.is_ascii_alphabetic();
            if boundary { result.push('-'); }
        }
        result.push(byte.to_ascii_lowercase() as char);
    }
    Some(result)
}

fn color_token(hex: &str) -> Option<&'static str> {
    let hex = hex.strip_prefix('#')?;
    let hex = if hex.len() == 3 {
        hex.chars().flat_map(|ch| [ch, ch]).collect::<String>()
    } else { hex.to_owned() };
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) { return None; }
    let color = u32::from_str_radix(&hex, 16).ok()?;
    // Опорные цвета --q-<token>-500 из styles/tokens/colors.css.
    let palette = [("red", 0xff3c29), ("amber", 0xff8400), ("green", 0x03c500), ("teal", 0x14b8a6),
        ("blue", 0x1471eb), ("purple", 0x9333ea), ("pink", 0xec4899), ("gray", 0x5d636f)];
    palette.into_iter().min_by_key(|(_, reference)| {
        [0, 8, 16].into_iter().map(|shift| {
            let difference = ((color >> shift) & 255) as i32 - ((reference >> shift) & 255) as i32;
            difference * difference
        }).sum::<i32>()
    }).map(|(token, _)| token)
}

fn import_colors(source: &Value, data: &mut VaultPluginData, report: &mut ImportReport) {
    let id = "obsidian-file-color";
    if let Some(rows) = source.get("fileColors").and_then(Value::as_array) {
        for row in rows {
            let Some(path) = row.get("path").and_then(Value::as_str) else { skip(report, id, "invalid_data"); continue; };
            if !valid_path(path) { skip(report, id, "invalid_path"); continue; }
            let color = row.get("color").and_then(Value::as_str);
            let hex = color.filter(|text| text.starts_with('#')).or_else(|| {
                source.get("palette")?.as_array()?.iter()
                    .find(|row| row.get("id").and_then(Value::as_str) == color)?
                    .get("value")?.as_str()
            });
            if let Some(token) = hex.and_then(color_token) {
                merge_value(&mut data.colors, path, token.into(), report);
            } else { skip(report, id, "invalid_color"); }
        }
    } else if source.get("fileColors").is_some() { skip(report, id, "invalid_data"); }
}

fn import_filters(source: &Value, target: &mut Vec<super::vault_data::Filter>, report: &mut ImportReport) {
    use super::vault_data::{Filter, FilterKind, FilterTarget, PatternType};
    let id = "file-explorer-plus";
    if source.is_null() { return; }
    if !source.is_object() { skip(report, id, "invalid_data"); return; }
    let group_active = source.get("active").and_then(Value::as_bool).unwrap_or(true);
    for (key, kind) in [("paths", FilterKind::Path), ("tags", FilterKind::Tag)] {
        let Some(rows) = source.get(key) else { continue; };
        let Some(rows) = rows.as_array() else { skip(report, id, "invalid_data"); continue; };
        for row in rows {
            let Some(pattern) = row.get("pattern").and_then(Value::as_str) else { skip(report, id, "invalid_data"); continue; };
            if pattern.trim().is_empty() { continue; }
            let pattern_type = match row.get("patternType").and_then(Value::as_str) {
                Some("STRICT") => PatternType::Strict,
                Some("WILDCARD") => PatternType::Wildcard,
                Some("REGEX") => PatternType::Regex,
                _ => { skip(report, id, "invalid_field"); continue; }
            };
            let filter_target = if kind == FilterKind::Tag { FilterTarget::Files } else {
                match row.get("type").and_then(Value::as_str) {
                    Some("FILES") => FilterTarget::Files,
                    Some("DIRECTORIES") => FilterTarget::Folders,
                    Some("FILES_AND_DIRECTORIES") => FilterTarget::Both,
                    _ => { skip(report, id, "invalid_field"); continue; }
                }
            };
            if kind == FilterKind::Path && pattern_type == PatternType::Strict && !valid_path(pattern) {
                skip(report, id, "invalid_path"); continue;
            }
            let filter = Filter {
                name: row.get("name").and_then(Value::as_str).unwrap_or("").into(),
                active: group_active && row.get("active").and_then(Value::as_bool).unwrap_or(true),
                kind, target: filter_target, pattern_type,
                pattern: if kind == FilterKind::Tag { pattern.trim_start_matches('#') } else { pattern }.into(),
            };
            if filter.pattern.trim().is_empty() { continue; }
            if let Some(old) = target.iter_mut().find(|old| old.kind == filter.kind && old.target == filter.target
                && old.pattern_type == filter.pattern_type && old.pattern == filter.pattern) {
                if *old != filter { overwritten(report, &filter.pattern); *old = filter; }
            } else { target.push(filter); }
        }
    }
    if let Some(rows) = source.get("frontMatter").and_then(Value::as_array) {
        if !rows.is_empty() { skip(report, id, "front_matter"); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::fs;

    fn write(root: &Path, relative: &str, value: Value) {
        let path = root.join(".obsidian").join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    }

    #[test]
    fn converts_lucide_names_and_hex_colors() {
        for (source, target) in [("LiCalendarDays", "calendar-days"), ("LiGamepad2", "gamepad-2"), ("LiBicepsFlexed", "biceps-flexed"), ("Li2", "2"), ("LiUSB", "usb")] {
            assert_eq!(lucide_name(source).as_deref(), Some(target));
        }
        for name in ["FaBook", "😀", "Li", "LiBad/name"] { assert_eq!(lucide_name(name), None); }
        for (hex, target) in [("#ff3c29", "red"), ("#ff8400", "amber"), ("#03c500", "green"), ("#14b8a6", "teal"), ("#1471eb", "blue"), ("#9333ea", "purple"), ("#ec4899", "pink"), ("#5d636f", "gray"), ("#FF3C28", "red")] {
            assert_eq!(color_token(hex), Some(target));
        }
        assert!(color_token("#f00").is_some());
        assert_eq!(color_token("invalid"), None);
    }

    #[test]
    fn template_paths_without_extensions_gain_md_only_when_nonempty() {
        for (daily, weekly, expected_daily, expected_weekly) in [
            ("Templates/Daily", "Templates/Week", "Templates/Daily.md", "Templates/Week.md"),
            ("Templates.v1/Daily", "Templates/Week.txt", "Templates.v1/Daily.md", "Templates/Week.txt"),
            ("", "", "", ""),
        ] {
            let dir = tempfile::tempdir().unwrap();
            write(dir.path(), "daily-notes.json", json!({"template": daily}));
            write(dir.path(), "community-plugins.json", json!(["calendar"]));
            write(dir.path(), "plugins/calendar/data.json", json!({"weeklyNoteTemplate": weekly}));
            let result = import(dir.path(), &AppConfig::default(), &VaultPluginData::default()).unwrap();
            assert_eq!(result.config.daily_notes.template, expected_daily);
            assert_eq!(result.config.plugins.calendar.weekly.template, expected_weekly);
        }
    }

    #[test]
    fn detects_only_an_obsidian_directory() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!detect(dir.path()));
        fs::write(dir.path().join(".obsidian"), "file").unwrap();
        assert!(!detect(dir.path()));
        fs::remove_file(dir.path().join(".obsidian")).unwrap();
        fs::create_dir(dir.path().join(".obsidian")).unwrap();
        assert!(detect(dir.path()));
    }

    #[test]
    fn empty_daily_format_preserves_the_current_format() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "daily-notes.json", json!({"folder":"D", "format":""}));
        let mut config = AppConfig::default();
        config.daily_notes.format = "YYYY/MM/DD".into();
        let result = import(dir.path(), &config, &VaultPluginData::default()).unwrap();
        assert_eq!(result.config.daily_notes.folder, "D");
        assert_eq!(result.config.daily_notes.format, "YYYY/MM/DD");
    }

    #[test]
    fn imports_every_fr102_branch_and_preserves_existing_values_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "community-plugins.json", json!([
            "calendar", "file-explorer-note-count", "obsidian-file-color", "obsidian-icon-folder",
            "file-explorer-plus", "colored-tags", "ninja-cursor", "code-styler", "toggle-all-reading",
            "table-editor-obsidian", "obsidian-git", "obsidian-local-rest-api", "mcp-tools-istefox", "unknown",
        ]));
        write(dir.path(), "daily-notes.json", json!({"folder":"Daily", "format":"YYYY/MM/DD", "template":"Templates/Daily.md"}));
        write(dir.path(), "plugins/calendar/data.json", json!({
            "shouldConfirmBeforeCreate":false,"weekStart":"monday","wordsPerDot":100,
            "showWeeklyNote":true,"weeklyNoteFolder":"Weekly","weeklyNoteFormat":"gggg-[W]ww","weeklyNoteTemplate":"Templates/Week.md",
        }));
        write(dir.path(), "plugins/colored-tags/data.json", json!({"mixColors":false}));
        write(dir.path(), "plugins/obsidian-git/data.json", json!({"commitMessage":"backup {{date}}","autoSaveInterval":10,"autoPullOnBoot":true,"disablePush":true,"syncMethod":"rebase"}));
        write(dir.path(), "plugins/obsidian-icon-folder/data.json", json!({"settings":{},"Projects":"LiGamepad2","Health":{"iconName":"LiBicepsFlexed"},"Other":"FaBook","../bad":"LiBook"}));
        write(dir.path(), "plugins/obsidian-file-color/data.json", json!({"fileColors":[{"path":"Projects","color":"#1471eb"},{"path":"Archive","color":"warm"}],"palette":[{"id":"warm","value":"#ff8400"}]}));
        write(dir.path(), "plugins/file-explorer-plus/data.json", json!({
            "hideFilters":{"active":true,"paths":[
                {"name":"hidden","active":false,"type":"DIRECTORIES","pattern":"Projects","patternType":"STRICT"},
                {"name":"","active":true,"type":"FILES","pattern":"  ","patternType":"WILDCARD"},
                {"name":"regex","active":true,"type":"FILES","pattern":".*\\.tmp$","patternType":"REGEX"}
            ],"tags":[{"name":"tag","active":true,"pattern":"#secret","patternType":"STRICT"}],
                "frontMatter":[{"name":"property","active":true,"path":"state","pattern":"draft","patternType":"STRICT"}]},
            "pinFilters":{"active":false,"paths":[{"name":"pin","active":true,"type":"FILES_AND_DIRECTORIES","pattern":"Pinned/*","patternType":"WILDCARD"}],"tags":[],"frontMatter":[]}
        }));
        let config = AppConfig::default();
        let mut data = VaultPluginData::default();
        data.icons.insert("Existing".into(), "book".into());
        data.icons.insert("Projects".into(), "folder".into());
        data.colors.insert("Existing".into(), "green".into());
        data.filters.hide.push(serde_json::from_value(json!({"name":"existing","active":true,"kind":"path","target":"files","pattern":"Keep.md","patternType":"strict"})).unwrap());
        data.filters.hide.push(serde_json::from_value(json!({"name":"old name","active":true,"kind":"path","target":"folders","pattern":"Projects","patternType":"strict"})).unwrap());
        let source = fs::read(dir.path().join(".obsidian/plugins/obsidian-icon-folder/data.json")).unwrap();
        let result = import(dir.path(), &config, &data).unwrap();
        let plugins = &result.config.plugins;
        assert!(plugins.calendar.enabled && plugins.calendar.show_week_numbers && plugins.calendar.weekly.enabled);
        assert_eq!(plugins.calendar.weekly.folder, "Weekly");
        assert_eq!(plugins.calendar.weekly.template, "Templates/Week.md");
        assert_eq!(result.config.daily_notes.folder, "Daily");
        assert_eq!(result.config.daily_notes.format, "YYYY/MM/DD");
        assert_eq!(result.config.daily_notes.template, "Templates/Daily.md");
        assert!(!result.config.daily_notes.confirm_before_create);
        assert_eq!(result.config.daily_notes.week_start, "monday");
        assert_eq!(result.config.daily_notes.words_per_dot, 100);
        assert!(plugins.folder_counts.enabled && plugins.file_colors.enabled && plugins.file_icons.enabled && plugins.explorer_filters.enabled);
        assert!(plugins.cursor_trail.enabled && plugins.code_styler.enabled && plugins.reading_mode.enabled && plugins.advanced_tables.enabled);
        assert!(plugins.colored_tags.enabled && !plugins.colored_tags.mix_nested);
        assert!(plugins.git_sync.enabled && plugins.git_sync.pull_on_open && !plugins.git_sync.push);
        assert_eq!(plugins.git_sync.auto_backup_minutes, 10);
        assert_eq!(plugins.git_sync.sync_method, "rebase");
        assert_eq!(plugins.git_sync.commit_message, "backup {{date}}");
        assert_eq!(result.data.icons["Projects"], "gamepad-2");
        assert_eq!(result.data.icons["Health"], "biceps-flexed");
        assert_eq!(result.data.icons["Existing"], "book");
        assert_eq!(result.data.colors["Projects"], "blue");
        assert_eq!(result.data.colors["Archive"], "amber");
        assert_eq!(result.data.colors["Existing"], "green");
        assert_eq!(result.data.filters.hide.len(), 4);
        let filters = serde_json::to_value(&result.data.filters).unwrap();
        assert_eq!(filters["hide"][1]["target"], "folders");
        assert_eq!(filters["hide"][1]["patternType"], "strict");
        assert_eq!(filters["hide"][1]["name"], "hidden");
        assert_eq!(filters["hide"][1]["active"], false);
        assert_eq!(filters["hide"][2]["patternType"], "regex");
        assert_eq!(filters["hide"][3]["kind"], "tag");
        assert_eq!(filters["hide"][3]["pattern"], "secret");
        assert_eq!(filters["pin"][0]["target"], "both");
        assert_eq!(filters["pin"][0]["patternType"], "wildcard");
        assert_eq!(filters["pin"][0]["active"], false);
        for id in ["obsidian-local-rest-api", "mcp-tools-istefox"] {
            assert!(result.report.skipped.iter().any(|row| row.id == id && row.reason == "covered_by_mcp"));
        }
        assert!(result.report.skipped.iter().any(|row| row.reason == "front_matter"));
        assert!(result.report.skipped.iter().any(|row| row.id == "unknown"));
        assert!(result.report.overwritten.iter().any(|path| path.contains("Projects")));
        assert_eq!(fs::read(dir.path().join(".obsidian/plugins/obsidian-icon-folder/data.json")).unwrap(), source);
        assert!(!dir.path().join(".aquilum").exists());
        let again = import(dir.path(), &result.config, &result.data).unwrap();
        assert_eq!(again.data, result.data);
    }

    #[test]
    fn broken_and_unknown_data_are_skipped_without_harming_other_plugins() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "community-plugins.json", json!(["colored-tags","file-explorer-note-count","../../outside","obsidian-git"]));
        write(dir.path(), "plugins/colored-tags/data.json", json!({}));
        fs::write(dir.path().join(".obsidian/plugins/colored-tags/data.json"), "{broken").unwrap();
        write(dir.path(), "plugins/obsidian-git/data.json", json!({"autoSaveInterval":-1,"syncMethod":"invalid"}));
        let result = import(dir.path(), &AppConfig::default(), &VaultPluginData::default()).unwrap();
        assert!(result.config.plugins.folder_counts.enabled);
        assert!(!result.config.plugins.colored_tags.enabled);
        assert_eq!(result.config.plugins.git_sync.auto_backup_minutes, 0);
        assert_eq!(result.config.plugins.git_sync.sync_method, "merge");
        assert!(result.report.skipped.iter().any(|row| row.id == "colored-tags" && row.reason == "invalid_data"));
        assert!(result.report.skipped.iter().any(|row| row.id == "../../outside" && row.reason == "unsupported_plugin"));
        assert!(import(Path::new("/nonexistent-import-vault"), &AppConfig::default(), &VaultPluginData::default()).is_err());
    }
}
