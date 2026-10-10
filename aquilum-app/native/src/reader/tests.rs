use std::path::PathBuf;

use serde_json::Value;

use super::{Book, Point};

fn after(book: &Book, index: usize, point: Point) -> String {
    let Point::At(node, offset) = point else { return String::new() };
    let Some(text) = book.document(index).and_then(|d| d.text_of(node)) else { return String::new() };
    let units: Vec<u16> = text.encode_utf16().collect();
    let from = (offset as usize).min(units.len());
    String::from_utf16_lossy(&units[from..(from + 24).min(units.len())])
}

fn prefix(text: &str, units: usize) -> String {
    let all: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16_lossy(&all[..units.min(all.len())])
}

fn text_point(point: Point) -> Option<(u32, u32)> {
    match point {
        Point::At(node, offset) => Some((node, offset)),
        _ => None,
    }
}

#[test]
#[ignore]
fn matches_foliate_golden() {
    let golden: Value = serde_json::from_str(&std::fs::read_to_string(std::env::var("AQ_READER_GOLDEN").unwrap()).unwrap()).unwrap();
    let vault = PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let mut failures = Vec::new();
    let mut checked = 0;
    for entry in golden.as_array().unwrap() {
        let file = entry["file"].as_str().unwrap();
        let started = std::time::Instant::now();
        let Some(book) = Book::open(&vault.join(file)) else {
            failures.push(format!("{file}: не открылась"));
            continue;
        };
        let sections = entry["sections"].as_array().unwrap();
        if sections.len() != book.sections.len() {
            failures.push(format!("{file}: глав {} вместо {}", book.sections.len(), sections.len()));
            continue;
        }
        for (index, section) in sections.iter().enumerate() {
            if book.sections[index].base != section["base"].as_str().unwrap() {
                failures.push(format!("{file} #{index}: база {} вместо {}", book.sections[index].base, section["base"]));
            }
            if book.document(index).is_none() {
                failures.push(format!("{file} #{index}: глава не разобрана"));
                continue;
            }
            for point in section["points"].as_array().unwrap() {
                checked += 1;
                let cfi = point["cfi"].as_str().unwrap();
                let Some((i, start, _)) = book.resolve(cfi) else {
                    failures.push(format!("{file}: {cfi} не разрешился"));
                    continue;
                };
                let got = after(&book, i, start);
                if got != point["after"].as_str().unwrap() {
                    failures.push(format!("{file}: {cfi} → {got:?} вместо {:?}", point["after"]));
                    continue;
                }
                let back = text_point(start).and_then(|p| book.cfi(i, p, p));
                if back.as_deref() != Some(cfi) {
                    failures.push(format!("{file}: из места собран {back:?} вместо {cfi}"));
                }
            }
            for range in section["ranges"].as_array().unwrap() {
                checked += 1;
                let cfi = range["cfi"].as_str().unwrap();
                let Some((i, start, end)) = book.resolve(cfi) else {
                    failures.push(format!("{file}: {cfi} не разрешился"));
                    continue;
                };
                let text = book.text(i, start, end).unwrap_or_default();
                let length = text.encode_utf16().count() as u64;
                if length != range["length"].as_u64().unwrap() || prefix(&text, 48) != range["text"].as_str().unwrap() {
                    failures.push(format!("{file}: {cfi} → {:?} ({length}) вместо {:?} ({})", prefix(&text, 48), range["text"], range["length"]));
                    continue;
                }
                let back = text_point(start).zip(text_point(end)).and_then(|(a, b)| book.cfi(i, a, b));
                if back.as_deref() != Some(cfi) {
                    failures.push(format!("{file}: из выделения собран {back:?} вместо {cfi}"));
                }
            }
        }
        for saved in entry["saved"].as_array().unwrap() {
            checked += 1;
            let cfi = saved["cfi"].as_str().unwrap();
            let Some((i, start, end)) = book.resolve(cfi) else {
                failures.push(format!("{file}: сохранённая {cfi} не разрешилась"));
                continue;
            };
            let text = book.text(i, start, end).unwrap_or_default();
            if i as u64 != saved["index"].as_u64().unwrap() || prefix(&text, 60) != saved["text"].as_str().unwrap() || text.encode_utf16().count() as u64 != saved["length"].as_u64().unwrap() {
                failures.push(format!("{file}: сохранённая {cfi} → #{i} {:?} вместо #{} {:?}", prefix(&text, 60), saved["index"], saved["text"]));
            }
        }
        eprintln!("{file}: глав {}, {:?}", book.sections.len(), started.elapsed());
    }
    for failure in failures.iter().take(40) {
        eprintln!("{failure}");
    }
    assert!(failures.is_empty(), "расхождений {} из {checked}", failures.len());
    eprintln!("проверено {checked}, всё совпало");
}
