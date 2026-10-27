use super::*;
use crate::updater::Status;

pub(super) const AUTHOR_URL: &str = "https://t.me/dmitriy_yiu";
const REPOSITORY_URL: &str = "https://github.com/Freaction/Aquilum";

pub fn update_row(status: &Status) -> Row {
    let description = match status {
        Status::Idle => t("settings.system.updatesIdle"),
        Status::Checking => t("settings.system.updatesChecking"),
        Status::Latest => t("settings.system.updatesLatest"),
        Status::Available(release) => t_with("settings.system.updatesAvailable", &[("version", &release.version)]),
        Status::Downloading { version, .. } | Status::Ready(version) => t_with("settings.system.updatesInstalling", &[("version", version)]),
        Status::Installing => t("update.installing"),
        Status::Failed(message) => t_with("settings.system.updatesFailed", &[("message", message)]),
        Status::ManualRequired { version, .. } => t_with("settings.system.updatesManualRequired", &[("version", version)]),
    };
    let button = match status {
        Status::Available(_) => Button::new(t("settings.system.installUpdate"), Action::InstallUpdate),
        Status::Checking | Status::Downloading { .. } | Status::Ready(_) | Status::Installing => Button::new(t("settings.system.checkUpdates"), Action::CheckUpdates).enabled(false),
        Status::ManualRequired { url, .. } => Button::new(t("settings.system.downloadManual"), Action::OpenUrl(url.clone())),
        _ => Button::new(t("settings.system.checkUpdates"), Action::CheckUpdates),
    };
    info(t("settings.system.manualUpdate"), Some(description), Control::Buttons(vec![button]))
}

pub(super) fn system(config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let updates = match &ctx.update {
        Some(status) => {
            let auto = (!crate::updater::ASKS).then(|| row(t("settings.system.autoUpdate"), Some(t("settings.system.autoUpdateHint")), Control::Switch(config.updates.auto), |c, v| c.updates.auto = flag(v)));
            let manual = (crate::updater::ASKS || !config.updates.auto || matches!(status, Status::ManualRequired { .. })).then(|| update_row(status));
            auto.into_iter().chain(manual).collect()
        }
        None => {
            let reason = t(if cfg!(feature = "installed") { "settings.system.updatesByPackage" } else { "settings.system.updatesUnavailable" });
            vec![info(t("settings.system.autoUpdate"), Some(reason), Control::Locked(false))]
        }
    };
    vec![
        Block { title: t("settings.system.updates"), rows: updates },
        Block {
            title: t("settings.system.autostart"),
            rows: vec![match ctx.autostart {
                Some(on) => bound(t("settings.system.launch"), None, Control::Switch(on), Setter::Autostart),
                None => info(t("settings.system.launch"), Some(t("settings.system.launchUnavailable")), Control::Locked(false)),
            }],
        },
        Block {
            title: t("settings.system.about"),
            rows: vec![
                info(
                    t("settings.system.author"),
                    Some(t("settings.system.authorHint")),
                    Control::Buttons(vec![Button::new(AUTHOR_URL.to_owned(), Action::OpenUrl(AUTHOR_URL.to_owned()))]),
                ),
                info(
                    t_with("settings.system.version", &[("version", env!("CARGO_PKG_VERSION"))]),
                    Some(t("settings.system.versionHint")),
                    Control::Buttons(vec![Button::new(t("settings.system.repository"), Action::OpenUrl(REPOSITORY_URL.to_owned()))]),
                ),
            ],
        },
    ]
}
