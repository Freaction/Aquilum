use super::model::Row;

pub const MAX_PINS: usize = 5;

pub fn pins(rows: &[Row], line: f64, step: f64) -> Vec<usize> {
    let mut found = Vec::new();
    if line <= 0.0 {
        return found;
    }
    for slot in 0..MAX_PINS {
        let at = slot as f64 * step;
        let under = ((line + at) / step) as usize;
        let Some(row) = rows.get(under) else { break };
        let top = under as f64 * step - line;
        let index = if row.depth > slot {
            rows[..under].iter().rposition(|r| r.depth <= slot).expect("у вложенной строки есть родитель")
        } else if row.depth == slot && row.folder && row.expanded && top < at {
            under
        } else {
            break;
        };
        found.push(index);
    }
    found
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const STEP: f64 = 10.0;

    fn row(name: &str, depth: usize, folder: bool) -> Row {
        Row { path: PathBuf::from(name), name: name.into(), folder, depth, expanded: folder, color: None, items: None, guides: Vec::new(), active: false, selected: false }
    }

    fn rows() -> Vec<Row> {
        vec![
            row("A", 0, true),
            row("a1", 1, false),
            row("F", 1, true),
            row("f1", 2, false),
            row("f2", 2, false),
            row("a2", 1, false),
            row("B", 0, true),
            row("b1", 1, false),
            row("b2", 1, false),
        ]
    }

    fn at(line: f64) -> Vec<usize> {
        pins(&rows(), line, STEP)
    }

    #[test]
    fn parent_sticks_as_soon_as_its_row_starts_to_leave() {
        assert!(at(0.0).is_empty());
        assert_eq!(at(5.0), [0]);
        assert_eq!(at(10.0), [0]);
    }

    #[test]
    fn nested_folder_sticks_under_its_parent() {
        assert_eq!(at(15.0), [0, 2]);
        assert_eq!(at(35.0), [0, 2]);
        assert_eq!(at(40.0), [0]);
    }

    #[test]
    fn next_folder_replaces_the_previous_one() {
        assert_eq!(at(55.0), [0]);
        assert!(at(60.0).is_empty());
        assert_eq!(at(61.0), [6]);
    }
}
