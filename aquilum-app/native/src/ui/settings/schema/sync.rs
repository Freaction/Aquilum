use super::*;
use aquilum_core::settings::models::SyncSettings;
use aquilum_core::sync::{Large, Progress, Stage};

const INTERVALS: [u64; 5] = [0, 30, 60, 300, 600];

fn button(key: &str, action: Action) -> Button {
    Button::new(t(&format!("settings.sync.{key}")), action)
}

fn vault_row(key: &str, control: Control, setter: fn(&mut SyncSettings, &Value)) -> Row {
    bound(t(&format!("settings.sync.{key}")), None, control, Setter::VaultSync(setter))
}

fn set_interval(sync: &mut SyncSettings, value: &Value) {
    let seconds = INTERVALS[index(value).min(INTERVALS.len() - 1)];
    sync.enabled = seconds > 0;
    if seconds > 0 {
        sync.interval = seconds;
    }
}

fn clock(key: &str, seconds: u64) -> String {
    t_with(key, &[("time", &format!("{}:{:02}", seconds / 60, seconds % 60))])
}

fn next_sync(seconds: u64) -> String {
    clock("settings.sync.next", seconds)
}

fn offline_retry(seconds: u64) -> String {
    clock("settings.sync.offlineRetry", seconds)
}

fn intervals(vault: SyncSettings) -> Control {
    let options = INTERVALS.iter().map(|seconds| t(&format!("settings.sync.every.{seconds}"))).collect();
    let selected = if vault.enabled { position(&INTERVALS, &vault.interval) } else { 0 };
    Control::Dropdown { options, selected }
}

fn large_row(large: &Large) -> Row {
    let path = &large.path;
    let send = large.kept.then(|| button("send", Action::SyncSend(vec![path.clone()])).icon(ButtonIcon::Upload));
    let reveal = crate::system::DESKTOP.then(|| button("reveal", Action::Reveal(path.clone())).icon(ButtonIcon::Folder));
    let trash = button("toTrash", Action::Trash(path.clone())).icon(ButtonIcon::Trash);
    let place = t(if large.kept { "settings.sync.onDevice" } else { "settings.sync.inCloud" });
    info(path.clone(), Some(format!("{} · {place}", format_size(large.size))), Control::Buttons(send.into_iter().chain(reveal).chain([trash]).collect()))
}

fn status(ctx: &Context) -> String {
    match &ctx.sync {
        SyncStatus::Off(None) => t("settings.sync.off"),
        SyncStatus::Off(Some(error)) | SyncStatus::Failed(error) => error.clone(),
        SyncStatus::Login => t("settings.sync.login"),
        SyncStatus::Deletions { planned, synced } => t_with("settings.sync.deletions", &[("planned", &planned.to_string()), ("synced", &synced.to_string())]),
        SyncStatus::Offline => t("settings.sync.offline"),
        SyncStatus::Unsynced { files, first } => t_with("settings.sync.unsynced", &[("files", &plural("settings.trash.files", *files as u64)), ("error", first)]),
        SyncStatus::Running(_) | SyncStatus::Ready => match ctx.sync_last {
            Some(at) => t_with("settings.sync.done", &[("time", &crate::dates::date_time(at))]),
            None => t("settings.sync.connected"),
        },
    }
}

pub fn stage(progress: &Progress) -> (String, usize, usize) {
    let (key, count) = match progress.stage() {
        Stage::Download(count) => ("settings.sync.downloading", count),
        Stage::Upload(count) => ("settings.sync.uploading", count),
        Stage::Other(count) => ("settings.sync.syncing", count),
    };
    (t(key), count.done, count.total)
}

fn running(progress: &Progress) -> String {
    match stage(progress) {
        (_, _, 0) => t("settings.sync.running"),
        (stage, done, total) => t_with("settings.sync.progress", &[("stage", &stage), ("done", &done.to_string()), ("total", &total.to_string())]),
    }
}

fn provider(ctx: &Context, enabled: bool) -> Control {
    let disconnect = || button("disconnect", Action::SyncDisconnect);
    let indicator = match ctx.sync {
        SyncStatus::Off(_) => return Control::Buttons(vec![button("connect", Action::SyncConnect)]),
        SyncStatus::Login => return Control::Buttons(vec![button("cancel", Action::SyncDisconnect)]),
        SyncStatus::Deletions { .. } => return Control::Buttons(vec![button("allowDeletions", Action::SyncAllowDeletions), disconnect()]),
        SyncStatus::Running(_) => Some(Indicator::Syncing),
        SyncStatus::Ready => ctx.sync_last.map(|_| Indicator::Synced),
        SyncStatus::Offline => None,
        SyncStatus::Unsynced { .. } | SyncStatus::Failed(_) => Some(Indicator::Failed),
    };
    match indicator.filter(|_| enabled) {
        Some(indicator) => Control::Indicated(vec![disconnect()], indicator),
        None => Control::Buttons(vec![disconnect()]),
    }
}

fn files(ctx: &Context) -> Option<Block> {
    if ctx.sync_large.is_empty() {
        return None;
    }
    let kept: Vec<&Large> = ctx.sync_large.iter().filter(|large| large.kept).collect();
    let mut rows = Vec::new();
    if kept.len() > 1 {
        let bytes = format_size(kept.iter().map(|large| large.size).sum());
        let send = button("sendAllButton", Action::SyncSend(kept.iter().map(|large| large.path.clone()).collect()));
        rows.push(info(t("settings.sync.sendAll"), Some(t_with("settings.sync.sendAllHint", &[("size", &bytes)])), Control::Buttons(vec![send])));
    }
    rows.extend(ctx.sync_large.iter().map(large_row));
    Some(Block { title: t("settings.sync.files"), rows })
}

pub(super) fn sync(_config: &AppConfig, ctx: &Context) -> Vec<Block> {
    let enabled = ctx.vault_sync.is_some_and(|vault| vault.enabled);
    let mut dropbox = info(t("settings.sync.dropbox"), Some(status(ctx)), provider(ctx, enabled));
    if ctx.sync == SyncStatus::Offline && let Some(due) = ctx.sync_due {
        dropbox.description = None;
        dropbox.countdown = Some((due, offline_retry as fn(u64) -> String));
    }
    let mut blocks = vec![Block { title: t("settings.sync.providers"), rows: vec![dropbox] }];
    let connected = !matches!(ctx.sync, SyncStatus::Off(_) | SyncStatus::Login);
    let Some(vault) = ctx.vault_sync.filter(|_| connected) else { return blocks };
    let mut rows = Vec::new();
    if vault.enabled && ctx.sync_size.synced > 0 {
        let files = plural("settings.trash.files", ctx.sync_size.files);
        let text = t_with("settings.sync.sizeValue", &[("files", &files), ("size", &format_size(ctx.sync_size.synced))]);
        rows.push(info(t("settings.sync.size"), None, Control::Status { text, tone: Tone::Muted }));
    }
    let mut schedule = vault_row("enabled", intervals(vault), set_interval);
    schedule.countdown = ctx.sync_due.map(|at| (at, next_sync as fn(u64) -> String));
    if vault.enabled && let SyncStatus::Running(progress) = &ctx.sync {
        schedule.description = Some(running(progress));
    }
    rows.push(schedule);
    if vault.enabled {
        rows.push(vault_row("trash", Control::Switch(vault.trash), |s, v| s.trash = flag(v)));
        rows.push(vault_row("history", Control::Switch(vault.history), |s, v| s.history = flag(v)));
    }
    blocks.push(Block { title: t("settings.sync.settings"), rows });
    blocks.extend(files(ctx));
    blocks
}
