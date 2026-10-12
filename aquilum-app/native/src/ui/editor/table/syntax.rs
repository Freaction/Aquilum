use super::model::Align;

pub(super) const META: &str = "<!--q-table:";

pub fn is_meta(line: &str) -> bool {
    line.trim_start().starts_with(META)
}

pub(super) fn cells(line: &str) -> Option<Vec<String>> {
    if is_meta(line) {
        return None;
    }
    let line = line.trim();
    let inner = line.strip_prefix('|').unwrap_or(line);
    let inner = match inner.strip_suffix('|') {
        Some(body) if !body.ends_with('\\') => body,
        _ => inner,
    };
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut previous = ' ';
    for c in inner.chars() {
        if c == '|' && previous != '\\' {
            out.push(std::mem::take(&mut cell));
        } else {
            cell.push(c);
        }
        previous = c;
    }
    if out.is_empty() && !line.starts_with('|') {
        return None;
    }
    out.push(cell);
    Some(out)
}

pub(super) fn shape(cell: &str, keep_tail: bool) -> String {
    let lead = cell.trim_start();
    if keep_tail { lead.strip_suffix(' ').unwrap_or(lead).to_owned() } else { lead.trim_end().to_owned() }
}

pub(super) fn split_row(line: &str) -> Option<Vec<String>> {
    cells(line).map(|row| row.iter().map(|c| shape(c, false)).collect())
}

pub fn is_row(line: &str) -> bool {
    split_row(line).is_some()
}

fn separator_cell(cell: &str) -> bool {
    let c = cell.trim().trim_start_matches(':').trim_end_matches(':');
    !c.is_empty() && c.bytes().all(|b| b == b'-')
}

pub(super) fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty() && cells.iter().all(|c| separator_cell(c))
}

pub fn is_separator(line: &str) -> bool {
    split_row(line).is_some_and(|cells| is_separator_row(&cells))
}

pub(super) fn align_of(cell: &str) -> Option<Align> {
    let v = cell.trim();
    match (v.starts_with(':'), v.ends_with(':')) {
        (true, true) => Some(Align::Center),
        (true, false) => Some(Align::Left),
        (false, true) => Some(Align::Right),
        _ => None,
    }
}

pub(super) fn escape(value: &str) -> String {
    value.replace('|', "\\|")
}

pub(super) fn unescape(cell: &str) -> String {
    cell.replace("\\|", "|")
}
