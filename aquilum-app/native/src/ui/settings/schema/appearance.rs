use super::*;
use crate::ui::keymap;

pub(super) fn ui(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let scale = ctx.scale;
    let scale_row = Row {
        label: t("settings.ui.scale"),
        description: Some(t_with("settings.ui.scaleHint", &[("reset", &keymap::label(keymap::Scope::Global, keymap::Command::ZoomReset))])),
        control: Control::Number(Number {
            value: (scale * 100.0).round(),
            min: MIN_SCALE * 100.0,
            max: MAX_SCALE * 100.0,
            step: (SCALE_STEP * 100.0).round(),
            stepper: true,
            commit: false,
            blank: false,
        }),
        setter: Some(Setter::Scale),
        code: None,
    };
    let theme = row(
        t("settings.ui.theme"),
        None,
        Control::Segmented {
            options: vec![t("theme.system"), t("theme.light"), t("theme.dark")],
            selected: position(&THEMES, &config.ui.theme.as_str()),
        },
        |c, v| c.ui.theme = THEMES[index(v).min(2)].to_owned(),
    );
    let language = row(
        t("settings.ui.language"),
        None,
        Control::Dropdown { options: vec!["Русский".into(), "English".into()], selected: language_index(&config.ui.language) },
        |c, v| c.ui.language = LANGUAGES[index(v).min(1)].to_owned(),
    );
    let fonts = font_rows(
        &config.ui.font,
        &families(&UI_FAMILIES),
        |c, v| c.ui.font.font_family = family(&UI_FAMILIES, v),
        |c, v| c.ui.font.font_weight = WEIGHTS[index(v).min(4)],
        |c, v| c.ui.font.font_size_base = font_size(v, 14),
    );
    let mut display = vec![scale_row, theme, language];
    if ctx.workspace.is_some() {
        display.push(Row {
            label: t("settings.ui.homePage"),
            description: Some(t("settings.ui.homePageHint")),
            control: Control::Text { value: ctx.home_page.clone(), commit: false },
            setter: Some(Setter::HomePage),
            code: None,
        });
    }
    display.push(row(t("settings.ui.primaryColor"), None, Control::Color(config.ui.primary_color.clone()), |c, v| {
        if let Some([r, g, b]) = crate::ui::theme::parse_hex(&text(v)) {
            c.ui.primary_color = format!("#{r:02x}{g:02x}{b:02x}");
        }
    }));
    display.push(row(t("settings.ui.animations"), Some(t("settings.ui.animationsHint")), Control::Switch(config.ui.animations), |c, v| c.ui.animations = flag(v)));
    vec![
        Block { title: t("settings.ui.display"), rows: display },
        Block { title: t("settings.font.section"), rows: fonts },
    ]
}

pub(super) fn editor(config: &AppConfig) -> Vec<Block> {
    let e = &config.editor;
    let mut fonts = font_rows(
        &e.font,
        &families(&EDITOR_FAMILIES),
        |c, v| c.editor.font.font_family = family(&EDITOR_FAMILIES, v),
        |c, v| c.editor.font.font_weight = WEIGHTS[index(v).min(4)],
        |c, v| c.editor.font.font_size_base = font_size(v, 16),
    );
    fonts.push(row(
        t("settings.editor.lineHeight"),
        None,
        Control::Number(Number::new(f64::from(e.line_height), 1.2, 2.0, 0.05)),
        |c, v| c.editor.line_height = number(v) as f32,
    ));
    let switch = |key: &str, on: bool, set: fn(&mut AppConfig, &Value)| {
        row(t(&format!("settings.editor.{key}")), Some(t(&format!("settings.editor.{key}Hint"))), Control::Switch(on), set)
    };
    vec![
        Block { title: t("settings.font.section"), rows: fonts },
        Block {
            title: t("settings.editor.layout"),
            rows: vec![row(
                t("settings.editor.maxWidth"),
                Some(t("settings.editor.maxWidthHint")),
                Control::Number(Number::new(f64::from(e.max_width_ch), 40.0, 120.0, 1.0)),
                |c, v| c.editor.max_width_ch = number(v) as u32,
            )],
        },
        Block {
            title: t("settings.editor.input"),
            rows: vec![
                switch("smartDashes", e.smart_dashes, |c, v| c.editor.smart_dashes = flag(v)),
                switch("listCallouts", e.list_callouts, |c, v| c.editor.list_callouts = flag(v)),
                switch("coloredTags", e.colored_tags, |c, v| c.editor.colored_tags = flag(v)),
                switch("selectionMatches", e.selection_matches, |c, v| c.editor.selection_matches = flag(v)),
                switch("autoLinkTitle", e.auto_link_title, |c, v| c.editor.auto_link_title = flag(v)),
            ],
        },
        Block {
            title: t("settings.editor.completion"),
            rows: vec![
                switch("linkSuggest", e.link_suggest, |c, v| c.editor.link_suggest = flag(v)),
                row(
                    t("settings.editor.minChars"),
                    Some(t("settings.editor.minCharsHint")),
                    Control::Number(Number::new(f64::from(e.link_suggest_min_chars), 1.0, 5.0, 1.0)),
                    |c, v| c.editor.link_suggest_min_chars = number(v) as u32,
                ),
            ],
        },
        Block {
            title: t("settings.editor.saving"),
            rows: vec![row(
                t("settings.editor.saveDebounce"),
                Some(t("settings.editor.saveDebounceHint")),
                Control::Dropdown { options: SAVE_INTERVALS.map(interval).to_vec(), selected: position(&SAVE_INTERVALS, &e.save_debounce_ms) },
                |c, v| c.editor.save_debounce_ms = SAVE_INTERVALS[index(v).min(SAVE_INTERVALS.len() - 1)],
            )],
        },
    ]
}

