use super::*;

pub(super) const TRASH_DAYS: [u32; 5] = [0, 7, 30, 90, 365];
pub(super) const TRASH_PAGE_SIZE: usize = 20;

pub(super) fn format_locale(value: f64, digits: usize) -> String {
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

pub(super) fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        return t_with("settings.trash.bytes", &[("value", &format_locale(bytes as f64, 0))]);
    }
    if bytes < 1024 * 1024 {
        return t_with("settings.trash.kilobytes", &[("value", &format_locale((bytes as f64 / 1024.0).round(), 0))]);
    }
    t_with("settings.trash.megabytes", &[("value", &format_locale(bytes as f64 / (1024.0 * 1024.0), 1))])
}


pub(super) fn deletion_row(deletion: TrashedDeletion) -> Row {
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

pub(super) fn trash(config: &AppConfig, ctx: &Context) -> Vec<Block> {
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

pub(super) const AUTHOR_URL: &str = "https://t.me/dmitriy_yiu";

pub(super) fn update_row(status: &crate::updater::Status) -> Row {
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

pub(super) fn system(config: &AppConfig, ctx: &Context) -> Vec<Block> {
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
