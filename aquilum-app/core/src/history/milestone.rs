use super::store;
use super::delta::{inserted, starts, Region};
use crate::text::content::{chars_below, Content};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

const IDLE: Duration = Duration::from_secs(60);
const CAP: Duration = Duration::from_secs(5 * 60);
const BIG_CHANGE_CHARS: usize = 200;
const COALESCE_BYTES: usize = 1 << 20;
const RESTORE_LABEL: &str = "restore";
const REVERT_LABEL: &str = "revert";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Me,
    Agent,
    External,
    Links,
    Start,
    Reading,
    Synced,
    Restore(u64),
    Revert(u64),
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Self::Me => "me",
            Self::Agent => "agent",
            Self::External => "external",
            Self::Links => "links",
            Self::Start => "start",
            Self::Reading => "reading",
            Self::Synced => "synced",
            Self::Restore(_) => RESTORE_LABEL,
            Self::Revert(_) => REVERT_LABEL,
        }
    }

    pub fn file_label(self) -> String {
        match self.origin_ms() {
            Some(from_ms) => format!("{}-{from_ms:013}", self.label()),
            None => self.label().to_owned(),
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        if let Some((kind, from_ms)) = label.split_once('-') {
            let from_ms = from_ms.parse().ok()?;
            return match kind {
                RESTORE_LABEL => Some(Self::Restore(from_ms)),
                REVERT_LABEL => Some(Self::Revert(from_ms)),
                _ => None,
            };
        }
        [Self::Me, Self::Agent, Self::External, Self::Links, Self::Start]
            .into_iter()
            .find(|source| source.label() == label)
    }

    pub fn origin_ms(self) -> Option<u64> {
        match self {
            Self::Restore(from_ms) | Self::Revert(from_ms) => Some(from_ms),
            _ => None,
        }
    }

    pub fn is_quiet(self) -> bool {
        matches!(self, Self::Reading | Self::Synced)
    }
}

impl Serialize for Source {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.label())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MilestonePlan {
    Quiet,
    Skip,
    RevertToPrevious { remove_id: String },
    UpdateLatest { id: String },
    CreateNew { keep_original: bool },
}

pub struct Incoming {
    pub source: Source,
    pub same_as_latest: bool,
    pub same_as_previous: bool,
    pub original_differs: bool,
    pub small: bool,
    pub fits_latest: bool,
}

pub struct Recorded<'a> {
    pub versions: &'a [store::VersionFile],
    pub names: &'a HashMap<String, String>,
    pub last_edit: Option<SystemTime>,
}

pub fn is_small(text: &dyn Content, regions: &[Region]) -> std::io::Result<bool> {
    let removed: usize = regions.iter().map(|region| region.to - region.from).sum();
    Ok(removed < BIG_CHANGE_CHARS && chars_below(text, regions.iter().zip(starts(regions)).map(|(region, at)| at..at + region.len), BIG_CHANGE_CHARS)?)
}

pub fn fits(latest_inserted: usize, regions: &[Region]) -> bool {
    latest_inserted + inserted(regions) <= COALESCE_BYTES
}

pub fn plan(device: &str, incoming: &Incoming, recorded: &Recorded<'_>, now: SystemTime) -> MilestonePlan {
    if incoming.source.is_quiet() {
        return MilestonePlan::Quiet;
    }
    let latest = recorded.versions.last();
    let mine = |version: &store::VersionFile| {
        incoming.source == Source::Me
            && version.source == Source::Me
            && version.device == device
            && !recorded.names.contains_key(&version.id)
    };
    if let Some(latest) = latest {
        if incoming.same_as_latest {
            return MilestonePlan::Skip;
        }
        if recorded.versions.len() >= 2 && mine(latest) && incoming.same_as_previous {
            return MilestonePlan::RevertToPrevious { remove_id: latest.id.clone() };
        }
    }
    let can_coalesce = latest.is_some_and(|latest| {
        mine(latest)
            && recorded.last_edit.is_some_and(|edit| elapsed(edit, now) < IDLE)
            && elapsed(store::system_time_from_ms(latest.at_ms), now) < CAP
            && incoming.small
            && incoming.fits_latest
    });
    match latest {
        Some(latest) if can_coalesce => MilestonePlan::UpdateLatest { id: latest.id.clone() },
        _ => MilestonePlan::CreateNew { keep_original: latest.is_none() && incoming.original_differs },
    }
}

fn elapsed(from: SystemTime, to: SystemTime) -> Duration {
    to.duration_since(from).unwrap_or_default()
}

#[cfg(test)]
#[path = "milestone_tests.rs"]
mod tests;
