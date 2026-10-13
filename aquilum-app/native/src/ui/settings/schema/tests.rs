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
