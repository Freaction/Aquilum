use aquilum_core::history::Source;
use chrono::Datelike;

use super::Version;
use crate::dates;
use crate::i18n::{t, t_with};

fn source_name(source: &Source) -> String {
    let key = match source {
        Source::Me => "me",
        Source::Agent => "agent",
        Source::External => "external",
        Source::Links => "links",
        Source::Start | Source::Reading | Source::Synced => "start",
        Source::Restore(_) => "restore",
        Source::Revert(_) => "revert",
    };
    t(&format!("history.sources.{key}"))
}

pub fn label(version: &Version) -> String {
    version.name.clone().unwrap_or_else(|| source_name(&version.source))
}

fn title(version: &Version) -> String {
    match version.from_ms {
        None => source_name(&version.source),
        Some(from) => {
            let key = if matches!(version.source, Source::Revert(_)) { "history.revertedFrom" } else { "history.restoredFrom" };
            t_with(key, &[("date", &dates::date_time(from))])
        }
    }
}

pub fn heading(version: &Version) -> String {
    let named = version.name.as_ref().map(|n| format!("{n} · ")).unwrap_or_default();
    format!("{named}{} · {}", title(version), dates::date_time(version.at_ms))
}

fn day_label(at_ms: u64, now_ms: u64) -> String {
    let (Some(at), Some(now)) = (dates::local(at_ms), dates::local(now_ms)) else { return String::new() };
    let (day, today) = (at.date_naive(), now.date_naive());
    if day == today {
        t("history.today")
    } else if today.pred_opt() == Some(day) {
        t("history.yesterday")
    } else {
        dates::day(at_ms, at.year() != now.year())
    }
}

pub fn group_by_day(versions: &[Version], now_ms: u64) -> Vec<(String, Vec<&Version>)> {
    let mut groups: Vec<(chrono::NaiveDate, String, Vec<&Version>)> = Vec::new();
    for version in versions {
        let Some(at) = dates::local(version.at_ms) else { continue };
        let date = at.date_naive();
        match groups.last_mut() {
            Some((day, _, list)) if *day == date => list.push(version),
            _ => groups.push((date, day_label(version.at_ms, now_ms), vec![version])),
        }
    }
    groups.into_iter().map(|(_, label, list)| (label, list)).collect()
}
