use super::error::PluginError;
use super::settings::PluginSettings;
use super::vault_data::{Filter, FilterKind, FilterTarget, Filters, PatternType};
use rusqlite::Connection;
use serde::Serialize;
use globset::{GlobBuilder, GlobMatcher};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use crate::search::fields;
use crate::Core;
use std::path::Path;

#[derive(Debug, Default, Serialize)]
pub struct ExplorerOverview {
    pub counts: BTreeMap<String, usize>,
    pub hidden: Vec<String>,
    pub pinned: Vec<String>,
    pub errors: Vec<FilterError>,
}

#[derive(Debug, Serialize)]
pub struct FilterError {
    pub list: &'static str,
    pub index: usize,
    pub message: String,
}

pub fn overview(core: &Core, root: &Path) -> Result<ExplorerOverview, PluginError> {
    let settings = core.settings.get_config().plugins;
    let data = super::vault_data::load(root)?;
    let needs_tags = settings.explorer_filters.enabled
        && data.filters.hide.iter().chain(&data.filters.pin)
            .any(|filter| filter.active && filter.kind == FilterKind::Tag);
    let connection = if needs_tags {
        core.search.ready_metadata_path(&root.to_string_lossy())
            .map(|path| Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY))
            .transpose().map_err(index_error)?
    } else {
        None
    };
    scan_overview(root, &settings, &data.filters, connection.as_ref())
}

enum Matcher {
    Strict(String),
    Wildcard(GlobMatcher),
    Regex(Regex),
    Tagged(HashSet<String>),
}

impl Matcher {
    fn compile(pattern: &str, pattern_type: PatternType) -> Result<Self, String> {
        match pattern_type {
            PatternType::Strict => Ok(Self::Strict(pattern.to_owned())),
            PatternType::Wildcard => GlobBuilder::new(pattern).literal_separator(false).build()
                .map(|glob| Self::Wildcard(glob.compile_matcher())).map_err(|error| error.to_string()),
            PatternType::Regex => Regex::new(pattern).map(Self::Regex).map_err(|error| error.to_string()),
        }
    }

    fn matches(&self, path: &str, basename: bool) -> bool {
        match self {
            Self::Strict(pattern) => path == pattern,
            Self::Wildcard(matcher) => matcher.is_match(path)
                || (basename && matcher.is_match(path.rsplit('/').next().unwrap_or(path))),
            Self::Regex(matcher) => matcher.is_match(path),
            Self::Tagged(paths) => paths.contains(path),
        }
    }
}

struct CompiledFilter {
    target: FilterTarget,
    matcher: Matcher,
    basename: bool,
}

fn index_error(error: impl std::fmt::Display) -> PluginError {
    PluginError::Io { message: error.to_string() }
}

fn relative(path: &Path, root: &Path) -> Option<String> {
    path.strip_prefix(root).ok().map(|path| {
        path.components().map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>().join("/")
    })
}

fn tag_matches(
    connection: &Connection,
    root: &Path,
    filter: &Filter,
    matcher: &Matcher,
) -> Result<HashSet<String>, PluginError> {
    if filter.pattern_type == PatternType::Strict {
        return Ok(fields::paths_with_tag(connection, &filter.pattern).map_err(index_error)?
            .into_iter().filter_map(|path| relative(Path::new(&path), root)).collect());
    }
    let mut statement = connection.prepare(
        "SELECT path, text, items FROM note_fields WHERE key_lower='tags'"
    ).map_err(index_error)?;
    let rows = statement.query_map([], |row| Ok((
        row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?,
    ))).map_err(index_error)?;
    let mut found = HashSet::new();
    for row in rows {
        let (path, text, items) = row.map_err(index_error)?;
        let tags = match items {
            Some(items) => serde_json::from_str::<Vec<String>>(&items).map_err(index_error)?,
            None => vec![text],
        };
        if tags.iter().any(|tag| matcher.matches(tag, false)) {
            if let Some(path) = relative(Path::new(&path), root) {
                found.insert(path);
            }
        }
    }
    Ok(found)
}

