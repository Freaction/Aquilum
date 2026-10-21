use crate::search::note_date::to_civil_at;
use crate::text::content::{find_byte, read_bytes, Content};
use crate::text::pieces::Pieces;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictCause {
    Diverged,
    Displaced,
    Reverted,
}

struct Explanation {
    title: &'static str,
    why: &'static str,
}

fn explanation(cause: ConflictCause) -> Explanation {
    match cause {
        ConflictCause::Diverged => Explanation {
            title: "файл и документ изменились одновременно",
            why: "И файл на диске, и открытый в редакторе документ изменились после последней сверки. \
                  Свести их без потери было нельзя, поэтому в заметке осталась версия с диска, \
                  а версия из редактора сохранена здесь.",
        },
        ConflictCause::Displaced => Explanation {
            title: "строки не поместились при слиянии",
            why: "В заметку приехала внешняя правка, и её сведение с тем, что набиралось в редакторе, \
                  вытеснило строки ниже. Остальной текст заметки цел.",
        },
        ConflictCause::Reverted => Explanation {
            title: "изменения версии отменены поверх более поздних правок",
            why: "Отмена изменений одной версии из истории задела строки, которые правились позже. \
                  В заметке эти места вернулись к виду до той версии, а более поздний текст этих строк \
                  сохранён здесь.",
        },
    }
}

pub struct ConflictReport {
    pub cause: ConflictCause,
    pub synced_hash: Option<String>,
}

pub struct LocalMinute {
    pub date: String,
    pub time: String,
}

impl LocalMinute {
    pub fn at(unix_nanos: i64, offset_seconds: i64) -> Self {
        let (year, month, day, inside) = to_civil_at(unix_nanos, offset_seconds);
        Self {
            date: format!("{year:04}-{month:02}-{day:02}"),
            time: format!("{:02}:{:02}", inside / 3600, (inside % 3600) / 60),
        }
    }

    pub fn now(offset_seconds: i64) -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos() as i64);
        Self::at(nanos, offset_seconds)
    }
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn copy_stem(stem: &str, moment: &LocalMinute, device: &str) -> String {
    format!("{stem} (конфликт {} {} {device})", moment.date, moment.time.replace(':', "-"))
}

pub fn copy_content<'a>(
    note_path: &str,
    stem: &str,
    report: &ConflictReport,
    moment: &LocalMinute,
    body: &'a dyn Content,
) -> Option<Pieces<Box<dyn Content + 'a>>> {
    let len = body.byte_len();
    find_byte(body, 0..len, false, |byte| !byte.is_ascii_whitespace()).ok()??;
    let explanation = explanation(report.cause);
    let stamp = format!("{} {}", moment.date, moment.time);
    let mut lines = vec![
        "---".to_owned(),
        format!("конфликт: {stamp}"),
        format!("заметка: {}", quoted(stem)),
        format!("причина: {}", quoted(explanation.title)),
        "---".to_owned(),
        String::new(),
        format!("> Это конфликтная копия. Текст ниже не попал в заметку «{stem}» — он сохранён здесь, чтобы не пропасть."),
        ">".to_owned(),
        format!("> **Почему.** {}", explanation.why),
        ">".to_owned(),
        format!("> **Когда.** {stamp}"),
        format!("> **Заметка.** `{note_path}`"),
    ];
    if let Some(hash) = &report.synced_hash {
        lines.push(format!("> **Отпечаток последней сверки.** `{}`", one_line(hash)));
    }
    lines.extend([String::new(), "## Текст, который не попал в заметку".to_owned(), String::new()]);
    let head = lines.join("\n");
    let ends = read_bytes(body, len - 1..len).ok()? == b"\n";
    let (head_len, tail) = (head.len(), if ends { "" } else { "\n" });
    let mut copy: Pieces<Box<dyn Content + 'a>> = Pieces::new(vec![Box::new(head), Box::new(body), Box::new(tail)]);
    copy.push(0, 0..head_len);
    copy.push(1, 0..len);
    copy.push(2, 0..tail.len());
    Some(copy)
}

#[cfg(test)]
#[path = "conflict_tests.rs"]
mod tests;
