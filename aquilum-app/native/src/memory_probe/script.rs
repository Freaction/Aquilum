use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone)]
pub enum Step {
    OpenNote(PathBuf),
    Same(PathBuf),
    Tab(usize),
    OpenSettings,
    CloseSettings,
    OpenGraph,
    CloseTab,
    Reader(PathBuf),
    CloseReader,
    Scroll(f64),
    Video,
    Wait,
    Mark(String),
    Evict(String),
    Minimize(bool),
}

pub(super) fn parse(line: &str) -> Option<((String, Step), Duration)> {
    let mut words = line.trim().splitn(3, ' ');
    let pause = Duration::from_millis(words.next()?.parse().ok()?);
    let action = words.next()?;
    let arg = words.next().unwrap_or("").to_owned();
    let step = match action {
        "open" => Step::OpenNote(arg.clone().into()),
        "same" => Step::Same(arg.clone().into()),
        "tab" => Step::Tab(arg.parse().ok()?),
        "settings" => Step::OpenSettings,
        "close-settings" => Step::CloseSettings,
        "graph" => Step::OpenGraph,
        "close-tab" => Step::CloseTab,
        "reader" => Step::Reader(arg.clone().into()),
        "close-reader" => Step::CloseReader,
        "scroll" => Step::Scroll(arg.parse().ok()?),
        "video" => Step::Video,
        "wait" => Step::Wait,
        "mark" => Step::Mark(arg.clone()),
        "evict" => Step::Evict(arg.clone()),
        "minimize" => Step::Minimize(true),
        "restore" => Step::Minimize(false),
        _ => return None,
    };
    Some(((arg, step), pause))
}
