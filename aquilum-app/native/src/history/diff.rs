use aquilum_core::documents::merge::line_diff::{line_diff, split_lines};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Same,
    Added,
    Removed,
}

pub fn diff(previous: Option<&str>, text: &str) -> Vec<(Change, String)> {
    let next = split_lines(text);
    let Some(previous) = previous else {
        return next.into_iter().map(|line| (Change::Same, line.to_owned())).collect();
    };
    let base = split_lines(previous);
    let hunks = line_diff(&base, &next).hunks;
    let mut lines = Vec::new();
    let mut at = 0;
    for hunk in hunks {
        lines.extend(base[at..hunk.from].iter().map(|line| (Change::Same, (*line).to_owned())));
        lines.extend(base[hunk.from..hunk.to].iter().map(|line| (Change::Removed, (*line).to_owned())));
        lines.extend(hunk.lines.into_iter().map(|line| (Change::Added, line)));
        at = hunk.to;
    }
    lines.extend(base[at..].iter().map(|line| (Change::Same, (*line).to_owned())));
    lines
}

pub fn hunk_lines(hunks: &[aquilum_core::history::changes::Hunk]) -> Vec<(Change, String)> {
    let mut lines = Vec::new();
    for (index, hunk) in hunks.iter().enumerate() {
        if index > 0 {
            lines.push((Change::Same, "⋯".to_owned()));
        }
        let (head, lead) = hunk.before.rsplit_once('\n').unwrap_or(("", &hunk.before));
        let (trail, tail) = hunk.after.split_once('\n').unwrap_or((&hunk.after, ""));
        let same = |text: &str| text.split('\n').map(|line| (Change::Same, line.to_owned())).collect::<Vec<_>>();
        if hunk.before.contains('\n') {
            lines.extend(same(head));
        }
        lines.extend(format!("{lead}{}{trail}", hunk.removed).split('\n').map(|line| (Change::Removed, line.to_owned())));
        lines.extend(format!("{lead}{}{trail}", hunk.added).split('\n').map(|line| (Change::Added, line.to_owned())));
        if !tail.is_empty() {
            lines.extend(same(tail.strip_suffix('\n').unwrap_or(tail)));
        }
    }
    lines
}
