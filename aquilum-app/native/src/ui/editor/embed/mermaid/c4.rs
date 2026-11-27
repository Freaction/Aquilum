use mermaid_rs_renderer::{Graph, NodeShape, Subgraph};

use super::canvas::Canvas;
use super::graph::{edge, render};
use super::{split, unquote};

fn stereotype(word: &str) -> String {
    let (base, external) = word.strip_suffix("_Ext").map_or((word, false), |b| (b, true));
    let mut snake = String::new();
    for (i, c) in base.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            snake.push('_');
        }
        snake.extend(c.to_lowercase());
    }
    if external { format!("«external_{snake}»") } else { format!("«{snake}»") }
}

fn element(word: &str) -> Option<bool> {
    let base = word.strip_suffix("_Ext").unwrap_or(word);
    let kind = base.trim_end_matches("Db").trim_end_matches("Queue");
    match kind {
        "Person" | "System" => Some(false),
        "Container" | "Component" => Some(true),
        _ => None,
    }
}

pub(super) fn graph(lines: &[&str]) -> Graph {
    let mut graph = Graph::new();
    let mut stack: Vec<usize> = Vec::new();
    for line in lines.iter().map(|l| l.trim()) {
        if line.starts_with('}') {
            stack.pop();
            continue;
        }
        let Some((word, rest)) = line.split_once('(') else { continue };
        let word = word.trim();
        let inside = rest.rsplit_once(')').map_or(rest, |(a, _)| a);
        let mut args: Vec<&str> = split(inside).into_iter().filter(|a| !a.starts_with('$')).map(unquote).collect();
        if word == "RelIndex" && !args.is_empty() {
            args.remove(0);
        }
        let arg = |k: usize| args.get(k).copied().filter(|a| !a.is_empty());
        let Some(id) = arg(0) else { continue };
        if word.contains("Boundary") || word.starts_with("Deployment_Node") || matches!(word, "Node" | "Node_L" | "Node_R") {
            let label = arg(1).unwrap_or(id);
            let label = arg(2).map_or_else(|| label.to_owned(), |kind| format!("{label} [{kind}]"));
            graph.subgraphs.push(Subgraph { id: Some(id.to_owned()), label, nodes: Vec::new(), direction: None, icon: None });
            if line.ends_with('{') {
                stack.push(graph.subgraphs.len() - 1);
            }
        } else if word.starts_with("Rel") || word.starts_with("BiRel") {
            let Some(to) = arg(1) else { continue };
            let label = match (arg(2), arg(3)) {
                (Some(text), Some(tech)) => Some(format!("{text} [{tech}]")),
                (text, _) => text.map(str::to_owned),
            };
            let (from, to) = if word == "Rel_Back" { (to, id) } else { (id, to) };
            graph.edges.push(edge(from, to, label, (word.starts_with("BiRel"), true)));
        } else if let Some(technical) = element(word) {
            let name = arg(1).unwrap_or(id);
            let (tech, text) = if technical { (arg(2), arg(3)) } else { (None, arg(2)) };
            let mut rows = vec![stereotype(word), name.to_owned()];
            rows.extend(tech.map(|t| format!("[{t}]")));
            if let Some(text) = text {
                rows.extend(["---".to_owned(), text.to_owned()]);
            }
            graph.ensure_node(id, Some(rows.join("\n")), Some(NodeShape::Rectangle));
            for &g in &stack {
                graph.subgraphs[g].nodes.push(id.to_owned());
            }
        }
    }
    graph
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    render(&graph(lines), canvas);
}