fn compile_filters(
    filters: &[Filter],
    list: &'static str,
    root: &Path,
    connection: Option<&Connection>,
    errors: &mut Vec<FilterError>,
) -> Result<Vec<CompiledFilter>, PluginError> {
    let mut compiled = Vec::new();
    for (index, filter) in filters.iter().enumerate() {
        if !filter.active || (filter.kind == FilterKind::Tag && connection.is_none()) {
            continue;
        }
        let pattern = if filter.kind == FilterKind::Tag {
            filter.pattern.trim_start_matches('#')
        } else {
            &filter.pattern
        };
        let matcher = match Matcher::compile(pattern, filter.pattern_type) {
            Ok(matcher) => matcher,
            Err(message) => {
                errors.push(FilterError { list, index, message });
                continue;
            }
        };
        let matcher = if filter.kind == FilterKind::Tag {
            Matcher::Tagged(tag_matches(connection.unwrap(), root, filter, &matcher)?)
        } else {
            matcher
        };
        compiled.push(CompiledFilter {
            target: filter.target,
            matcher,
            basename: filter.kind == FilterKind::Path && !pattern.contains('/'),
        });
    }
    Ok(compiled)
}

fn matched(filters: &[CompiledFilter], path: &str, folder: bool) -> bool {
    filters.iter().any(|filter| {
        let target = match filter.target {
            FilterTarget::Files => !folder,
            FilterTarget::Folders => folder,
            FilterTarget::Both => true,
        };
        target && filter.matcher.matches(path, filter.basename)
    })
}

fn hidden_with_ancestors(path: &str, hidden: &HashSet<String>) -> bool {
    let mut path = path;
    loop {
        if hidden.contains(path) {
            return true;
        }
        match path.rsplit_once('/') {
            Some((parent, _)) => path = parent,
            None => return false,
        }
    }
}

