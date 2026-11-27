use mermaid_rs_renderer::{Graph, NodeShape};

use super::canvas::Canvas;
use super::graph::{edge, render};
use super::unquote;

const KINDS: [&str; 7] = ["requirement", "functionalRequirement", "interfaceRequirement", "performanceRequirement", "physicalRequirement", "designConstraint", "element"];

fn relation(line: &str) -> Option<(&str, &str, &str)> {
    if let Some((left, to)) = line.split_once("->") {
        let (from, kind) = left.split_once(" - ")?;
        return Some((from.trim(), kind.trim(), to.trim()));
    }
    let (to, right) = line.split_once("<-")?;
    let (kind, from) = right.split_once(" - ")?;
    Some((from.trim(), kind.trim(), to.trim()))
}

pub(super) fn graph(lines: &[&str]) -> Graph {
    let mut graph = Graph::new();
    let mut open: Option<(&str, Vec<String>)> = None;
    for line in lines.iter().map(|l| l.trim()) {
        if let Some((name, rows)) = open.as_mut() {
            if line.starts_with('}') {
                graph.ensure_node(name, Some(rows.join("\n")), Some(NodeShape::Rectangle));
                open = None;
            } else if let Some((key, value)) = line.split_once(':') {
                rows.push(format!("{}: {}", key.trim(), unquote(value)));
            }
            continue;
        }
        let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        if KINDS.contains(&word) && rest.trim_end().ends_with('{') {
            let name = unquote(rest.trim_end().trim_end_matches('{'));
            open = Some((name, vec![format!("«{word}»"), name.to_owned(), "---".to_owned()]));
        } else if let Some((from, kind, to)) = relation(line) {
            graph.ensure_node(from, None, None);
            graph.ensure_node(to, None, None);
            graph.edges.push(edge(from, to, Some(format!("«{kind}»")), (false, true)));
        }
    }
    graph
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    render(&graph(lines), canvas);
}
