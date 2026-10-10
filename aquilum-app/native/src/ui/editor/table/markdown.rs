use serde::Deserialize;

use super::model::{Align, DEFAULT_COL, DEFAULT_ROW, MIN_COL, MIN_ROW, Range2, Table};

const META: &str = "<!--q-table:";
const EMPTY: &str = "<!--q-empty-->";
const ESCAPED_EMPTY: &str = "&lt;!--q-empty--&gt;";
const ROWSPAN: &str = "^^";
const ESCAPED_ROWSPAN: &str = "\\^\\^";

#[derive(Default, Deserialize)]
struct Meta {
    merges: Option<Vec<[usize; 4]>>,
    rows: Option<Vec<f64>>,
    cols: Option<Vec<f64>>,
}

fn split_row(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    if !line.starts_with('|') {
        return None;
    }
    let inner = &line[1..];
    let inner = if inner.ends_with('|') && !inner.ends_with("\\|") { &inner[..inner.len() - 1] } else { inner };
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut escaped = false;
    for c in inner.chars() {
        if c == '|' && !escaped {
            out.push(std::mem::take(&mut cell));
        } else {
            cell.push(c);
        }
        escaped = c == '\\' && !escaped;
    }
    out.push(cell);
    Some(out.into_iter().map(|c| c.trim().to_owned()).collect())
}

fn separator_cell(cell: &str) -> bool {
    let c = cell.trim().trim_start_matches(':').trim_end_matches(':');
    !c.is_empty() && c.bytes().all(|b| b == b'-')
}

pub fn is_separator(line: &str) -> bool {
    split_row(line).is_some_and(|cells| !cells.is_empty() && cells.iter().all(|c| separator_cell(c)))
}

pub fn is_meta(line: &str) -> bool {
    line.trim_start().starts_with(META)
}

fn align_of(cell: &str) -> Option<Align> {
    let v = cell.trim();
    match (v.starts_with(':'), v.ends_with(':')) {
        (true, true) => Some(Align::Center),
        (true, false) => Some(Align::Left),
        (false, true) => Some(Align::Right),
        _ => None,
    }
}

fn from_markdown(value: &str) -> String {
    match value {
        EMPTY | ROWSPAN => String::new(),
        ESCAPED_EMPTY => EMPTY.to_owned(),
        ESCAPED_ROWSPAN => ROWSPAN.to_owned(),
        other => other.to_owned(),
    }
}

fn to_markdown(value: &str) -> String {
    match value {
        "" => EMPTY.to_owned(),
        EMPTY => ESCAPED_EMPTY.to_owned(),
        ROWSPAN => ESCAPED_ROWSPAN.to_owned(),
        other => other.to_owned(),
    }
}

fn inferred(raw: &[Vec<String>], columns: usize) -> Vec<Range2> {
    let mut merges: Vec<Range2> = Vec::new();
    for (row, cells) in raw.iter().enumerate() {
        let mut col = 0;
        while col < columns {
            if cells[col] == ROWSPAN || cells[col] == EMPTY {
                col += 1;
                continue;
            }
            let mut right = col;
            while right + 1 < columns && cells[right + 1].is_empty() {
                right += 1;
            }
            if right > col {
                merges.push(Range2 { top: row, left: col, bottom: row, right });
            }
            col = right + 1;
        }
    }
    for row in 1..raw.len() {
        let mut col = 0;
        while col < columns {
            if raw[row][col] != ROWSPAN {
                col += 1;
                continue;
            }
            let mut right = col;
            while right + 1 < columns && raw[row][right + 1] == ROWSPAN {
                right += 1;
            }
            match merges.iter_mut().find(|m| m.bottom == row - 1 && m.left == col && m.right == right) {
                Some(m) => m.bottom = row,
                None => merges.push(Range2 { top: row - 1, left: col, bottom: row, right }),
            }
            col = right + 1;
        }
    }
    merges
}

