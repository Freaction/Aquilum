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
    if bytes < 1024 * 1024 * 1024 {
        return t_with("settings.trash.megabytes", &[("value", &format_locale(bytes as f64 / (1024.0 * 1024.0), 1))]);
    }
    t_with("settings.trash.gigabytes", &[("value", &format_locale(bytes as f64 / (1024.0 * 1024.0 * 1024.0), 2))])
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
    let clearable = state.as_ref().is_some_and(|state| state.bytes > 0);
    let description = match state {
        Some(state) if state.count > 0 => format!("{}, {}", plural("settings.trash.files", state.count), format_size(state.bytes)),
        _ => t("settings.trash.emptyHint"),
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
                Control::Buttons(
                    crate::system::DESKTOP
                        .then(|| Button::new(t("settings.trash.open"), Action::OpenTrash).enabled(ctx.workspace.is_some()))
                        .into_iter()
                        .chain([Button::new(t("settings.trash.clear"), Action::Wipe(Wipe::Trash)).enabled(clearable)])
                        .collect(),
                ),
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