pub(super) fn reader(config: &AppConfig) -> Vec<Block> {
    let r = &config.reader;
    let mut fonts = font_rows(
        &r.font,
        &families(&READER_FAMILIES),
        |c, v| c.reader.font.font_family = family(&READER_FAMILIES, v),
        |c, v| c.reader.font.font_weight = WEIGHTS[index(v).min(4)],
        |c, v| c.reader.font.font_size_base = font_size(v, 18),
    );
    fonts.push(row(
        t("settings.reader.lineHeight"),
        None,
        Control::Number(Number::new(f64::from(r.line_height), 1.2, 2.4, 0.05)),
        |c, v| c.reader.line_height = number(v) as f32,
    ));
    let switch = |key: &str, on: bool, set: fn(&mut AppConfig, &Value)| {
        row(t(&format!("settings.reader.{key}")), Some(t(&format!("settings.reader.{key}Hint"))), Control::Switch(on), set)
    };
    vec![
        Block { title: t("settings.font.section"), rows: fonts },
        Block {
            title: t("settings.reader.page"),
            rows: vec![
                row(
                    t("settings.reader.flow"),
                    Some(t("settings.reader.flowHint")),
                    Control::Segmented {
                        options: vec![t("settings.reader.flowPaginated"), t("settings.reader.flowScrolled")],
                        selected: position(&READER_FLOWS, &r.flow.as_str()),
                    },
                    |c, v| c.reader.flow = READER_FLOWS[index(v).min(1)].to_owned(),
                ),
                row(
                    t("settings.reader.maxWidth"),
                    Some(t("settings.reader.maxWidthHint")),
                    Control::Number(Number::new(f64::from(r.max_width_ch), 30.0, 120.0, 1.0)),
                    |c, v| c.reader.max_width_ch = number(v) as u32,
                ),
                row(
                    t("settings.reader.margin"),
                    Some(t("settings.reader.marginHint")),
                    Control::Number(Number::new(f64::from(r.margin_px), 0.0, 160.0, 4.0)),
                    |c, v| c.reader.margin_px = number(v) as u32,
                ),
            ],
        },
        Block {
            title: t("settings.reader.typesetting"),
            rows: vec![
                switch("justify", r.justify, |c, v| c.reader.justify = flag(v)),
                switch("hyphenate", r.hyphenate, |c, v| c.reader.hyphenate = flag(v)),
            ],
        },
    ]
}

pub fn reader_quick(config: &AppConfig) -> Vec<Row> {
    let r = &config.reader;
    vec![
        row(
            t("reader.flow"),
            None,
            Control::Segmented { options: vec![t("reader.flowPaginated"), t("reader.flowScrolled")], selected: position(&READER_FLOWS, &r.flow.as_str()) },
            |c, v| c.reader.flow = READER_FLOWS[index(v).min(1)].to_owned(),
        ),
        row(t("reader.fontSize"), None, Control::Number(Number::font_size(r.font.font_size_base)), |c, v| {
            c.reader.font.font_size_base = font_size(v, 18)
        }),
        row(t("reader.lineHeight"), None, Control::Number(Number::new(f64::from(r.line_height), 1.2, 2.4, 0.05)), |c, v| c.reader.line_height = number(v) as f32),
        row(t("reader.columnWidth"), None, Control::Number(Number::new(f64::from(r.max_width_ch), 30.0, 120.0, 1.0)), |c, v| c.reader.max_width_ch = number(v) as u32),
    ]
}
