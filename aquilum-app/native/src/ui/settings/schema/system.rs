use super::*;

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
