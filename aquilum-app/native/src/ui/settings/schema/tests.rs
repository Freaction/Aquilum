use super::*;
use aquilum_core::settings::models::SyncSettings;

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

#[test]
fn font_lists_start_with_the_bundled_families_and_go_on_with_installed_ones() {
    let list = families(&EDITOR_FAMILIES);
    assert_eq!(list[..3], ["iA Writer Mono", "iA Writer Quattro", "Inter"]);
    assert_eq!(family(&EDITOR_FAMILIES, &Value::Index(1)), "iA Writer Quattro");
    if let Some(installed) = list.get(3) {
        assert_eq!(&family(&EDITOR_FAMILIES, &Value::Index(3)), installed);
    }
    assert_eq!(family(&EDITOR_FAMILIES, &Value::Index(10_000)), "iA Writer Mono");
}

#[test]
fn templates_and_attachments_share_one_section_and_search_tuning_lives_with_bm25f() {
    let config = AppConfig::default();
    let ctx = Context::new(1.0);
    let titles: Vec<String> = blocks(Section::Data, &config, &ctx).into_iter().map(|b| b.title).collect();
    assert_eq!(titles, [t("settings.templates.section"), t("settings.files.section")]);
    for old in ["templates", "files", "data"] {
        assert_eq!(Section::from_key(old), Some(Section::Data), "сохранённый раздел «{old}» открывает общий раздел");
    }
    assert_eq!(Section::from_key("search"), Some(Section::Analysis));
    assert_eq!(Section::ALL.len(), 11);

    let rows = |config: &AppConfig| blocks(Section::Analysis, config, &ctx).into_iter().flat_map(|b| b.rows).map(|r| r.label).collect::<Vec<_>>();
    let pool = t("settings.search.candidatePool");
    assert!(rows(&config).contains(&pool), "настройки подбора похожих заметок стоят среди параметров BM25F");
    let mut off = AppConfig::default();
    off.analysis.enable_bm25f = false;
    assert!(!rows(&off).contains(&pool), "без BM25F этих настроек нет");
}

#[test]
fn sync_is_turned_on_per_vault_after_connecting() {
    let config = AppConfig::default();
    let mut ctx = Context::new(1.0);
    ctx.workspace = Some(PathBuf::from("База"));
    ctx.vault_sync = Some(SyncSettings::default());
    let labels = |ctx: &Context| blocks(Section::Sync, &config, ctx).into_iter().flat_map(|b| b.rows).map(|r| r.label).collect::<Vec<_>>();
    assert!(!labels(&ctx).contains(&t("settings.sync.enabled")), "до входа настройки базы не показываются");

    ctx.sync = SyncStatus::Ready;
    assert!(labels(&ctx).contains(&t("settings.sync.enabled")));
    assert!(!labels(&ctx).contains(&t("settings.sync.trash")), "новая база по умолчанию не синхронизируется");

    ctx.vault_sync = Some(SyncSettings { enabled: true, ..SyncSettings::default() });
    assert!(labels(&ctx).contains(&t("settings.sync.trash")) && labels(&ctx).contains(&t("settings.sync.history")));

    ctx.sync_due = Some(std::time::Instant::now() + std::time::Duration::from_secs(90));
    let countdown = |ctx: &Context| {
        let rows = blocks(Section::Sync, &config, ctx).into_iter().flat_map(|b| b.rows);
        rows.filter(|r| r.label == t("settings.sync.enabled")).find_map(|r| r.countdown).map(|(_, text)| text(90))
    };
    assert_eq!(countdown(&ctx), Some(t_with("settings.sync.next", &[("time", "1:30")])));

    ctx.sync = SyncStatus::Running(aquilum_core::sync::Progress::default());
    let rows = blocks(Section::Sync, &config, &ctx).into_iter().flat_map(|b| b.rows);
    let description = rows.filter(|r| r.label == t("settings.sync.enabled")).find_map(|r| r.description);
    assert_eq!(description, Some(t("settings.sync.running")), "во время прохода вместо отсчёта — «Синхронизация…»");
}
