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
