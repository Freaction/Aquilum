use super::error::PluginError;
use std::cmp::Ordering;
use unicode_width::UnicodeWidthStr;

fn invalid(message: &str) -> PluginError {
    PluginError::Invalid { message: message.to_owned() }
}

struct Table {
    rows: Vec<Vec<String>>,
    aligns: Vec<(bool, bool)>,
    metadata: Option<String>,
    newline: &'static str,
    trailing_newline: bool,
}

fn has_closing_ticks(text: &[u8], count: usize) -> bool {
    let mut pos = 0;
    while pos < text.len() {
        if text[pos] == b'`' {
            let run = text[pos..].iter().take_while(|byte| **byte == b'`').count();
            if run == count { return true; }
            pos += run;
        } else { pos += 1; }
    }
    false
}

fn split_row(line: &str) -> Vec<String> {
    let line = line.trim();
    let bytes = line.as_bytes();
    let mut cells = Vec::new();
    let mut start = 0;
    let mut pos = 0;
    let mut ticks = 0;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' if ticks == 0 => pos += 2,
            b'`' => {
                let run = bytes[pos..].iter().take_while(|byte| **byte == b'`').count();
                if ticks == run { ticks = 0; }
                else if ticks == 0 && has_closing_ticks(&bytes[pos + run..], run) { ticks = run; }
                pos += run;
            }
            b'|' if ticks == 0 => {
                if pos != 0 { cells.push(line[start..pos].trim().to_owned()); }
                start = pos + 1;
                pos += 1;
            }
            _ => pos += 1,
        }
    }
    if start < line.len() { cells.push(line[start..].trim().to_owned()); }
    cells
}

impl Table {
    fn parse(md: &str) -> Result<Self, PluginError> {
        let mut lines: Vec<_> = md.lines().collect();
        let metadata = if lines.last().is_some_and(|line| line.trim().starts_with("<!--q-table:")) {
            Some(lines.pop().unwrap().to_owned())
        } else { None };
        if lines.len() < 2 || !lines[0].contains('|') { return Err(invalid("Ожидалась Markdown-таблица")); }
        let header = split_row(lines[0]);
        let separator = split_row(lines[1]);
        if header.is_empty() || separator.len() != header.len() { return Err(invalid("Число столбцов не совпадает")); }
        let mut aligns = Vec::new();
        for cell in separator {
            let left = cell.starts_with(':');
            let right = cell.ends_with(':');
            let dashes = cell.strip_prefix(':').unwrap_or(&cell);
            let dashes = dashes.strip_suffix(':').unwrap_or(dashes);
            if dashes.is_empty() || !dashes.bytes().all(|byte| byte == b'-') {
                return Err(invalid("Некорректная строка-разделитель"));
            }
            aligns.push((left, right));
        }
        let mut rows = vec![header];
        for line in &lines[2..] {
            let cells = split_row(line);
            if cells.len() != rows[0].len() { return Err(invalid("Число столбцов не совпадает")); }
            rows.push(cells);
        }
        Ok(Self { rows, aligns, metadata, newline: if md.contains("\r\n") { "\r\n" } else { "\n" }, trailing_newline: md.ends_with('\n') })
    }

    fn render(&self) -> String {
        let widths: Vec<_> = self.aligns.iter().enumerate().map(|(col, (left, right))| {
            self.rows.iter().map(|row| UnicodeWidthStr::width(row[col].as_str())).max().unwrap_or(0)
                .max(3 + usize::from(*left) + usize::from(*right))
        }).collect();
        let mut lines = Vec::new();
        for (index, row) in self.rows.iter().enumerate() {
            let cells: Vec<_> = row.iter().enumerate().map(|(col, cell)| {
                format!("{}{}", cell, " ".repeat(widths[col] - UnicodeWidthStr::width(cell.as_str())))
            }).collect();
            lines.push(format!("| {} |", cells.join(" | ")));
            if index == 0 {
                let separator: Vec<_> = self.aligns.iter().enumerate().map(|(col, (left, right))| {
                    format!("{}{}{}", if *left { ":" } else { "" }, "-".repeat(widths[col] - usize::from(*left) - usize::from(*right)), if *right { ":" } else { "" })
                }).collect();
                lines.push(format!("| {} |", separator.join(" | ")));
            }
        }
        if let Some(metadata) = &self.metadata { lines.push(metadata.clone()); }
        let mut result = lines.join(self.newline);
        if self.trailing_newline { result.push_str(self.newline); }
        result
    }
}

