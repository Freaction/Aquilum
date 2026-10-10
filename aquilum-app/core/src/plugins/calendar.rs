use super::{dates::LocalDateTime, error::PluginError};
use crate::Core;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDay {
    pub date: String, pub in_month: bool, pub is_today: bool, pub path: String,
    pub exists: bool, pub words: usize, pub dots: usize, pub open_tasks: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarWeek {
    pub number: i64, pub key: String, pub path: Option<String>, pub exists: bool, pub days: Vec<CalendarDay>,
}
#[derive(Debug, Serialize)]
pub struct CalendarMonth { pub weeks: Vec<CalendarWeek> }
#[derive(Debug, Serialize)]
pub struct OpenedNote { pub path: String, pub created: bool }

fn invalid(message: &str) -> PluginError { PluginError::Invalid { message: message.to_owned() } }
fn file_error(error: crate::files::error::FileCommandError) -> PluginError {
    PluginError::Io { message: error.to_string() }
}

fn validate(date: LocalDateTime, locale: &str) -> Result<(), PluginError> {
    if !date.valid() { return Err(invalid("Недопустимая дата")); }
    if locale != "ru" && locale != "en" { return Err(invalid("Недопустимая локаль")); }
    Ok(())
}

fn week_start(value: &str, locale: &str) -> Result<i64, PluginError> {
    match value {
        "monday" => Ok(1), "sunday" => Ok(0), "locale" => Ok(i64::from(locale == "ru")),
        _ => Err(invalid("Недопустимое начало недели")),
    }
}

fn relative(value: &str) -> Result<String, PluginError> {
    let path = value.replace('\\', "/");
    if path.starts_with('/') || path.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c))
        || path.split('/').any(|part| part.starts_with('.')) {
        return Err(invalid("Недопустимый путь заметки"));
    }
    Ok(path.split('/').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("/"))
}

fn vault_path(root: &Path, relative: &str) -> Result<std::path::PathBuf, PluginError> {
    let mut path = root.to_path_buf();
    for segment in relative.split('/').filter(|part| !part.is_empty()) {
        path.push(segment);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Err(invalid("Символические ссылки в пути заметки запрещены")),
            Ok(_) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error.into()),
        }
    }
    Ok(path)
}

fn note_path(root: &Path, folder: &str, pattern: &str, date: &LocalDateTime, locale: &str) -> Result<std::path::PathBuf, PluginError> {
    if pattern.trim().is_empty() { return Err(invalid("Укажите формат заметки")); }
    let folder = relative(folder)?;
    let name = relative(&super::moment::format(pattern, date, locale))?;
    if name.is_empty() { return Err(invalid("Укажите имя заметки")); }
    let path = if folder.is_empty() { format!("{name}.md") } else { format!("{folder}/{name}.md") };
    vault_path(root, &path)
}

fn response_path(workspace: &Path, root: &Path, path: &Path) -> String {
    workspace.join(path.strip_prefix(root).expect("проверенный путь хранилища"))
        .to_string_lossy().into_owned()
}

fn read(path: &Path) -> Result<Option<String>, PluginError> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn month(core: &Core, workspace: &Path, year: i64, month: i64, locale: &str, today: LocalDateTime) -> Result<CalendarMonth, PluginError> {
    validate(today, locale)?;
    let first = LocalDateTime { year, month, day: 1, hour: 0, minute: 0, second: 0 };
    validate(first, locale)?;
    let root = std::fs::canonicalize(workspace)?;
    let config = core.settings.get_config();
    let daily = &config.daily_notes;
    let weekly = &config.plugins.calendar.weekly;
    let start = week_start(&daily.week_start, locale)?;
    let mut date = first.start_of_week(start);
    let last = LocalDateTime { day: crate::search::note_date::days_in_month(year, month), ..first }.start_of_week(start);
    let mut weeks = Vec::new();
    while (date.year, date.month, date.day) <= (last.year, last.month, last.day) {
        let number = if start == 1 { date.iso_week().1 } else { date.local_week().1 };
        let path = if weekly.enabled { Some(note_path(&root, &weekly.folder, &weekly.format, &date, locale)?) } else { None };
        let exists = match &path { Some(path) => read(path)?.is_some(), None => false };
        let mut days = Vec::with_capacity(7);
        for offset in 0..7 {
            let day = date.add_days(offset);
            let path = note_path(&root, &daily.folder, &daily.format, &day, locale)?;
            let content = read(&path)?;
            let words = content.as_deref().map(super::words::count_words).unwrap_or(0);
            let dots = if words > 0 && daily.words_per_dot > 0 { (words / daily.words_per_dot as usize).clamp(1, 5) } else { 0 };
            days.push(CalendarDay {
                date: day.date(), in_month: day.year == year && day.month == month,
                is_today: day.date() == today.date(), path: response_path(workspace, &root, &path),
                exists: content.is_some(), words, dots,
                open_tasks: content.as_deref().is_some_and(super::words::has_open_tasks),
            });
        }
        weeks.push(CalendarWeek { number, key: date.date(), path: path.map(|path| response_path(workspace, &root, &path)), exists, days });
        date = date.add_days(7);
    }
    Ok(CalendarMonth { weeks })
}