fn scan_overview(
    root: &Path,
    settings: &PluginSettings,
    filters: &Filters,
    connection: Option<&Connection>,
) -> Result<ExplorerOverview, PluginError> {
    let root = crate::search::paths::canonical_workspace(&root.to_string_lossy()).map_err(index_error)?;
    if !root.is_dir() {
        return Err(PluginError::Invalid { message: "нужна папка хранилища".to_owned() });
    }
    let mut result = ExplorerOverview::default();
    let (hide, pin) = if settings.explorer_filters.enabled {
        (
            compile_filters(&filters.hide, "hide", &root, connection, &mut result.errors)?,
            compile_filters(&filters.pin, "pin", &root, connection, &mut result.errors)?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    let mut entries = Vec::new();
    let mut hidden = HashSet::new();
    let mut pinned = BTreeSet::new();
    result.counts.insert(String::new(), 0);
    for entry in walkdir::WalkDir::new(&root).follow_links(false).into_iter().filter_entry(|entry| {
        entry.depth() == 0 || (!entry.file_name().to_string_lossy().starts_with('.')
            && !entry.file_type().is_symlink())
    }) {
        let entry = entry.map_err(index_error)?;
        if entry.depth() == 0 {
            continue;
        }
        let folder = entry.file_type().is_dir();
        let markdown = crate::search::paths::is_markdown(entry.path());
        if !folder {
            if !entry.file_type().is_file() {
                continue;
            }
            let extension = entry.path().extension().and_then(|value| value.to_str())
                .unwrap_or_default().to_ascii_lowercase();
            // Те же вложения, что в files::workspace::read_directory_impl.
            if !markdown && !crate::files::attachments::MEDIA_EXTENSIONS.contains(&extension.as_str())
                && !["pdf", "epub", "mobi", "azw3", "fb2"].contains(&extension.as_str()) {
                continue;
            }
        }
        let path = relative(entry.path(), &root).unwrap();
        if folder {
            result.counts.insert(path.clone(), 0);
        }
        if matched(&hide, &path, folder) {
            hidden.insert(path.clone());
        }
        if matched(&pin, &path, folder) {
            pinned.insert(path.clone());
        }
        if !folder && (markdown || settings.folder_counts.show_all_files) {
            entries.push(path);
        }
    }
    for path in entries {
        if hidden_with_ancestors(&path, &hidden) {
            continue;
        }
        let mut parent = path.as_str();
        loop {
            parent = parent.rsplit_once('/').map_or("", |(parent, _)| parent);
            *result.counts.get_mut(parent).unwrap() += 1;
            if parent.is_empty() {
                break;
            }
        }
    }
    result.hidden = hidden.into_iter().collect();
    result.hidden.sort();
    result.pinned = pinned.into_iter().collect();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::fields;
    use serde_json::json;
    use std::fs;

    fn settings() -> PluginSettings {
        let mut settings = PluginSettings::default();
        settings.folder_counts.enabled = true;
        settings.explorer_filters.enabled = true;
        settings
    }

    fn filter(kind: FilterKind, target: FilterTarget, pattern: &str, pattern_type: PatternType) -> Filter {
        Filter { name: pattern.to_owned(), active: true, kind, target, pattern: pattern.to_owned(), pattern_type }
    }

    fn path_filter(target: FilterTarget, pattern: &str, pattern_type: PatternType) -> Filter {
        filter(FilterKind::Path, target, pattern, pattern_type)
    }

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for folder in ["A/B", "AB", "Empty", ".hidden/inside", "A/.hidden"] {
            fs::create_dir_all(dir.path().join(folder)).unwrap();
        }
        for file in ["root.md", "A/one.md", "A/TWO.MD", "A/B/three.md", "AB/four.md", "A/photo.PNG", "A/doc.pdf", "A/skip.txt", "A/no-extension", ".hidden/inside/n.md", "A/.hidden/n.md", "A/.note.md"] {
            fs::write(dir.path().join(file), "").unwrap();
        }
        dir
    }

    #[test]
    fn recursive_counts_keep_zeroes_and_hide_zero_is_only_a_ui_setting() {
        let dir = fixture();
        let mut settings = settings();
        let result = scan_overview(dir.path(), &settings, &Filters::default(), None).unwrap();
        assert_eq!(serde_json::to_value(&result.counts).unwrap(), json!({"":5,"A":3,"A/B":1,"AB":1,"Empty":0}));
        settings.folder_counts.hide_zero = false;
        assert_eq!(scan_overview(dir.path(), &settings, &Filters::default(), None).unwrap().counts, result.counts);
        settings.folder_counts.show_all_files = true;
        assert_eq!(serde_json::to_value(scan_overview(dir.path(), &settings, &Filters::default(), None).unwrap().counts).unwrap(), json!({"":7,"A":5,"A/B":1,"AB":1,"Empty":0}));
    }

    #[test]
    fn hiding_a_folder_excludes_descendants_without_hiding_a_similar_prefix() {
        let dir = fixture();
        let filters = Filters { hide: vec![path_filter(FilterTarget::Folders, "A", PatternType::Strict)], pin: vec![path_filter(FilterTarget::Files, "A/B/three.md", PatternType::Strict)] };
        let result = scan_overview(dir.path(), &settings(), &filters, None).unwrap();
        assert_eq!(result.hidden, ["A"]);
        assert_eq!(result.pinned, ["A/B/three.md"]);
        assert_eq!(result.counts[""], 2);
        assert_eq!(result.counts["A"], 0);
        assert_eq!(result.counts["A/B"], 0);
        assert_eq!(result.counts["AB"], 1);
    }

    #[test]
    fn path_patterns_respect_each_target_and_wildcards_match_basename_and_slashes() {
        let dir = fixture();
        let cases = [
            (PatternType::Strict, "A/B/three.md", FilterTarget::Files, vec!["A/B/three.md"]),
            (PatternType::Strict, "A/B", FilterTarget::Folders, vec!["A/B"]),
            (PatternType::Strict, "A/B", FilterTarget::Both, vec!["A/B"]),
            (PatternType::Wildcard, "*.md", FilterTarget::Files, vec!["A/B/three.md", "A/one.md", "AB/four.md", "root.md"]),
            (PatternType::Wildcard, "B", FilterTarget::Folders, vec!["A/B"]),
            (PatternType::Wildcard, "A*", FilterTarget::Both, vec!["A", "A/B", "A/B/three.md", "A/TWO.MD", "A/doc.pdf", "A/one.md", "A/photo.PNG", "AB", "AB/four.md"]),
            (PatternType::Wildcard, "A/*.md", FilterTarget::Files, vec!["A/B/three.md", "A/one.md"]),
            (PatternType::Wildcard, "A/**/three.?d", FilterTarget::Files, vec!["A/B/three.md"]),
            (PatternType::Regex, "^A/.*\\.md$", FilterTarget::Files, vec!["A/B/three.md", "A/one.md"]),
            (PatternType::Regex, "^A(/B)?$", FilterTarget::Folders, vec!["A", "A/B"]),
            (PatternType::Regex, "^A(/B)?$", FilterTarget::Both, vec!["A", "A/B"]),
            (PatternType::Strict, "A", FilterTarget::Files, vec![]),
            (PatternType::Strict, "root.md", FilterTarget::Folders, vec![]),
        ];
        for (pattern_type, pattern, target, expected) in cases {
            let filters = Filters { hide: vec![], pin: vec![path_filter(target, pattern, pattern_type)] };
            let result = scan_overview(dir.path(), &settings(), &filters, None).unwrap();
            assert_eq!(result.pinned, expected, "{pattern_type:?}, {target:?}, {pattern}");
            assert!(result.errors.is_empty());
        }
    }

    #[test]
    fn invalid_patterns_report_original_list_indices_and_do_not_stop_valid_filters() {
        let dir = fixture();
        let mut inactive = path_filter(FilterTarget::Both, "[", PatternType::Regex);
        inactive.active = false;
        let filters = Filters {
            hide: vec![inactive, path_filter(FilterTarget::Both, "[", PatternType::Regex), path_filter(FilterTarget::Files, "root.md", PatternType::Strict)],
            pin: vec![path_filter(FilterTarget::Both, "(", PatternType::Regex), path_filter(FilterTarget::Folders, "Empty", PatternType::Strict)],
        };
        let result = scan_overview(dir.path(), &settings(), &filters, None).unwrap();
        assert_eq!(result.hidden, ["root.md"]);
        assert_eq!(result.pinned, ["Empty"]);
        assert_eq!(result.counts[""], 4);
        assert_eq!(result.errors.len(), 2);
        assert_eq!((result.errors[0].list, result.errors[0].index), ("hide", 1));
        assert_eq!((result.errors[1].list, result.errors[1].index), ("pin", 0));
        assert!(result.errors.iter().all(|error| !error.message.is_empty()));
        assert_eq!(serde_json::to_value(&result).unwrap()["errors"][0]["list"], "hide");
    }

    #[test]
    fn disabled_filters_do_not_hide_pin_or_report_errors() {
        let dir = fixture();
        let mut settings = settings();
        settings.explorer_filters.enabled = false;
        let filters = Filters { hide: vec![path_filter(FilterTarget::Folders, "A", PatternType::Strict)], pin: vec![path_filter(FilterTarget::Both, "[", PatternType::Regex)] };
        let result = scan_overview(dir.path(), &settings, &filters, None).unwrap();
        assert!(result.hidden.is_empty() && result.pinned.is_empty() && result.errors.is_empty());
        assert_eq!(result.counts[""], 5);
    }

    #[test]
    fn tag_filters_use_frontmatter_index_with_nested_tags_and_ignore_missing_index() {
        let dir = fixture();
        let mut connection = Connection::open_in_memory().unwrap();
        fields::open_schema(&connection).unwrap();
        let transaction = connection.transaction().unwrap();
        for (path, body) in [
            ("A/one.md", "---\ntags: [project/alpha, urgent]\n---\n#body-only"),
            ("A/B/three.md", "---\ntags: project\n---"),
            ("AB/four.md", "---\ntags: [projectile]\n---"),
            ("root.md", "#project"),
        ] {
            fields::index_document(&transaction, &crate::search::paths::canonical_path(dir.path()).join(path), body).unwrap();
        }
        transaction.commit().unwrap();
        for (pattern_type, pattern, expected) in [
            (PatternType::Strict, "#PROJECT", vec!["A/B/three.md", "A/one.md"]),
            (PatternType::Wildcard, "project/*", vec!["A/one.md"]),
            (PatternType::Regex, "^urgent$", vec!["A/one.md"]),
        ] {
            let filters = Filters { hide: vec![filter(FilterKind::Tag, FilterTarget::Files, pattern, pattern_type)], pin: vec![filter(FilterKind::Tag, FilterTarget::Both, pattern, pattern_type), filter(FilterKind::Tag, FilterTarget::Folders, pattern, pattern_type)] };
            let result = scan_overview(dir.path(), &settings(), &filters, Some(&connection)).unwrap();
            assert_eq!(result.hidden, expected);
            assert_eq!(result.pinned, expected);
            assert_eq!(result.counts[""], 5 - expected.len());
            assert!(result.errors.is_empty());
            let missing = scan_overview(dir.path(), &settings(), &filters, None).unwrap();
            assert!(missing.hidden.is_empty() && missing.pinned.is_empty() && missing.errors.is_empty());
        }
        let filters = Filters { hide: vec![filter(FilterKind::Tag, FilterTarget::Files, "[", PatternType::Regex)], pin: vec![] };
        assert!(scan_overview(dir.path(), &settings(), &filters, None).unwrap().errors.is_empty());
        assert_eq!(scan_overview(dir.path(), &settings(), &filters, Some(&connection)).unwrap().errors.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_neither_followed_nor_counted() {
        let dir = fixture();
        std::os::unix::fs::symlink(dir.path().join("A"), dir.path().join("linked-folder")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("root.md"), dir.path().join("linked.md")).unwrap();
        let filters = Filters { hide: vec![], pin: vec![path_filter(FilterTarget::Both, "*", PatternType::Wildcard)] };
        let result = scan_overview(dir.path(), &settings(), &filters, None).unwrap();
        assert_eq!(result.counts[""], 5);
        assert!(!result.counts.contains_key("linked-folder"));
        assert!(!result.pinned.iter().any(|path| path.starts_with("linked")));
        assert!(!result.pinned.iter().any(|path| path.split('/').any(|part| part.starts_with('.'))));
    }

    #[test]
    fn overview_reads_saved_settings_and_filters_without_starting_an_index() {
        let dir = fixture();
        let app_data = tempfile::tempdir().unwrap();
        let core = Core::open(app_data.path(), std::sync::Arc::new(|_| {}));
        let mut config = core.settings.get_config();
        config.plugins = settings();
        config.plugins.folder_counts.show_all_files = true;
        core.settings.update_config(config).unwrap();
        let mut data = super::super::vault_data::VaultPluginData::default();
        data.filters.hide.push(path_filter(FilterTarget::Folders, "A", PatternType::Strict));
        data.filters.pin.push(filter(FilterKind::Tag, FilterTarget::Files, "project", PatternType::Strict));
        super::super::vault_data::save(dir.path(), &data).unwrap();
        let result = overview(&core, dir.path()).unwrap();
        assert_eq!(result.counts[""], 2);
        assert_eq!(result.hidden, ["A"]);
        assert!(result.pinned.is_empty() && result.errors.is_empty());
        assert!(core.search.active_root().is_none());
        core.shutdown();
    }

    #[test]
    #[ignore]
    fn explorer_overview_ten_thousand_files_under_100_ms() {
        use std::io::Write;
        use std::time::{Duration, Instant};
        assert!(!cfg!(debug_assertions), "запускайте бенч в release");
        let dir = tempfile::tempdir().unwrap();
        for folder in 0..100 {
            let path = dir.path().join(format!("folder-{folder:02}"));
            fs::create_dir(&path).unwrap();
            for file in 0..100 {
                fs::write(path.join(format!("note-{file:02}.md")), "").unwrap();
            }
        }
        let app_data = tempfile::tempdir().unwrap();
        let core = Core::open(app_data.path(), std::sync::Arc::new(|_| {}));
        let mut config = core.settings.get_config();
        config.plugins = settings();
        core.settings.update_config(config).unwrap();
        let mut data = super::super::vault_data::VaultPluginData::default();
        data.filters.hide = vec![
            path_filter(FilterTarget::Folders, "^folder-00$", PatternType::Regex),
            path_filter(FilterTarget::Files, "folder-01/note-01.md", PatternType::Strict),
        ];
        data.filters.pin = vec![path_filter(FilterTarget::Files, "*0.md", PatternType::Wildcard)];
        super::super::vault_data::save(dir.path(), &data).unwrap();
        overview(&core, dir.path()).unwrap();
        let start = Instant::now();
        let result = overview(&core, dir.path()).unwrap();
        let elapsed = start.elapsed();
        writeln!(std::io::stdout(), "explorer overview: 10000 файлов, {elapsed:?}").unwrap();
        assert_eq!(result.counts[""], 9899);
        assert_eq!(result.counts.len(), 101);
        assert_eq!(result.hidden.len(), 2);
        assert_eq!(result.pinned.len(), 1000);
        assert!(elapsed < Duration::from_millis(100), "overview: {elapsed:?}");
        core.shutdown();
    }

    #[test]
    fn missing_root_reports_io_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(scan_overview(&dir.path().join("missing"), &settings(), &Filters::default(), None), Err(PluginError::Io { .. })));
    }
}