pub fn parse(lines: &[&str]) -> Option<Table> {
    let header = split_row(lines.first()?)?;
    let separator = split_row(lines.get(1)?)?;
    if !separator.iter().all(|c| separator_cell(c)) {
        return None;
    }
    let columns = header.len().max(separator.len());
    let normalize = |mut row: Vec<String>| {
        row.resize(columns, String::new());
        row
    };
    let mut raw = vec![normalize(header)];
    let mut line = 2;
    while let Some(row) = lines.get(line).and_then(|l| split_row(l)) {
        raw.push(normalize(row));
        line += 1;
    }
    if raw.len() < 2 {
        return None;
    }
    let meta_line = lines.get(line).filter(|l| is_meta(l));
    let meta: Option<Meta> = meta_line.map(|l| l.trim().strip_prefix(META).and_then(|j| j.strip_suffix("-->")).and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default());
    let rows = raw.len();
    let merges = match &meta {
        Some(m) => m.merges.clone().unwrap_or_default().into_iter().map(|[top, left, bottom, right]| Range2 { top, left, bottom, right }).collect(),
        None => inferred(&raw, columns),
    };
    let cells = raw.iter().map(|r| r.iter().map(|c| from_markdown(c)).collect()).collect();
    let heights = (0..rows).map(|r| meta.as_ref().and_then(|m| m.rows.as_ref()).and_then(|v| v.get(r).copied()).filter(|h| *h > 0.0).unwrap_or(DEFAULT_ROW).max(MIN_ROW)).collect();
    let cols = meta.as_ref().and_then(|m| m.cols.clone());
    let widths = (0..columns).map(|c| cols.as_ref().and_then(|v| v.get(c).copied()).filter(|w| *w > 0.0).unwrap_or(DEFAULT_COL).max(MIN_COL)).collect();
    let aligns = (0..columns).map(|c| align_of(separator.get(c).map_or("---", String::as_str))).collect();
    Some(Table { cells, merges, aligns, heights, widths, explicit: cols.is_some() }.normalized())
}

impl Table {
    pub fn serialize(&self) -> String {
        let mut content: Vec<Vec<String>> = self.cells.iter().map(|r| r.iter().map(|c| to_markdown(c)).collect()).collect();
        for m in &self.merges {
            for r in m.top..=m.bottom {
                for c in m.left..=m.right {
                    if (r, c) != (m.top, m.left) {
                        content[r][c] = if r > m.top { ROWSPAN.to_owned() } else { String::new() };
                    }
                }
            }
        }
        let mut lines = Vec::new();
        for (r, row) in content.iter().enumerate() {
            lines.push(format!("| {} |", row.join(" | ")));
            if r == 0 {
                let sep: Vec<&str> = self
                    .aligns
                    .iter()
                    .map(|a| match a {
                        Some(Align::Left) => ":---",
                        Some(Align::Center) => ":---:",
                        Some(Align::Right) => "---:",
                        None => "---",
                    })
                    .collect();
                lines.push(format!("| {} |", sep.join(" | ")));
            }
        }
        let custom_rows = self.heights.iter().any(|h| (*h - DEFAULT_ROW).abs() > f64::EPSILON);
        if !self.merges.is_empty() || custom_rows || self.explicit {
            let merges: Vec<String> = self.merges.iter().map(|m| format!("[{},{},{},{}]", m.top, m.left, m.bottom, m.right)).collect();
            let mut json = format!("{{\"merges\":[{}]", merges.join(","));
            if custom_rows {
                let rows: Vec<String> = self.heights.iter().map(|h| format!("{}", h.round() as i64)).collect();
                json.push_str(&format!(",\"rows\":[{}]", rows.join(",")));
            }
            if self.explicit {
                let cols: Vec<String> = self.widths.iter().map(|w| format!("{}", w.round() as i64)).collect();
                json.push_str(&format!(",\"cols\":[{}]", cols.join(",")));
            }
            json.push('}');
            lines.push(format!("{META}{json}-->"));
        }
        lines.join("\n")
    }
}