pub fn open(core: &Core, workspace: &Path, period: &str, date: &str, create: bool, now: LocalDateTime, locale: &str) -> Result<Option<OpenedNote>, PluginError> {
    validate(now, locale)?;
    let mut date = LocalDateTime::parse(date).ok_or_else(|| invalid("Недопустимая дата"))?;
    let config = core.settings.get_config();
    let (folder, pattern, template) = match period {
        "day" => (&config.daily_notes.folder, &config.daily_notes.format, &config.daily_notes.template),
        "week" => {
            date = date.start_of_week(week_start(&config.daily_notes.week_start, locale)?);
            let weekly = &config.plugins.calendar.weekly;
            (&weekly.folder, &weekly.format, &weekly.template)
        },
        _ => return Err(invalid("Недопустимый период")),
    };
    let root = std::fs::canonicalize(workspace)?;
    let path = note_path(&root, folder, pattern, &date, locale)?;
    let result = OpenedNote { path: response_path(workspace, &root, &path), created: false };
    if read(&path)?.is_some() { return Ok(Some(result)); }
    if !create { return Ok(None); }
    let content = if template.is_empty() { String::new() } else {
        let template = relative(template)?;
        if template.is_empty() { return Err(invalid("Укажите путь шаблона")); }
        let template_path = vault_path(&root, &template)?;
        let text = match std::fs::read_to_string(&template_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && template_path.extension().is_none() => {
                std::fs::read_to_string(vault_path(&root, &format!("{template}.md"))?)?
            },
            result => result?,
        };
        let title = path.file_stem().and_then(|name| name.to_str()).unwrap_or_default();
        super::moment::apply_placeholders(&text, &date, &now, title, locale)
    };
    crate::files::document::ensure_directory_impl(path.parent().ok_or_else(|| invalid("Недопустимый путь"))?).map_err(file_error)?;
    let created = match crate::files::gate::create(core, &path, &content, crate::history::Source::Me, None) {
        Ok(_) => true,
        Err(crate::files::error::FileCommandError::AlreadyExists { .. }) => false,
        Err(error) => return Err(file_error(error)),
    };
    Ok(Some(OpenedNote { created, ..result }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::Arc};
    fn setup() -> (tempfile::TempDir, Arc<Core>) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("app-data")).unwrap();
        let core = Core::open(&dir.path().join("app-data"), Arc::new(|_: crate::CoreEvent| {}));
        core.ui_state.resolve_workspace(dir.path().to_str().unwrap(), 0).unwrap();
        (dir, core)
    }
    fn today() -> LocalDateTime { LocalDateTime::parse("2026-10-08").unwrap() }
    #[test]
    fn month_grid_indicators_and_week_notes() {
        let (dir, core) = setup();
        fs::write(dir.path().join("2026-10-08.md"), "one two\n- [ ] todo").unwrap();
        fs::write(dir.path().join("2026-10-09.md"), "").unwrap();
        fs::write(dir.path().join("2026-W41.md"), "weekly").unwrap();
        let mut config = core.settings.get_config();
        config.daily_notes.words_per_dot = 1;
        config.plugins.calendar.weekly.enabled = true;
        core.settings.update_config(config).unwrap();
        let result = month(&core, dir.path(), 2026, 10, "ru", today()).unwrap();
        assert_eq!(result.weeks.len(), 5);
        assert_eq!(result.weeks[0].days[0].date, "2026-09-28");
        assert_eq!(result.weeks[4].days[6].date, "2026-11-01");
        assert!(!result.weeks[0].days[0].in_month);
        assert!(!result.weeks[4].days[6].in_month);
        assert_eq!(result.weeks[1].number, 41);
        assert!(result.weeks[1].exists);
        let day = &result.weeks[1].days[3];
        assert!(day.is_today && day.exists && day.open_tasks);
        assert_eq!((day.words, day.dots), (4,4));
        let empty = &result.weeks[1].days[4];
        assert!(empty.exists);
        assert_eq!((empty.words, empty.dots), (0,0));
        assert!(!result.weeks[1].days[5].exists);
        let mut config = core.settings.get_config(); config.daily_notes.words_per_dot = 0;
        core.settings.update_config(config).unwrap();
        assert_eq!(month(&core, dir.path(), 2026, 10, "ru", today()).unwrap().weeks[1].days[3].dots, 0);
        core.shutdown();
    }
    #[test]
    fn reads_extensionless_daily_and_weekly_templates_with_md_fallback() {
        let (dir, core) = setup();
        fs::create_dir(dir.path().join("Templates.v1")).unwrap();
        fs::write(dir.path().join("Templates.v1/Daily.md"), "daily {{date}}").unwrap();
        fs::write(dir.path().join("Templates.v1/Week.md"), "weekly {{title}}").unwrap();
        let mut config = core.settings.get_config();
        config.daily_notes.template = "Templates.v1/Daily".into();
        config.plugins.calendar.weekly.template = "Templates.v1/Week".into();
        core.settings.update_config(config).unwrap();
        let daily = open(&core, dir.path(), "day", "2026-10-08", true, today(), "en").unwrap().unwrap();
        let weekly = open(&core, dir.path(), "week", "2026-10-08", true, today(), "en").unwrap().unwrap();
        assert_eq!(fs::read_to_string(daily.path).unwrap(), "daily 2026-10-08");
        assert_eq!(fs::read_to_string(weekly.path).unwrap(), "weekly 2026-W41");
        // An exact extensionless file takes precedence over the fallback.
        fs::write(dir.path().join("Templates.v1/Daily"), "exact").unwrap();
        let daily = open(&core, dir.path(), "day", "2026-10-09", true, today(), "en").unwrap().unwrap();
        assert_eq!(fs::read_to_string(daily.path).unwrap(), "exact");
        core.shutdown();
    }

    #[cfg(unix)]
    #[test]
    fn template_md_fallback_rejects_symlinks() {
        let (dir, core) = setup();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.md"), "outside").unwrap();
        std::os::unix::fs::symlink(outside.path().join("secret.md"), dir.path().join("template.md")).unwrap();
        let mut config = core.settings.get_config();
        config.daily_notes.template = "template".into();
        core.settings.update_config(config).unwrap();
        assert!(open(&core, dir.path(), "day", "2026-10-08", true, today(), "en").is_err());
        assert!(!dir.path().join("2026-10-08.md").exists());
        core.shutdown();
    }

    #[test]
    fn nested_creation_template_and_existing_file() {
        let (dir, core) = setup();
        fs::write(dir.path().join("template.md"), "{{date}} {{time}} {{title}} {{date:DDDD}}").unwrap();
        let mut config = core.settings.get_config();
        config.daily_notes.folder = "Daily".into(); config.daily_notes.format = "YYYY/MM/DD".into();
        config.daily_notes.template = "template.md".into();
        config.plugins.calendar.weekly.format = "GGGG-[W]WW".into();
        core.settings.update_config(config).unwrap();
        let now = LocalDateTime { hour: 12, minute: 34, ..today() };
        assert!(open(&core, dir.path(), "day", "2026-10-08", false, now, "ru").unwrap().is_none());
        let note = open(&core, dir.path(), "day", "2026-10-08", true, now, "ru").unwrap().unwrap();
        assert!(note.created);
        assert!(Path::new(&note.path).ends_with("Daily/2026/10/08.md"));
        assert_eq!(fs::read_to_string(&note.path).unwrap(), "2026-10-08 12:34 08 четверг");
        assert!(!open(&core, dir.path(), "day", "2026-10-08", true, now, "ru").unwrap().unwrap().created);
        let week = open(&core, dir.path(), "week", "2021-01-03", true, now, "ru").unwrap().unwrap();
        assert!(Path::new(&week.path).ends_with("2020-W53.md"));
        core.shutdown();
    }
    #[test]
    fn unsafe_paths_and_invalid_inputs_are_rejected() {
        let (dir, core) = setup();
        for path in ["../x", ".hidden/x", "/x", "x:y", "x?y", "x\u{7f}y"] {
            let mut config = core.settings.get_config(); config.daily_notes.folder = path.into();
            core.settings.update_config(config).unwrap();
            assert!(open(&core, dir.path(), "day", "2026-10-08", true, today(), "ru").is_err());
        }
        assert!(month(&core, dir.path(), 2026, 13, "ru", today()).is_err());
        assert!(open(&core, dir.path(), "day", "2023-02-29", true, today(), "ru").is_err());
        core.shutdown();
    }
    #[test]
    fn sunday_grid_uses_local_week_numbers() {
        let (dir, core) = setup();
        let result = month(&core, dir.path(), 2021, 1, "en", today()).unwrap();
        assert_eq!(result.weeks[0].number, 1);
        assert_eq!(result.weeks[0].days[0].date, "2020-12-27");
        assert_eq!(result.weeks[1].number, 2);
        assert_eq!(month(&core, dir.path(), 9999, 12, "en", today()).unwrap().weeks.len(), 5);
        core.shutdown();
    }
    #[test]
    fn dot_minimum_cap_and_unsafe_format_or_template() {
        let (dir, core) = setup();
        fs::write(dir.path().join("2026-10-08.md"), "word").unwrap();
        fs::write(dir.path().join("2026-10-09.md"), "word ".repeat(1500)).unwrap();
        let result = month(&core, dir.path(), 2026, 10, "ru", today()).unwrap();
        assert_eq!(result.weeks[1].days[3].dots, 1);
        assert_eq!(result.weeks[1].days[4].dots, 5);
        let mut config = core.settings.get_config();
        config.daily_notes.format = "[../x]".into();
        core.settings.update_config(config).unwrap();
        assert!(month(&core, dir.path(), 2026, 10, "ru", today()).is_err());
        let mut config = core.settings.get_config();
        config.daily_notes.format = "YYYY-MM-DD".into();
        config.daily_notes.template = "../x".into();
        core.settings.update_config(config).unwrap();
        assert!(open(&core, dir.path(), "day", "2026-10-10", true, today(), "ru").is_err());
        assert!(!dir.path().join("2026-10-10.md").exists());
        core.shutdown();
    }
    #[cfg(unix)]
    #[test]
    fn response_paths_preserve_workspace_symlink() {
        let (dir, core) = setup();
        let alias_dir = tempfile::tempdir().unwrap();
        let workspace = alias_dir.path().join("vault-link");
        std::os::unix::fs::symlink(dir.path(), &workspace).unwrap();
        let mut config = core.settings.get_config();
        config.plugins.calendar.weekly.enabled = true;
        core.settings.update_config(config).unwrap();
        let result = month(&core, &workspace, 2026, 10, "ru", today()).unwrap();
        assert!(Path::new(&result.weeks[0].days[0].path).starts_with(&workspace));
        assert!(Path::new(result.weeks[0].path.as_ref().unwrap()).starts_with(&workspace));
        let note = open(&core, &workspace, "day", "2026-10-08", true, today(), "ru").unwrap().unwrap();
        assert!(Path::new(&note.path).starts_with(&workspace));
        assert!(dir.path().join("2026-10-08.md").is_file());
        let existing = open(&core, &workspace, "day", "2026-10-08", false, today(), "ru").unwrap().unwrap();
        assert_eq!(existing.path, note.path);
        assert!(!existing.created);
        core.shutdown();
    }
    #[cfg(unix)]
    #[test]
    fn symlink_cannot_escape_workspace() {
        let (dir, core) = setup(); let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("Daily")).unwrap();
        let mut config = core.settings.get_config(); config.daily_notes.folder = "Daily".into();
        core.settings.update_config(config).unwrap();
        assert!(open(&core, dir.path(), "day", "2026-10-08", true, today(), "ru").is_err());
        assert!(!outside.path().join("2026-10-08.md").exists());
        core.shutdown();
    }
}
