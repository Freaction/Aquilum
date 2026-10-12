use super::item::Item;
use super::*;
use aquilum_core::markdown::line;

fn listed(line: &str) -> bool {
    line.trim().is_empty() || Item::parse(line).is_some()
}

fn counter(stack: &mut Vec<(usize, Option<u64>)>, columns: usize) -> &mut Option<u64> {
    while stack.last().is_some_and(|(deeper, _)| *deeper > columns) {
        stack.pop();
    }
    if stack.last().is_none_or(|(shallower, _)| *shallower < columns) {
        stack.push((columns, None));
    }
    let last = stack.len() - 1;
    &mut stack[last].1
}

pub(in super::super) fn renumber(text: &str, around: Range<usize>) -> Option<(Range<usize>, String)> {
    let line_start = |at: usize| line::bounds(text, at).start;
    let line_end = |at: usize| line::bounds(text, at).end;
    let mut from = line_start(around.start);
    while from > 0 && listed(&text[line_start(from - 1)..from - 1]) {
        from = line_start(from - 1);
    }
    let mut to = line_end(around.end);
    while to < text.len() && listed(&text[to + 1..line_end(to + 1)]) {
        to = line_end(to + 1);
    }
    let mut counters: Vec<(usize, Option<u64>)> = Vec::new();
    let mut changes: Vec<(Range<usize>, String)> = Vec::new();
    let mut start = from;
    for line in text[from..to].split('\n') {
        let at = start;
        start += line.len() + 1;
        let Some(item) = Item::parse(line) else {
            if !line.trim().is_empty() {
                counters.clear();
            }
            continue;
        };
        let slot = counter(&mut counters, item.columns);
        let Some(number) = item.number else {
            *slot = None;
            continue;
        };
        let expected = slot.map_or(1, |n| n + 1);
        *slot = Some(expected);
        if number != expected {
            changes.push((at + item.marker.start..at + item.marker.end - 2, expected.to_string()));
        }
    }
    let (first, last) = (changes.first()?.0.start, changes.last()?.0.end);
    let mut out = String::new();
    let mut cursor = first;
    for (range, number) in &changes {
        out.push_str(&text[cursor..range.start]);
        out.push_str(number);
        cursor = range.end;
    }
    Some((first..last, out))
}
