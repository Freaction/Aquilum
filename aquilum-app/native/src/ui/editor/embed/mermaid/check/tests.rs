use super::measure;

const CASES: &str = include_str!("../../../../../../testdata/mermaid-cases.md");
const NOTE: &str = include_str!("../../../../../../testdata/markdown-test.md");

fn blocks(text: &str) -> Vec<String> {
    text.split("```mermaid").skip(1).filter_map(|b| b.find("```").map(|end| b[..end].trim().to_owned())).collect()
}

#[test]
fn every_case_lays_out_without_overlaps() {
    let mut report = String::new();
    let (mut faults, mut measured) = (0, 0);
    for (i, body) in blocks(CASES).iter().chain(&blocks(NOTE)).enumerate() {
        let Some(m) = measure(body) else { continue };
        measured += 1;
        let head = body.lines().next().unwrap_or_default();
        report.push_str(&format!("#{i:02} {head:<20} пересечений {:>2}, изгибов {:>2}\n", m.crossings, m.bends));
        for fault in &m.faults {
            report.push_str(&format!("    {fault}\n"));
        }
        faults += m.faults.len();
    }
    println!("{report}");
    assert!(measured > 40, "{measured}");
    assert_eq!(faults, 0, "\n{report}");
}