pub fn format_table(md: &str) -> Result<String, PluginError> {
    Ok(Table::parse(md)?.render())
}

pub fn sort_table(md: &str, column: usize, descending: bool) -> Result<String, PluginError> {
    let mut table = Table::parse(md)?;
    if column >= table.aligns.len() { return Err(invalid("Столбец вне таблицы")); }
    if table.rows.iter().flatten().any(|cell| cell == "^^") {
        return Err(invalid("Сортировка вертикально объединённых ячеек недоступна"));
    }
    let mut metadata = table.metadata.as_ref().map(|line| {
        let json = line.trim().strip_prefix("<!--q-table:").and_then(|text| text.strip_suffix("-->"))
            .ok_or_else(|| invalid("Некорректные метаданные таблицы"))?;
        serde_json::from_str::<serde_json::Value>(json).map_err(|_| invalid("Некорректные метаданные таблицы"))
    }).transpose()?;
    if let Some(meta) = metadata.as_ref() {
        if !meta.is_object() { return Err(invalid("Некорректные метаданные таблицы")); }
        if let Some(merges) = meta.get("merges") {
            let merges = merges.as_array().ok_or_else(|| invalid("Некорректные объединения"))?;
            for merge in merges {
                let values = merge.as_array().filter(|values| values.len() == 4).ok_or_else(|| invalid("Некорректные объединения"))?;
                let coords: Vec<_> = values.iter().map(|value| value.as_u64().map(|value| value as usize).ok_or_else(|| invalid("Некорректные объединения"))).collect::<Result<_, _>>()?;
                if coords[0] != coords[2] { return Err(invalid("Сортировка вертикально объединённых ячеек недоступна")); }
                if coords[0] >= table.rows.len() || coords[1] > coords[3] || coords[3] >= table.aligns.len() {
                    return Err(invalid("Некорректные объединения"));
                }
            }
        }
        if let Some(rows) = meta.get("rows") {
            if rows.as_array().is_none_or(|rows| rows.len() != table.rows.len()) { return Err(invalid("Некорректные высоты строк")); }
        }
    }
    let keys: Vec<_> = table.rows.iter().map(|row| if row[column] == "<!--q-empty-->" { "" } else { row[column].as_str() }).collect();
    let numbers: Vec<_> = keys.iter().map(|value| {
        value.chars().filter(|ch| !ch.is_whitespace()).collect::<String>().replace(',', ".")
            .parse::<f64>().ok()
    }).collect();
    let numeric = keys.iter().enumerate().skip(1).all(|(index, value)| value.is_empty() || numbers[index].is_some());
    let lower: Vec<_> = keys.iter().map(|key| key.to_lowercase()).collect();
    let mut order: Vec<_> = (1..table.rows.len()).collect();
    order.sort_by(|&a, &b| {
        // Пустые ячейки остаются в конце при обоих направлениях сортировки.
        match (keys[a].is_empty(), keys[b].is_empty()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => {
                let cmp = if numeric {
                    match (numbers[a], numbers[b]) {
                        (Some(a), Some(b)) => a.total_cmp(&b),
                        _ => Ordering::Equal,
                    }
                } else { lower[a].cmp(&lower[b]) };
                if descending { cmp.reverse() } else { cmp }
            }
        }
    });
    order.insert(0, 0);
    if let Some(meta) = metadata.as_mut() {
        if let Some(rows) = meta.get_mut("rows") {
            let original = rows.as_array().unwrap().clone();
            *rows = order.iter().map(|&index| original[index].clone()).collect();
        }
        if let Some(merges) = meta.get_mut("merges").and_then(|value| value.as_array_mut()) {
            for merge in merges {
                let row = merge[0].as_u64().unwrap() as usize;
                let mapped = order.iter().position(|&index| index == row).unwrap();
                merge[0] = mapped.into();
                merge[2] = mapped.into();
            }
        }
        table.metadata = Some(format!("<!--q-table:{}-->", meta));
    }
    table.rows = order.iter().map(|&index| table.rows[index].clone()).collect();
    Ok(table.render())
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn formats_cjk_emoji_and_preserves_alignment() {
        let md = "| 名 | icon | n |\n| :-- | :-: | --: |\n| 東京 | 👩‍💻 | 12 |\n| a | 🙂 | 3 |";
        let actual = format_table(md).unwrap();
        let lines: Vec<_> = actual.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[1], "| :--- | :---: | ---: |");
        let widths: Vec<_> = lines.iter().map(|line| UnicodeWidthStr::width(*line)).collect();
        assert!(widths.iter().all(|width| *width == widths[0]));
        assert!(actual.contains("東京") && actual.contains("👩‍💻") && actual.contains("🙂"));
        assert_eq!(format_table(&actual).unwrap(), actual);
    }

    #[test]
    fn keeps_escaped_pipes_and_backtick_code_in_their_cells() {
        let md = "| h | v |\n| --- | --- |\n| a\\|b | `a|b` |\n| ``a|`b`` | c |";
        let actual = format_table(md).unwrap();
        assert!(actual.contains("a\\|b"));
        assert!(actual.contains("`a|b`"));
        assert!(actual.contains("``a|`b``"));
        assert_eq!(format_table(&actual).unwrap(), actual);
        let backslash = "| h | v |\n| --- | --- |\n| `a\\` | b |";
        assert!(format_table(backslash).unwrap().contains("`a\\`"));
    }

    #[test]
    fn numeric_sort_accepts_decimal_commas_and_spaces_and_keeps_header() {
        let md = "| n | name |\n| --: | --- |\n| 10 | ten |\n| 1,5 | small |\n| 1 000 | thousand |\n| | empty |\n| 1.5 | equal |";
        let ascending = sort_table(md, 0, false).unwrap();
        assert_eq!(ascending.lines().skip(2).map(|line| line.split('|').nth(2).unwrap().trim()).collect::<Vec<_>>(), ["small", "equal", "ten", "thousand", "empty"]);
        let descending = sort_table(md, 0, true).unwrap();
        assert_eq!(descending.lines().skip(2).map(|line| line.split('|').nth(2).unwrap().trim()).collect::<Vec<_>>(), ["thousand", "ten", "small", "equal", "empty"]);
        assert!(ascending.starts_with("| n"));
        assert!(ascending.lines().nth(1).unwrap().contains(':'));
    }

    #[test]
    fn sorts_strings_with_unicode_lowercase() {
        let md = "| word |\n| --- |\n| Яблоко |\n| банан |\n| Абрикос |";
        let actual = sort_table(md, 0, false).unwrap();
        assert_eq!(actual.lines().skip(2).map(|line| line.trim_matches('|').trim()).collect::<Vec<_>>(), ["Абрикос", "банан", "Яблоко"]);
    }

    #[test]
    fn rejects_invalid_input_and_out_of_range_columns() {
        for md in ["", "not a table", "| a | b |\n| --- |", "| a |\n| nope |\n| b |", "| a |\n| --- |\n| b | c |"] {
            assert!(matches!(format_table(md), Err(PluginError::Invalid { .. })), "{md}");
        }
        assert!(matches!(sort_table("| a |\n| --- |\n| b |", 1, false), Err(PluginError::Invalid { .. })));
    }

    #[test]
    fn vertical_merges_format_but_cannot_sort() {
        for md in [
            "| a | b |\n| --- | --- |\n| z | x |\n| ^^ | y |",
            "| a | b |\n| --- | --- |\n| z | x |\n| | y |\n<!--q-table:{\"merges\":[[1,0,2,0]]}-->",
        ] {
            let formatted = format_table(md).unwrap();
            assert_eq!(format_table(&formatted).unwrap(), formatted);
            assert!(matches!(sort_table(md, 1, false), Err(PluginError::Invalid { .. })));
        }
    }

    #[test]
    fn sorting_moves_horizontal_merges_and_row_heights() {
        let md = "| a | b | c |\n| --- | --- | --- |\n| z | merged | |\n| a | x | y |\n<!--q-table:{\"merges\":[[1,1,1,2]],\"rows\":[30,40,50],\"cols\":[100,120,140]}-->";
        let actual = sort_table(md, 0, false).unwrap();
        let metadata = actual.lines().last().unwrap().strip_prefix("<!--q-table:").unwrap().strip_suffix("-->").unwrap();
        let value: serde_json::Value = serde_json::from_str(metadata).unwrap();
        assert_eq!(value["merges"], serde_json::json!([[2,1,2,2]]));
        assert_eq!(value["rows"], serde_json::json!([30,50,40]));
        assert_eq!(value["cols"], serde_json::json!([100,120,140]));
    }

    #[test]
    fn preserves_line_endings_and_table_metadata() {
        let md = "| a |\r\n| --- |\r\n| b |\r\n<!--q-table:{\"cols\":[180]}-->\r\n";
        let actual = format_table(md).unwrap();
        assert!(actual.ends_with("<!--q-table:{\"cols\":[180]}-->\r\n"));
        assert_eq!(actual.replace("\r\n", "").matches('\n').count(), 0);
    }
}
