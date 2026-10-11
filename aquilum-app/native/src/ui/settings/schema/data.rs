use super::*;

pub(super) fn search(config: &AppConfig) -> Vec<Block> {
    let s = &config.search;
    vec![Block {
        title: t("settings.search.section"),
        rows: vec![
            row(
                t("settings.search.candidatePool"),
                None,
                Control::Number(Number::new(s.candidate_pool_size as f64, 32.0, 2048.0, 32.0)),
                |c, v| c.search.candidate_pool_size = number(v) as usize,
            ),
            row(
                t("settings.search.maxQueryTerms"),
                None,
                Control::Number(Number::new(s.max_query_terms as f64, 8.0, 128.0, 1.0)),
                |c, v| c.search.max_query_terms = number(v) as usize,
            ),
        ],
    }]
}

pub(super) fn files(config: &AppConfig) -> Vec<Block> {
    vec![Block {
        title: t("settings.files.section"),
        rows: vec![row(
            t("settings.files.folder"),
            Some(t("settings.files.folderHint")),
            Control::Text { value: config.files.folder.clone(), commit: false },
            |c, v| c.files.folder = text(v),
        )],
    }]
}

pub(super) fn analysis(config: &AppConfig) -> Vec<Block> {
    let a = &config.analysis;
    let methods = Block {
        title: t("settings.analysis.methods"),
        rows: vec![
            row(t("settings.analysis.bm25f"), None, Control::Switch(a.enable_bm25f), |c, v| c.analysis.enable_bm25f = flag(v)),
            row(t("settings.analysis.adamicAdar"), None, Control::Switch(a.enable_adamic_adar), |c, v| {
                c.analysis.enable_adamic_adar = flag(v);
            }),
            row(t("settings.analysis.wiki"), None, Control::Switch(a.enable_wikixiv), |c, v| c.analysis.enable_wikixiv = flag(v)),
        ],
    };
    if !a.enable_bm25f {
        return vec![methods];
    }
    let p = &a.bm25f_params;
    let param = |label: String, value: f32, min, max, step, set: fn(&mut AppConfig, &Value)| {
        row(label, None, Control::Number(Number::new(round2(f64::from(value)), min, max, step)), set)
    };
    let params = Block {
        title: t("settings.analysis.bm25fParams"),
        rows: vec![
            param("k1".into(), p.k1, 0.1, 3.0, 0.1, |c, v| c.analysis.bm25f_params.k1 = number(v) as f32),
            param("k3".into(), p.k3, 1.0, 20.0, 0.5, |c, v| c.analysis.bm25f_params.k3 = number(v) as f32),
            param(t("settings.analysis.bTitle"), p.b_title, 0.0, 1.0, 0.05, |c, v| c.analysis.bm25f_params.b_title = number(v) as f32),
            param(t("settings.analysis.bBody"), p.b_body, 0.0, 1.0, 0.05, |c, v| c.analysis.bm25f_params.b_body = number(v) as f32),
            param(t("settings.analysis.titleWeight"), p.title_weight, 0.5, 10.0, 0.1, |c, v| {
                c.analysis.bm25f_params.title_weight = number(v) as f32;
            }),
        ],
    };
    vec![methods, params]
}

pub(super) fn history(config: &AppConfig) -> Vec<Block> {
    let options = vec![
        t("settings.history.forever"),
        t_with("settings.history.days", &[("count", "30")]),
        t_with("settings.history.days", &[("count", "90")]),
        t("settings.history.year"),
    ];
    vec![Block {
        title: t("settings.history.section"),
        rows: vec![row(
            t("settings.history.retention"),
            Some(t("settings.history.retentionHint")),
            Control::Dropdown { options, selected: position(&HISTORY_DAYS, &config.history.retention_days) },
            |c, v| c.history.retention_days = HISTORY_DAYS[index(v).min(3)],
        )],
    }]
}

pub(super) fn shortcuts() -> Vec<Block> {
    const GROUPS: &[(&str, &[(&str, &str)])] = &[
        (
            "global",
            &[
                ("newTab", "Ctrl + N"),
                ("newFromTemplate", "Ctrl + U"),
                ("globalSearch", "Ctrl + O"),
                ("focusMode", "Ctrl + Shift + F"),
                ("pageSearch", "Ctrl + F"),
                ("zoomIn", "Ctrl + +"),
                ("zoomOut", "Ctrl + -"),
                ("zoomReset", "Ctrl + 0"),
            ],
        ),
        (
            "editor",
            &[
                ("bold", "Ctrl + B"),
                ("italic", "Ctrl + I"),
                ("strikethrough", "Ctrl + Shift + S"),
                ("indent", "Tab"),
                ("outdent", "Shift + Tab"),
                ("lineBreak", "Shift + ↵"),
            ],
        ),
        (
            "searchWindow",
            &[
                ("searchNext", "↓"),
                ("searchPrevious", "↑"),
                ("searchOpen", "↵"),
                ("searchOpenNewPane", "Ctrl + ↵"),
                ("searchCreate", "Shift + ↵"),
            ],
        ),
        ("pageSearchWindow", &[("searchNext", "↵"), ("searchPrevious", "Shift + ↵")]),
        ("reader", &[("nextPage", "→"), ("previousPage", "←")]),
    ];
    GROUPS
        .iter()
        .map(|(group, items)| Block {
            title: t(&format!("settings.shortcuts.{group}")),
            rows: items
                .iter()
                .map(|(label, keys)| Row {
                    label: t(&format!("settings.shortcuts.{label}")),
                    description: None,
                    control: Control::Kbd((*keys).to_owned()),
                    setter: None,
                    code: None,
                })
                .collect(),
        })
        .collect()
}

pub(super) const DEFAULT_TEMPLATES_FOLDER: &str = "Templates";

pub fn templates_path(workspace: &Path, folder: &str) -> PathBuf {
    let value = folder.trim();
    let drive = value.len() > 2 && value.as_bytes()[0].is_ascii_alphabetic() && value.as_bytes()[1] == b':';
    let absolute = value.starts_with('/') || value.starts_with("\\\\") || drive && value[2..].starts_with(['/', '\\']);
    if absolute {
        return PathBuf::from(value.trim_end_matches(['/', '\\']));
    }
    let normalized = value.trim_matches(['/', '\\']);
    workspace.join(if normalized.is_empty() { DEFAULT_TEMPLATES_FOLDER } else { normalized })
}

pub(super) fn templates(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let mut rows = vec![row(
        t("settings.templates.folder"),
        Some(t("settings.templates.folderHint")),
        Control::Text { value: config.templates.folder.clone(), commit: false },
        |c, v| c.templates.folder = text(v),
    )];
    if let Some(workspace) = &ctx.workspace
        && !templates_path(workspace, &config.templates.folder).is_dir()
    {
        let folder = config.templates.folder.trim();
        let folder = if folder.is_empty() { DEFAULT_TEMPLATES_FOLDER } else { folder };
        rows.push(info(
            t("settings.templates.create"),
            Some(t_with("settings.templates.createHint", &[("folder", folder)])),
            Control::Buttons(vec![Button::new(t("settings.templates.createButton"), Action::CreateTemplates)]),
        ));
    }
    vec![Block { title: t("settings.templates.section"), rows }]
}
