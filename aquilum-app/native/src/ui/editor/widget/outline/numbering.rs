use super::item::Item;
use super::*;

fn listed(doc: &Document, i: usize) -> bool {
    let p = doc.paragraph(i);
    matches!(p.block, Block::Item(_) | Block::Continuation { .. } | Block::Code { .. }) || doc.slice(p.range()).trim().is_empty()
}

pub(in super::super) fn renumbered(doc: &Document, edited: Range<usize>) -> Vec<(Range<usize>, String)> {
    let (mut i, last) = (doc.at_offset(edited.start), doc.at_offset(edited.end));
    while i > 0 && listed(doc, i - 1) {
        i -= 1;
    }
    let mut counters: Vec<Option<u64>> = Vec::new();
    let mut out = Vec::new();
    while i < doc.line_count() && (i <= last || listed(doc, i)) {
        let p = doc.paragraph(i);
        match &p.block {
            Block::Item(item) => {
                let level = item.level();
                let previous = counters.get(level).copied().flatten();
                counters.resize(level, None);
                let number = item.ordered.then(|| Item::parse(doc.slice(p.range())).and_then(|found| found.number())).flatten();
                counters.push(number.map(|(digits, number)| {
                    let expected = previous.map_or(if level == 0 { number } else { 1 }, |previous| previous + 1);
                    if number != expected {
                        out.push((p.start + digits.start..p.start + digits.end, expected.to_string()));
                    }
                    expected
                }));
            }
            Block::Continuation { level, .. } => counters.truncate(level + 1),
            _ if listed(doc, i) => {}
            _ => counters.clear(),
        }
        i += 1;
    }
    out
}
