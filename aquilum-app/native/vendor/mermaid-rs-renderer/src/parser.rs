use crate::ir::{DiagramKind, Direction, Graph, NodeStyle, Subgraph};
use anyhow::{Result, bail};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, VecDeque};

type NodeTokenParts = (
    String,
    Option<String>,
    Option<crate::ir::NodeShape>,
    Vec<String>,
);

static HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(flowchart|graph)\s+(\w+)").unwrap());
static SUBGRAPH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^subgraph\s+(.*)$").unwrap());
static PIPE_LABEL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<arrow><[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+[-.=ox]*|[-.=ox]*[-=]+[-.=ox]*>|[-.=ox]*[-=]+[-.=ox]*)\|(?P<label>.+?)\|\s*(?P<right>.+)$",
    )
    .unwrap()
});
static QUOTED_LABEL_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^(?P<left>.+?)\s*(?P<start><)?(?P<dash1>[-.=ox]*[-=]+[-.=ox]*)\s+"(?P<label>[^"]+)"\s+(?P<dash2>[-.=ox]*[-=]+[-.=ox]*)(?P<end>>)?\s*(?P<right>.+)$"#,
    )
    .unwrap()
});
static LABEL_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<start><)?(?P<dash1>[-.=ox]*[-=]+[-.=ox]*)\s+(?P<label>[^<>=]+?)\s+(?P<dash2>[-.=ox]*[-=]+[-.=ox]*)(?P<end>>)?\s*(?P<right>.+)$",
    )
    .unwrap()
});
static COMPACT_DOTTED_LABEL_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<start><)?(?P<dash1>[-=ox]*[-=]+[-=ox]*)\.(?P<label>[^<>=|].*?)\.(?P<dash2>[-.=ox]*[-=]+[-.=ox]*)(?P<end>>)?\s*(?P<right>.+)$",
    )
    .unwrap()
});
static ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<arrow><[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+[-.=ox]*|[-.=ox]*[-=]+[-.=ox]*>|[-.=ox]*[-=]+[-.=ox]*)\s*(?P<right>.+)$",
    )
    .unwrap()
});
static ARROW_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+[-.=ox]*|[-.=ox]*[-=]+[-.=ox]*>|[-.=ox]*[-=]+[-.=ox]*").unwrap()
});

#[derive(Debug, Default)]
pub struct ParseOutput {
    pub graph: Graph,
}

pub fn parse_mermaid(input: &str) -> Result<ParseOutput> {
    let Some(kind) = detect_diagram_kind(input) else {
        bail!("unknown or missing Mermaid diagram header");
    };
    match kind {
        DiagramKind::Class => parse_class_diagram(input),
        DiagramKind::State => parse_state_diagram(input),
        DiagramKind::Sequence => parse_sequence_diagram(input),
        DiagramKind::Er => parse_er_diagram(input),
        DiagramKind::Pie => parse_pie_diagram(input),
        DiagramKind::Mindmap => parse_mindmap_diagram(input),
        DiagramKind::Gantt => parse_gantt_diagram(input),
        DiagramKind::Flowchart => parse_flowchart(input),
        _ => bail!("diagram type is shown as code"),
    }
}


fn detect_diagram_kind(input: &str) -> Option<DiagramKind> {
    let mut in_frontmatter = false;
    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if trimmed_line == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter {
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        if trimmed_line.starts_with("%%{") {
            continue;
        }
        let without_comment = strip_trailing_comment(trimmed_line);
        if without_comment.is_empty() {
            continue;
        }
        let lower = without_comment.to_ascii_lowercase();
        if starts_with_diagram_header(&lower, "sequencediagram") {
            return Some(DiagramKind::Sequence);
        }
        if starts_with_diagram_header(&lower, "classdiagram") {
            return Some(DiagramKind::Class);
        }
        if starts_with_diagram_header(&lower, "statediagram") {
            return Some(DiagramKind::State);
        }
        if starts_with_diagram_header(&lower, "erdiagram") {
            return Some(DiagramKind::Er);
        }
        if starts_with_diagram_header(&lower, "pie") {
            return Some(DiagramKind::Pie);
        }
        if starts_with_diagram_header(&lower, "mindmap") {
            return Some(DiagramKind::Mindmap);
        }
        if starts_with_diagram_header(&lower, "journey") {
            return Some(DiagramKind::Journey);
        }
        if starts_with_diagram_header(&lower, "timeline") {
            return Some(DiagramKind::Timeline);
        }
        if starts_with_diagram_header(&lower, "gantt") {
            return Some(DiagramKind::Gantt);
        }
        if starts_with_diagram_header(&lower, "requirementdiagram") {
            return Some(DiagramKind::Requirement);
        }
        if starts_with_diagram_header(&lower, "gitgraph") {
            return Some(DiagramKind::GitGraph);
        }
        if starts_with_c4_header(&lower) {
            return Some(DiagramKind::C4);
        }
        if starts_with_diagram_header(&lower, "sankey") {
            return Some(DiagramKind::Sankey);
        }
        if starts_with_diagram_header(&lower, "quadrantchart") {
            return Some(DiagramKind::Quadrant);
        }
        if starts_with_diagram_header(&lower, "zenuml") {
            return Some(DiagramKind::ZenUML);
        }
        if starts_with_diagram_header(&lower, "block") {
            return Some(DiagramKind::Block);
        }
        if starts_with_diagram_header(&lower, "packet") {
            return Some(DiagramKind::Packet);
        }
        if starts_with_diagram_header(&lower, "kanban") {
            return Some(DiagramKind::Kanban);
        }
        if starts_with_diagram_header(&lower, "architecture") {
            return Some(DiagramKind::Architecture);
        }
        if starts_with_diagram_header(&lower, "radar") {
            return Some(DiagramKind::Radar);
        }
        if starts_with_diagram_header(&lower, "treemap") {
            return Some(DiagramKind::Treemap);
        }
        if starts_with_diagram_header(&lower, "xychart") {
            return Some(DiagramKind::XYChart);
        }
        if starts_with_diagram_header(&lower, "flowchart")
            || starts_with_diagram_header(&lower, "graph")
        {
            return Some(DiagramKind::Flowchart);
        }
        if looks_like_flowchart_edge_syntax(&without_comment) {
            return Some(DiagramKind::Flowchart);
        }
        return None;
    }
    None
}

fn starts_with_diagram_header(line: &str, keyword: &str) -> bool {
    let Some(rest) = line.strip_prefix(keyword) else {
        return false;
    };
    rest.chars()
        .next()
        .is_none_or(|ch| !ch.is_ascii_alphanumeric())
}

fn starts_with_c4_header(line: &str) -> bool {
    [
        "c4",
        "c4context",
        "c4container",
        "c4component",
        "c4dynamic",
        "c4deployment",
    ]
    .iter()
    .any(|keyword| starts_with_diagram_header(line, keyword))
}


fn looks_like_flowchart_edge_syntax(line: &str) -> bool {
    ARROW_TOKEN_RE.is_match(&mask_bracket_content(line))
}

fn has_missing_flowchart_edge_endpoint(line: &str) -> bool {
    let masked = mask_bracket_content(line);
    let trimmed = masked.trim();
    ARROW_TOKEN_RE
        .find_iter(trimmed)
        .any(|m| trimmed[..m.start()].trim().is_empty() || trimmed[m.end()..].trim().is_empty())
}

fn preprocess_input(input: &str) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let mut in_frontmatter = false;

    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if trimmed_line == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter {
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        let without_comment = strip_trailing_comment(trimmed_line);
        if without_comment.is_empty() {
            continue;
        }
        lines.push(without_comment.to_string());
    }

    Ok(lines)
}

fn preprocess_input_keep_indent(input: &str) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let mut in_frontmatter = false;

    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if trimmed_line == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter {
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        let without_comment = strip_trailing_comment_keep_indent(raw_line);
        if without_comment.trim().is_empty() {
            continue;
        }
        lines.push(without_comment);
    }

    Ok(lines)
}

fn parse_flowchart(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Flowchart;
    let mut subgraph_stack: Vec<usize> = Vec::new();

    let lines = preprocess_input(input)?;

    for raw_line in lines {
        for line in split_statements(&raw_line) {
            if line.is_empty() {
                continue;
            }

            if let Some(caps) = HEADER_RE.captures(&line) {
                if let Some(dir) = caps.get(2).and_then(|m| Direction::from_token(m.as_str())) {
                    graph.direction = dir;
                }
                continue;
            }

            if line == "end" {
                subgraph_stack.pop();
                continue;
            }

            if let Some(caps) = SUBGRAPH_RE.captures(&line) {
                let rest = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let (id, label, classes) = parse_subgraph_header(rest);
                graph.subgraphs.push(Subgraph {
                    id: id.clone(),
                    label,
                    nodes: Vec::new(),
                    direction: None,
                    icon: None,
                });
                subgraph_stack.push(graph.subgraphs.len() - 1);
                if let Some(id) = id {
                    apply_subgraph_classes(&mut graph, &id, &classes);
                }
                continue;
            }

            if let Some(direction) = parse_direction_line(&line) {
                if let Some(idx) = subgraph_stack.last().copied() {
                    if let Some(sub) = graph.subgraphs.get_mut(idx) {
                        sub.direction = Some(direction);
                    }
                } else {
                    graph.direction = direction;
                }
                continue;
            }

            if line.starts_with("classDef") {
                parse_class_def(&line, &mut graph);
                continue;
            }

            if line.starts_with("class ") {
                parse_class_line(&line, &mut graph);
                continue;
            }

            if line.starts_with("style ") {
                parse_style_line(&line, &mut graph);
                continue;
            }

            if line.starts_with("linkStyle") {
                parse_link_style_line(&line, &mut graph);
                continue;
            }

            if let Some((id, link)) = parse_click_line(&line) {
                graph.node_links.insert(id, link);
                continue;
            }

            if line.starts_with("accTitle")
                || line.starts_with("accDescr")
                || line.starts_with("title ")
            {
                continue;
            }

            if has_missing_flowchart_edge_endpoint(&line) {
                bail!("invalid flowchart edge syntax: {line}");
            }

            if let Some(chain_lines) = split_edge_chain(&line) {
                let mut added = false;
                for edge_line in chain_lines {
                    if add_flowchart_edge(&edge_line, &mut graph, &subgraph_stack) {
                        added = true;
                    } else if looks_like_flowchart_edge_syntax(&edge_line) {
                        bail!("invalid flowchart edge syntax: {edge_line}");
                    }
                }
                if added {
                    continue;
                }
            }

            if add_flowchart_edge(&line, &mut graph, &subgraph_stack) {
                continue;
            }

            if looks_like_flowchart_edge_syntax(&line) {
                bail!("invalid flowchart edge syntax: {line}");
            }

            if let Some((node_id, node_label, node_shape, node_classes)) = parse_node_only(&line) {
                graph.ensure_node(&node_id, node_label, node_shape);
                apply_node_classes(&mut graph, &node_id, &node_classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &node_id);
            }
        }
    }

    Ok(ParseOutput { graph })
}

fn add_flowchart_edge(line: &str, graph: &mut Graph, subgraph_stack: &[usize]) -> bool {
    let Some((left, label, right, edge_meta)) = parse_edge_line(line) else {
        return false;
    };

    let sources = split_on_ampersand(&left);
    let targets = split_on_ampersand(&right);

    let mut source_ids = Vec::new();
    for source in sources {
        let (left_id, left_label, left_shape, left_classes) = parse_node_token(source);
        graph.ensure_node(&left_id, left_label, left_shape);
        apply_node_classes(graph, &left_id, &left_classes);
        add_node_to_subgraphs(graph, subgraph_stack, &left_id);
        source_ids.push(left_id);
    }

    let mut target_ids = Vec::new();
    for target in targets {
        let (right_id, right_label, right_shape, right_classes) = parse_node_token(target);
        graph.ensure_node(&right_id, right_label, right_shape);
        apply_node_classes(graph, &right_id, &right_classes);
        add_node_to_subgraphs(graph, subgraph_stack, &right_id);
        target_ids.push(right_id);
    }

    for left_id in &source_ids {
        for right_id in &target_ids {
            graph.edges.push(crate::ir::Edge {
                from: left_id.clone(),
                to: right_id.clone(),
                label: label.clone(),
                start_label: None,
                end_label: None,
                directed: edge_meta.directed,
                arrow_start: edge_meta.arrow_start,
                arrow_end: edge_meta.arrow_end,
                arrow_start_kind: edge_meta.arrow_start_kind,
                arrow_end_kind: edge_meta.arrow_end_kind,
                start_decoration: edge_meta.start_decoration,
                end_decoration: edge_meta.end_decoration,
                style: edge_meta.style,
            });
        }
    }

    true
}

fn split_trailing_quoted(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_end();
    let quote = trimmed.chars().last()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut iter = trimmed.char_indices().rev();
    let _ = iter.next();
    for (idx, ch) in iter {
        if ch == quote {
            let before = &trimmed[..idx];
            let value = &trimmed[idx + 1..trimmed.len() - 1];
            return Some((before, value));
        }
    }
    None
}

fn split_leading_quoted(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_start();
    let mut iter = trimmed.char_indices();
    let Some((_, quote)) = iter.next() else {
        return None;
    };
    if quote != '"' && quote != '\'' {
        return None;
    }
    for (idx, ch) in iter {
        if ch == quote {
            let value = &trimmed[1..idx];
            let rest = &trimmed[idx + 1..];
            return Some((value, rest));
        }
    }
    None
}

fn split_multiplicity_left(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    if let Some((before, value)) = split_trailing_quoted(trimmed) {
        let before = before.trim();
        if !before.is_empty() && !value.is_empty() {
            return (before.to_string(), Some(value.to_string()));
        }
    }
    (trimmed.to_string(), None)
}

fn split_multiplicity_right(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    if let Some((value, rest)) = split_leading_quoted(trimmed) {
        let rest = rest.trim();
        if !rest.is_empty() && !value.is_empty() {
            return (rest.to_string(), Some(value.to_string()));
        }
    }
    (trimmed.to_string(), None)
}

fn parse_class_relation_line(
    line: &str,
) -> Option<(
    String,
    String,
    EdgeMeta,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    let tokens = [
        "<|..", "..|>", "<|--", "--|>", "*--", "--*", "o--", "--o", "<..", "..>", "<--", "-->",
        "..", "--",
    ];

    for token in tokens {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            let (left, start_label) = split_multiplicity_left(left);
            let (right, end_label) = split_multiplicity_right(&right);
            let meta = edge_meta_from_class_token(token);
            return Some((left, right, meta, label, start_label, end_label));
        }
    }
    None
}

fn edge_meta_from_class_token(token: &str) -> EdgeMeta {
    let arrow_start = token.contains('<');
    let arrow_end = token.contains('>');
    let directed = arrow_start || arrow_end;
    let style = if token.contains("..") {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };

    let mut start_decoration = None;
    let mut end_decoration = None;
    if token.starts_with('*') {
        start_decoration = Some(crate::ir::EdgeDecoration::DiamondFilled);
    }
    if token.ends_with('*') {
        end_decoration = Some(crate::ir::EdgeDecoration::DiamondFilled);
    }
    if token.starts_with('o') {
        start_decoration = Some(crate::ir::EdgeDecoration::Diamond);
    }
    if token.ends_with('o') {
        end_decoration = Some(crate::ir::EdgeDecoration::Diamond);
    }

    let mut arrow_start_kind = None;
    let mut arrow_end_kind = None;
    if token.contains('|') {
        if arrow_start {
            arrow_start_kind = Some(crate::ir::EdgeArrowhead::OpenTriangle);
        }
        if arrow_end {
            arrow_end_kind = Some(crate::ir::EdgeArrowhead::OpenTriangle);
        }
    } else {
        if arrow_start {
            arrow_start_kind = Some(crate::ir::EdgeArrowhead::ClassDependency);
        }
        if arrow_end {
            arrow_end_kind = Some(crate::ir::EdgeArrowhead::ClassDependency);
        }
    }

    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind,
        arrow_end_kind,
        start_decoration,
        end_decoration,
        style,
    }
}

fn parse_class_declaration(input: &str) -> Option<(String, Option<String>, Option<String>, bool)> {
    let mut rest = input.trim();
    if rest.is_empty() {
        return None;
    }

    let mut body: Option<String> = None;
    let mut open_body = false;
    if let Some(open_idx) = rest.find('{') {
        let header = rest[..open_idx].trim();
        let tail = rest[open_idx + 1..].trim();
        if let Some(close_idx) = tail.find('}') {
            let body_str = tail[..close_idx].trim();
            if !body_str.is_empty() {
                body = Some(body_str.to_string());
            }
        } else {
            open_body = true;
        }
        rest = header;
    }

    let lower = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower.find(" as ") {
        let label_part = rest[..as_idx].trim();
        let id_part = rest[as_idx + 4..].trim();
        if !id_part.is_empty() {
            let label = strip_quotes(label_part);
            return Some((id_part.to_string(), Some(label), body, open_body));
        }
    }

    if rest.starts_with('"') && rest.ends_with('"') {
        let label = strip_quotes(rest);
        return Some((label.clone(), Some(label), body, open_body));
    }

    let id = strip_quotes(rest);
    if let Some((name, _)) = id.split_once('~') {
        return Some((name.trim().to_string(), Some(id.clone()), body, open_body));
    }
    Some((id, None, body, open_body))
}

fn split_class_body(body: &str) -> Vec<String> {
    let mut entries = Vec::new();
    for part in body.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        for line in trimmed.lines() {
            let line_trim = line.trim();
            if !line_trim.is_empty() {
                entries.push(line_trim.to_string());
            }
        }
    }
    entries
}

fn normalize_class_method_signature(entry: &str) -> String {
    let trimmed = entry.trim();
    let Some(close_idx) = trimmed.find(')') else {
        return trimmed.to_string();
    };
    let (sig, rest) = trimmed.split_at(close_idx + 1);
    let rest = rest.trim();
    if rest.is_empty() {
        return trimmed.to_string();
    }
    if rest.starts_with(':') {
        return format!("{} {}", sig, rest);
    }
    if trimmed.contains("):") || trimmed.contains(") :") {
        return trimmed.to_string();
    }
    format!("{} : {}", sig, rest)
}

fn parse_class_member_line(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once(':')?;
    let id = left.trim();
    let member = right.trim();
    if id.is_empty() || member.is_empty() {
        return None;
    }
    if id.contains(' ') {
        return None;
    }
    Some((id.to_string(), member.to_string()))
}

fn normalize_class_id(token: &str) -> (String, Option<String>) {
    let trimmed = token.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        let label = strip_quotes(trimmed);
        return (label.clone(), Some(label));
    }
    (trimmed.to_string(), None)
}

fn is_class_stereotype(entry: &str) -> bool {
    let trimmed = entry.trim();
    trimmed.starts_with("<<") && trimmed.ends_with(">>") && trimmed.len() > 4
}

fn parse_state_alias_line(line: &str) -> Option<(String, String, Vec<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    if trimmed.contains('{') {
        return None;
    }
    let rest = trimmed.trim_start_matches("state ").trim();
    if !rest.starts_with('"') {
        return None;
    }
    let end_quote = rest[1..].find('"')? + 1;
    let label = rest[1..end_quote].to_string();
    let remaining = rest[end_quote + 1..].trim();
    if !remaining.to_ascii_lowercase().starts_with("as ") {
        return None;
    }
    let id = remaining[3..].trim();
    let (id, classes) = parse_state_id_with_classes(id);
    if id.is_empty() {
        return None;
    }
    Some((id, label, classes))
}

fn parse_state_stereotype(line: &str) -> (String, Option<crate::ir::NodeShape>, Option<String>) {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return (trimmed.to_string(), None, None);
    }
    let Some(start) = trimmed.find("<<") else {
        return (trimmed.to_string(), None, None);
    };
    let Some(end) = trimmed[start + 2..].find(">>") else {
        return (trimmed.to_string(), None, None);
    };
    let stereo_raw = &trimmed[start + 2..start + 2 + end];
    let stereo = stereo_raw.trim().to_ascii_lowercase();

    let before = trimmed[..start].trim_end();
    let after = trimmed[start + 2 + end + 2..].trim_start();
    let cleaned = if after.is_empty() {
        before.to_string()
    } else if before.is_empty() {
        after.to_string()
    } else {
        format!("{before} {after}")
    };

    let (shape, label_override) = match stereo.as_str() {
        "choice" => (Some(crate::ir::NodeShape::Diamond), None),
        "fork" | "join" => (Some(crate::ir::NodeShape::ForkJoin), Some(String::new())),
        _ => (None, None),
    };

    (cleaned, shape, label_override)
}

fn parse_state_description_line(line: &str) -> Option<(String, String, Vec<String>)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.to_ascii_lowercase().starts_with("note ") {
        return None;
    }
    let rest = if trimmed.starts_with("state ") {
        trimmed[6..].trim()
    } else {
        trimmed
    };
    if rest.to_ascii_lowercase().contains(" as ") {
        return None;
    }
    let mut sep = None;
    let bytes = rest.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b':' {
            if idx + 2 < bytes.len() && bytes[idx + 1] == b':' && bytes[idx + 2] == b':' {
                idx += 3;
                continue;
            }
            sep = Some(idx);
            break;
        }
        idx += 1;
    }
    let sep = sep?;
    let (id_part, desc_part) = rest.split_at(sep);
    let desc_part = desc_part.get(1..).unwrap_or("");
    let (id, classes) = parse_state_id_with_classes(id_part.trim());
    let desc = strip_quotes(desc_part.trim());
    if id.is_empty() || desc.is_empty() {
        return None;
    }
    Some((id, desc, classes))
}

fn state_display_label(
    id: &str,
    labels: &HashMap<String, String>,
    descriptions: &HashMap<String, Vec<String>>,
) -> String {
    let title = labels.get(id).map(String::as_str).unwrap_or(id);
    let Some(descriptions) = descriptions.get(id) else {
        return title.to_string();
    };
    if descriptions.is_empty() {
        return title.to_string();
    }

    let mut label = String::with_capacity(
        title.len() + descriptions.iter().map(String::len).sum::<usize>() + descriptions.len() + 4,
    );
    label.push_str(title);
    label.push_str("\n---");
    for description in descriptions {
        label.push('\n');
        label.push_str(description);
    }
    label
}

fn state_display_label_option(
    id: &str,
    labels: &HashMap<String, String>,
    descriptions: &HashMap<String, Vec<String>>,
) -> Option<String> {
    if labels.contains_key(id) || descriptions.contains_key(id) {
        Some(state_display_label(id, labels, descriptions))
    } else {
        None
    }
}

fn parse_state_id_with_classes(input: &str) -> (String, Vec<String>) {
    let (base, classes) = split_inline_classes(input);
    (strip_quotes(base.trim()), classes)
}

fn parse_state_note(line: &str) -> Option<(crate::ir::StateNotePosition, String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let rest = trimmed[4..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let (position, targets_part) = if lower_rest.starts_with("right of ") {
        (crate::ir::StateNotePosition::RightOf, rest[9..].trim())
    } else if lower_rest.starts_with("left of ") {
        (crate::ir::StateNotePosition::LeftOf, rest[8..].trim())
    } else {
        return None;
    };
    let (target, label) = targets_part.split_once(':')?;
    let target = target.trim();
    let label = label.trim();
    if target.is_empty() || label.is_empty() {
        return None;
    }
    Some((position, target.to_string(), label.to_string()))
}

fn parse_state_transition(line: &str) -> Option<(String, EdgeMeta, String, Option<String>)> {
    let tokens = ["<-->", "<--", "-->", "<->", "->", "<-", "..>", "<.."];
    for token in tokens {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            let meta = edge_meta_from_state_token(token);
            return Some((left.to_string(), meta, right.to_string(), label));
        }
    }
    None
}

fn edge_meta_from_state_token(token: &str) -> EdgeMeta {
    let arrow_start = token.contains('<');
    let arrow_end = token.contains('>');
    let directed = arrow_start || arrow_end;
    let style = if token.contains("..") {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };
    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind: None,
        arrow_end_kind: None,
        start_decoration: None,
        end_decoration: None,
        style,
    }
}

fn normalize_state_token(
    token: &str,
    is_start: bool,
    start_states: &mut HashMap<String, String>,
    end_states: &mut HashMap<String, String>,
    scope: &str,
) -> (String, crate::ir::NodeShape, Option<String>) {
    let trimmed = token.trim();
    if trimmed == "[*]" || trimmed == "*" {
        let (id, shape) = if is_start {
            // Start states are shared per scope. This lets fan-out/fan-in
            // patterns be recognized and rendered as fork/join bars.
            let id = start_states
                .entry(scope.to_string())
                .or_insert_with(|| format!("__start_{}__", scope))
                .clone();
            (id, crate::ir::NodeShape::Circle)
        } else {
            // End states are shared per scope - all X --> [*] in same scope go to same node
            let id = end_states
                .entry(scope.to_string())
                .or_insert_with(|| format!("__end_{}__", scope))
                .clone();
            (id, crate::ir::NodeShape::DoubleCircle)
        };
        return (id, shape, Some(String::new()));
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::RoundRect, None)
}

fn parse_state_simple(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    if trimmed.contains('{') {
        return None;
    }
    let mut rest = trimmed.trim_start_matches("state ").trim();
    if rest.to_ascii_lowercase().contains(" as ") {
        return None;
    }
    if let Some(idx) = rest.find('{') {
        rest = rest[..idx].trim();
    }
    if rest.is_empty() {
        return None;
    }
    let (id, classes) = parse_state_id_with_classes(rest);
    if id.is_empty() {
        return None;
    }
    Some((id, classes))
}

fn parse_state_container_header(line: &str) -> Option<(Option<String>, String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    let brace_idx = trimmed.find('{')?;
    let head = trimmed[..brace_idx].trim();
    let tail = trimmed[brace_idx + 1..].trim().to_string();

    let rest = head.trim_start_matches("state ").trim();
    if rest.is_empty() {
        return None;
    }

    if rest.starts_with('"') {
        let end_quote = rest[1..].find('"')? + 1;
        let label = rest[1..end_quote].to_string();
        let remaining = rest[end_quote + 1..].trim();
        if remaining.to_ascii_lowercase().starts_with("as ") {
            let id = remaining[3..].trim();
            if id.is_empty() {
                return None;
            }
            return Some((Some(id.to_string()), label, tail));
        }
        return Some((None, label, tail));
    }

    let lower = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower.find(" as ") {
        let id_part = rest[..as_idx].trim();
        let label_part = rest[as_idx + 4..].trim();
        if id_part.is_empty() || label_part.is_empty() {
            return None;
        }
        let id = strip_quotes(id_part);
        let label = strip_quotes(label_part);
        return Some((Some(id), label, tail));
    }

    let id = strip_quotes(rest);
    Some((Some(id.clone()), id, tail))
}

fn parse_sequence_participant(
    line: &str,
) -> Option<(String, Option<String>, crate::ir::NodeShape)> {
    let lowered = line.to_ascii_lowercase();
    let keywords = [
        ("participant ", crate::ir::NodeShape::ActorBox),
        ("actor ", crate::ir::NodeShape::ActorBox),
        ("boundary ", crate::ir::NodeShape::ActorBox),
        ("control ", crate::ir::NodeShape::ActorBox),
        ("entity ", crate::ir::NodeShape::ActorBox),
        ("database ", crate::ir::NodeShape::Cylinder),
    ];
    let mut rest = None;
    let mut shape = crate::ir::NodeShape::ActorBox;
    for (keyword, keyword_shape) in keywords {
        if lowered.starts_with(keyword) {
            rest = Some(line[keyword.len()..].trim());
            shape = keyword_shape;
            break;
        }
    }
    let rest = rest?;
    if rest.is_empty() {
        return None;
    }

    let lower_rest = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower_rest.find(" as ") {
        let label_part = rest[..as_idx].trim();
        let id_part = rest[as_idx + 4..].trim();
        if id_part.is_empty() {
            return None;
        }
        let id = strip_quotes(label_part);
        let display_label = strip_quotes(id_part);
        return Some((id, Some(display_label), shape));
    }

    if rest.starts_with('"') && rest.ends_with('"') {
        let label = strip_quotes(rest);
        return Some((label.clone(), Some(label), shape));
    }

    Some((strip_quotes(rest), None, shape))
}

fn is_color_token(token: &str) -> bool {
    let lower = token.trim().to_ascii_lowercase();
    lower == "transparent"
        || lower.starts_with('#')
        || lower.starts_with("rgb(")
        || lower.starts_with("rgba(")
        || lower.starts_with("hsl(")
        || lower.starts_with("hsla(")
}

fn parse_sequence_box_line(line: &str) -> Option<(Option<String>, Option<String>)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("box") {
        return None;
    }
    let rest = trimmed[3..].trim();
    if rest.is_empty() {
        return Some((None, None));
    }
    let tokens = tokenize_quoted(rest);
    if tokens.is_empty() {
        return Some((None, None));
    }
    let first = tokens[0].clone();
    if first.eq_ignore_ascii_case("transparent") {
        let label = tokens[1..].join(" ");
        let label = if label.trim().is_empty() {
            None
        } else {
            Some(label)
        };
        return Some((None, label));
    }
    let color = if tokens.len() > 1 {
        Some(first.clone())
    } else if is_color_token(&first) {
        Some(first.clone())
    } else {
        None
    };
    let label = if tokens.len() > 1 {
        Some(tokens[1..].join(" "))
    } else if color.is_none() {
        Some(first.clone())
    } else {
        None
    };
    let label = label.filter(|value| !value.trim().is_empty());
    let color = color.filter(|value| !value.eq_ignore_ascii_case("transparent"));
    Some((color, label))
}

fn ensure_sequence_node(
    graph: &mut Graph,
    labels: &HashMap<String, String>,
    id: &str,
    shape: Option<crate::ir::NodeShape>,
) {
    let label = labels.get(id).cloned();
    if let Some(shape) = shape {
        graph.ensure_node(id, label, Some(shape));
        return;
    }
    if graph.nodes.contains_key(id) {
        graph.ensure_node(id, label, None);
    } else {
        graph.ensure_node(id, label, Some(crate::ir::NodeShape::ActorBox));
    }
}

fn parse_sequence_message(
    line: &str,
) -> Option<(
    String,
    String,
    Option<String>,
    crate::ir::EdgeStyle,
    Option<crate::ir::SequenceActivationKind>,
)> {
    let tokens = [
        "-->>+", "->>+", "-->+", "->+", "-->>-", "->>-", "-->-", "->-", "<--+", "<-+", "<--", "<-",
        "-->>", "->>", "-->", "->",
    ];
    for token in tokens {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            let mut from = left.to_string();
            let mut to = right.to_string();
            if token.starts_with('<') {
                std::mem::swap(&mut from, &mut to);
            }
            let trimmed = token.trim_start_matches('<').trim_end_matches(['+', '-']);
            let style = if trimmed.starts_with("--") {
                crate::ir::EdgeStyle::Dotted
            } else {
                crate::ir::EdgeStyle::Solid
            };
            let activation = if token.ends_with('+') {
                Some(crate::ir::SequenceActivationKind::Activate)
            } else if token.ends_with('-') {
                Some(crate::ir::SequenceActivationKind::Deactivate)
            } else {
                None
            };
            return Some((from, to, label, style, activation));
        }
    }
    None
}

fn parse_sequence_note(
    line: &str,
) -> Option<(crate::ir::SequenceNotePosition, Vec<String>, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let rest = trimmed[4..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let (position, targets_part) = if lower_rest.starts_with("left of ") {
        (crate::ir::SequenceNotePosition::LeftOf, rest[8..].trim())
    } else if lower_rest.starts_with("right of ") {
        (crate::ir::SequenceNotePosition::RightOf, rest[9..].trim())
    } else if lower_rest.starts_with("over ") {
        (crate::ir::SequenceNotePosition::Over, rest[5..].trim())
    } else {
        return None;
    };

    let (targets, label) = targets_part.split_once(':')?;
    let label = label.trim();
    if label.is_empty() {
        return None;
    }
    let participants = targets
        .split(',')
        .map(|part| strip_quotes(part.trim()))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if participants.is_empty() {
        return None;
    }

    Some((position, participants, label.to_string()))
}

fn split_label(input: &str) -> (String, Option<String>) {
    if let Some((left, right)) = input.split_once(':') {
        let label = right.trim();
        let target = left.trim();
        if !label.is_empty() {
            return (target.to_string(), Some(label.to_string()));
        }
        return (target.to_string(), None);
    }
    (input.trim().to_string(), None)
}

fn parse_class_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Class;
    graph.direction = Direction::TopDown;
    let lines = preprocess_input(input)?;

    let mut members: HashMap<String, Vec<String>> = HashMap::new();
    let mut stereotypes: HashMap<String, Vec<String>> = HashMap::new();
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut current_class: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("classdiagram") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1
                && let Some(dir) = Direction::from_token(parts[1])
            {
                graph.direction = dir;
            }
            continue;
        }

        if let Some(direction) = parse_direction_line(line) {
            graph.direction = direction;
            continue;
        }

        if let Some(active) = current_class.clone() {
            if let Some(end_idx) = line.find('}') {
                let fragment = line[..end_idx].trim();
                if !fragment.is_empty() {
                    if is_class_stereotype(fragment) {
                        stereotypes
                            .entry(active.clone())
                            .or_default()
                            .push(fragment.to_string());
                    } else {
                        members
                            .entry(active.clone())
                            .or_default()
                            .push(fragment.to_string());
                    }
                }
                current_class = None;
            } else if is_class_stereotype(line.trim()) {
                stereotypes
                    .entry(active.clone())
                    .or_default()
                    .push(line.trim().to_string());
            } else {
                members
                    .entry(active.clone())
                    .or_default()
                    .push(line.to_string());
            }
            continue;
        }

        if let Some((left, right, meta, label, start_label, end_label)) =
            parse_class_relation_line(line)
        {
            let (left_id, left_label) = normalize_class_id(&left);
            let (right_id, right_label) = normalize_class_id(&right);
            if let Some(label) = left_label {
                labels.insert(left_id.clone(), label);
            }
            if let Some(label) = right_label {
                labels.insert(right_id.clone(), label);
            }
            graph.ensure_node(
                &left_id,
                labels.get(&left_id).cloned(),
                Some(crate::ir::NodeShape::Rectangle),
            );
            graph.ensure_node(
                &right_id,
                labels.get(&right_id).cloned(),
                Some(crate::ir::NodeShape::Rectangle),
            );
            graph.edges.push(crate::ir::Edge {
                from: left_id,
                to: right_id,
                label,
                start_label,
                end_label,
                directed: meta.directed,
                arrow_start: meta.arrow_start,
                arrow_end: meta.arrow_end,
                arrow_start_kind: meta.arrow_start_kind,
                arrow_end_kind: meta.arrow_end_kind,
                start_decoration: meta.start_decoration,
                end_decoration: meta.end_decoration,
                style: meta.style,
            });
            continue;
        }

        if line.starts_with("class ") {
            let rest = line.trim_start_matches("class ").trim();
            if let Some((id, label, body, open_body)) = parse_class_declaration(rest) {
                if let Some(label) = label.clone() {
                    labels.insert(id.clone(), label);
                }
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    Some(crate::ir::NodeShape::Rectangle),
                );
                if let Some(body) = body {
                    for entry in split_class_body(&body) {
                        if !entry.is_empty() {
                            if is_class_stereotype(&entry) {
                                stereotypes.entry(id.clone()).or_default().push(entry);
                            } else {
                                members.entry(id.clone()).or_default().push(entry);
                            }
                        }
                    }
                }
                if open_body {
                    current_class = Some(id.clone());
                }
                continue;
            }
        }

        if let Some((id, member)) = parse_class_member_line(line) {
            if is_class_stereotype(&member) {
                stereotypes.entry(id).or_default().push(member);
            } else {
                members.entry(id).or_default().push(member);
            }
            continue;
        }
    }

    for (id, node) in graph.nodes.iter_mut() {
        let class_name = labels
            .get(id)
            .cloned()
            .unwrap_or_else(|| node.label.clone());
        let mut lines = Vec::new();
        if let Some(st) = stereotypes.get(id) {
            lines.extend(st.iter().cloned());
        }
        lines.push(class_name.clone());
        if let Some(items) = members.get(id)
            && !items.is_empty()
        {
            let mut attrs = Vec::new();
            let mut methods = Vec::new();
            for entry in items {
                let trimmed = entry.trim();
                if trimmed.contains('(') && trimmed.contains(')') {
                    methods.push(normalize_class_method_signature(trimmed));
                } else {
                    attrs.push(trimmed.to_string());
                }
            }
            if !attrs.is_empty() || !methods.is_empty() {
                lines.push("---".to_string());
                if !attrs.is_empty() {
                    lines.extend(attrs);
                    if !methods.is_empty() {
                        lines.push("---".to_string());
                        lines.extend(methods);
                    }
                } else {
                    lines.extend(methods);
                }
            }
        }
        node.label = lines.join("\n");
    }

    Ok(ParseOutput { graph })
}

fn is_er_card_char(ch: char) -> bool {
    matches!(ch, '|' | 'o' | '{' | '}')
}

fn split_er_cardinality_left(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let len = chars.len();
    if len >= 2 {
        let last_two = &chars[len - 2..];
        if last_two.iter().all(|ch| is_er_card_char(*ch)) {
            let entity = chars[..len - 2].iter().collect::<String>();
            let token = last_two.iter().collect::<String>();
            return (entity.trim().to_string(), Some(token));
        }
    }
    if let Some(&last) = chars.last()
        && is_er_card_char(last)
    {
        let entity = chars[..len - 1].iter().collect::<String>();
        return (entity.trim().to_string(), Some(last.to_string()));
    }
    (trimmed.to_string(), None)
}

fn split_er_cardinality_right(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let len = chars.len();
    if len >= 2 {
        let first_two = &chars[..2];
        if first_two.iter().all(|ch| is_er_card_char(*ch)) {
            let entity = chars[2..].iter().collect::<String>();
            let token = first_two.iter().collect::<String>();
            return (entity.trim().to_string(), Some(token));
        }
    }
    if is_er_card_char(chars[0]) {
        let entity = chars[1..].iter().collect::<String>();
        return (entity.trim().to_string(), Some(chars[0].to_string()));
    }
    (trimmed.to_string(), None)
}

fn normalize_er_cardinality(token: &str) -> (String, Option<crate::ir::EdgeDecoration>) {
    let trimmed = token.trim();
    match trimmed {
        "||" | "|" => (
            "1".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootOne),
        ),
        "o|" | "|o" | "o" => (
            "0..1".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootZeroOne),
        ),
        "|{" | "}|" => (
            "1..*".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootMany),
        ),
        "o{" | "}o" | "}" | "{" => (
            "0..*".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootZeroMany),
        ),
        _ => (trimmed.to_string(), None),
    }
}

fn parse_er_relation_line(
    line: &str,
) -> Option<(
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<crate::ir::EdgeDecoration>,
    Option<crate::ir::EdgeDecoration>,
    crate::ir::EdgeStyle,
)> {
    let (relation_part, label) = if let Some((before, after)) = line.split_once(':') {
        let label = after.trim();
        let label = if label.is_empty() {
            None
        } else {
            Some(label.to_string())
        };
        (before.trim(), label)
    } else {
        (line.trim(), None)
    };

    let (sep, style) = if let Some(idx) = relation_part.find("--") {
        (idx, crate::ir::EdgeStyle::Solid)
    } else if let Some(idx) = relation_part.find("..") {
        (idx, crate::ir::EdgeStyle::Dotted)
    } else {
        return None;
    };
    let left_part = relation_part[..sep].trim();
    let right_part = relation_part[sep + 2..].trim();
    if left_part.is_empty() || right_part.is_empty() {
        return None;
    }
    let (left_entity, left_card) = split_er_cardinality_left(left_part);
    let (right_entity, right_card) = split_er_cardinality_right(right_part);
    if left_entity.is_empty() || right_entity.is_empty() {
        return None;
    }
    let left_id = strip_quotes(left_entity.trim());
    let right_id = strip_quotes(right_entity.trim());
    if left_id.is_empty() || right_id.is_empty() {
        return None;
    }
    let (left_label, left_decoration) = left_card
        .map(|token| normalize_er_cardinality(&token))
        .map(|(label, dec)| (Some(label), dec))
        .unwrap_or((None, None));
    let (right_label, right_decoration) = right_card
        .map(|token| normalize_er_cardinality(&token))
        .map(|(label, dec)| (Some(label), dec))
        .unwrap_or((None, None));
    Some((
        left_id,
        right_id,
        label,
        left_label,
        right_label,
        left_decoration,
        right_decoration,
        style,
    ))
}

fn parse_er_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Er;
    graph.direction = Direction::TopDown;
    let lines = preprocess_input(input)?;

    let mut members: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_entity: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("erdiagram") {
            continue;
        }
        if let Some(direction) = parse_direction_line(line) {
            graph.direction = direction;
            continue;
        }

        if let Some(active) = current_entity.clone() {
            if let Some(end_idx) = line.find('}') {
                let fragment = line[..end_idx].trim();
                if !fragment.is_empty() {
                    members
                        .entry(active.clone())
                        .or_default()
                        .push(fragment.to_string());
                }
                current_entity = None;
            } else {
                members
                    .entry(active.clone())
                    .or_default()
                    .push(line.to_string());
            }
            continue;
        }

        if let Some((
            left,
            right,
            label,
            _left_label,
            _right_label,
            left_decoration,
            right_decoration,
            style,
        )) = parse_er_relation_line(line)
        {
            graph.ensure_node(&left, None, Some(crate::ir::NodeShape::RoundRect));
            graph.ensure_node(&right, None, Some(crate::ir::NodeShape::RoundRect));
            // Don't use start_label/end_label for ER diagrams - crow's foot symbols convey cardinality
            graph.edges.push(crate::ir::Edge {
                from: left,
                to: right,
                label,
                start_label: None,
                end_label: None,
                directed: false,
                arrow_start: false,
                arrow_end: false,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: left_decoration,
                end_decoration: right_decoration,
                style,
            });
            continue;
        }

        if let Some(open_idx) = line.find('{') {
            let name = line[..open_idx].trim();
            let name = strip_quotes(name);
            if !name.is_empty() {
                graph.ensure_node(&name, None, Some(crate::ir::NodeShape::RoundRect));
                current_entity = Some(name.clone());
                let tail = line[open_idx + 1..].trim();
                if let Some(close_idx) = tail.find('}') {
                    let fragment = tail[..close_idx].trim();
                    if !fragment.is_empty() {
                        members.entry(name).or_default().push(fragment.to_string());
                    }
                    current_entity = None;
                } else if !tail.is_empty() {
                    members.entry(name).or_default().push(tail.to_string());
                }
            }
            continue;
        }

        let entity = strip_quotes(line);
        if !entity.is_empty() {
            graph.ensure_node(&entity, None, Some(crate::ir::NodeShape::RoundRect));
        }
    }

    for (id, node) in graph.nodes.iter_mut() {
        let mut lines = Vec::new();
        lines.push(node.label.clone());
        if let Some(attrs) = members.get(id)
            && !attrs.is_empty()
        {
            lines.push("---".to_string());
            lines.extend(attrs.clone());
        }
        node.label = lines.join("\n");
    }

    Ok(ParseOutput { graph })
}

fn parse_pie_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Pie;
    let lines = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("pie") {
            if lower.contains("showdata") {
                graph.pie_show_data = true;
            }
            // Check for title on the same line: "pie title My Title"
            if let Some(title_pos) = lower.find("title") {
                let title_start = title_pos + 5; // len("title")
                if let Some(title) = line.get(title_start..) {
                    let title = title.trim();
                    if !title.is_empty() {
                        graph.pie_title = Some(title.to_string());
                    }
                }
            }
            continue;
        }
        if lower.starts_with("showdata") {
            graph.pie_show_data = true;
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.pie_title = Some(title.to_string());
            }
            continue;
        }
        if let Some((label, value)) = parse_pie_slice_line(line) {
            graph.pie_slices.push(crate::ir::PieSlice { label, value });
        }
    }

    Ok(ParseOutput { graph })
}

fn parse_pie_slice_line(line: &str) -> Option<(String, f32)> {
    let (label_part, value_part) = line.split_once(':')?;
    let label = strip_quotes(label_part.trim());
    if label.is_empty() {
        return None;
    }
    let value_str = value_part.trim();
    if value_str.is_empty() {
        return None;
    }
    let value = value_str.parse::<f32>().ok()?;
    Some((label, value))
}

fn parse_mindmap_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Mindmap;
    graph.direction = Direction::LeftRight;
    let lines = preprocess_input_keep_indent(input)?;
    let mut stack: Vec<String> = Vec::new();
    let mut base_indent: Option<usize> = None;
    let mut node_index: HashMap<String, usize> = HashMap::new();

    for raw_line in lines {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("mindmap") {
            continue;
        }

        let indent = count_indent(&raw_line);
        let base = *base_indent.get_or_insert(indent);
        let rel_indent = indent.saturating_sub(base);
        let mut level = rel_indent / 2;
        if level > stack.len() {
            level = stack.len();
        }

        let (raw_id, label, node_type, classes) = parse_mindmap_node_token(trimmed);
        let mut id = raw_id;
        if id.is_empty() {
            id = sanitize_id(&label);
        }
        if id.is_empty() {
            id = format!("mindmap_{}", graph.mindmap.nodes.len());
        }
        if graph.nodes.contains_key(&id) {
            id = format!("{}_{}", id, graph.nodes.len());
        }

        let shape = match node_type {
            crate::ir::MindmapNodeType::Circle => crate::ir::NodeShape::Circle,
            crate::ir::MindmapNodeType::RoundedRect => crate::ir::NodeShape::RoundRect,
            crate::ir::MindmapNodeType::Rect => crate::ir::NodeShape::Rectangle,
            crate::ir::MindmapNodeType::Hexagon => crate::ir::NodeShape::Hexagon,
            crate::ir::MindmapNodeType::Cloud | crate::ir::MindmapNodeType::Bang => {
                crate::ir::NodeShape::RoundRect
            }
            crate::ir::MindmapNodeType::Default => crate::ir::NodeShape::MindmapDefault,
        };

        graph.ensure_node(&id, Some(label.clone()), Some(shape));
        if !classes.is_empty() {
            apply_node_classes(&mut graph, &id, &classes);
        }

        if graph.mindmap.root_id.is_none() {
            graph.mindmap.root_id = Some(id.clone());
        }

        if level > 0 && stack.len() > level {
            stack.truncate(level);
        }

        let parent_id = if level > 0 {
            stack.last().cloned()
        } else {
            None
        };

        let section = if level == 0 {
            None
        } else if let Some(parent_id) = parent_id.as_ref() {
            let parent_idx = node_index.get(parent_id).copied();
            if let Some(parent_idx) = parent_idx {
                let parent = &graph.mindmap.nodes[parent_idx];
                if parent.level == 0 {
                    Some(parent.children.len())
                } else {
                    parent.section
                }
            } else {
                None
            }
        } else {
            None
        };

        let node = crate::ir::MindmapNode {
            id: id.clone(),
            label: label.clone(),
            level,
            section,
            node_type,
            icon: None,
            class: None,
            children: Vec::new(),
        };

        let idx = graph.mindmap.nodes.len();
        graph.mindmap.nodes.push(node);
        node_index.insert(id.clone(), idx);

        if let Some(parent_id) = parent_id {
            if let Some(parent_idx) = node_index.get(&parent_id).copied() {
                graph.mindmap.nodes[parent_idx].children.push(id.clone());
            }
            graph.edges.push(crate::ir::Edge {
                from: parent_id,
                to: id.clone(),
                label: None,
                start_label: None,
                end_label: None,
                directed: false,
                arrow_start: false,
                arrow_end: false,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                style: crate::ir::EdgeStyle::Solid,
            });
        } else {
            stack.clear();
        }

        stack.push(id);
    }

    Ok(ParseOutput { graph })
}

fn parse_mindmap_node_token(
    token: &str,
) -> (String, String, crate::ir::MindmapNodeType, Vec<String>) {
    let (base, classes) = split_inline_classes(token);
    let trimmed = base.trim();
    if trimmed.is_empty() {
        return (
            String::new(),
            String::new(),
            crate::ir::MindmapNodeType::Default,
            classes,
        );
    }

    let mut id = String::new();
    let mut label = trimmed.to_string();
    let mut node_type = crate::ir::MindmapNodeType::Default;

    let shape_start = trimmed.find(['[', '(', '{']).unwrap_or(0);
    if shape_start > 0 && !trimmed[..shape_start].contains(' ') {
        id = trimmed[..shape_start].trim().to_string();
        let raw = trimmed[shape_start..].trim();
        if let Some((shape_label, shape_type)) = parse_mindmap_shape(raw) {
            label = shape_label;
            node_type = shape_type;
        }
    } else if let Some((shape_label, shape_type)) = parse_mindmap_shape(trimmed) {
        label = shape_label;
        node_type = shape_type;
    }

    if id.is_empty() {
        id = sanitize_id(&label);
    }

    (id, label, node_type, classes)
}

fn parse_mindmap_shape(raw: &str) -> Option<(String, crate::ir::MindmapNodeType)> {
    let trimmed = raw.trim();
    if trimmed.starts_with("((") && trimmed.ends_with("))") {
        return Some((
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::MindmapNodeType::Circle,
        ));
    }
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
        return Some((
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::MindmapNodeType::Hexagon,
        ));
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return Some((
            strip_quotes(&trimmed[1..trimmed.len() - 1]),
            crate::ir::MindmapNodeType::Rect,
        ));
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        return Some((
            strip_quotes(&trimmed[1..trimmed.len() - 1]),
            crate::ir::MindmapNodeType::RoundedRect,
        ));
    }
    None
}

fn sanitize_id(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}





fn extract_frontmatter_value(input: &str, key: &str) -> Option<String> {
    let mut in_frontmatter = false;
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_frontmatter {
                return None;
            }
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter
            && let Some((k, v)) = trimmed.split_once(':')
            && k.trim() == key
        {
            let val = v.trim().to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    None
}

fn parse_gantt_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Gantt;
    graph.direction = Direction::LeftRight;
    let lines = preprocess_input(input)?;

    graph.gantt_display_mode = extract_frontmatter_value(input, "displayMode");

    let mut current_section: Option<usize> = None;
    let mut current_section_name: Option<String> = None;
    let mut last_task: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("gantt") {
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.gantt_title = Some(title.to_string());
            }
            continue;
        }
        if lower.starts_with("dateformat")
            || lower.starts_with("axisformat")
            || lower.starts_with("tickinterval")
            || lower.starts_with("todaymarker")
            || lower.starts_with("excludes")
            || lower.starts_with("includes")
        {
            continue;
        }
        if lower.starts_with("section") {
            let label = line.get(7..).unwrap_or("").trim();
            let id = format!("section_{}", graph.subgraphs.len());
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: label.to_string(),
                nodes: Vec::new(),
                direction: None,
                icon: None,
            });
            current_section = Some(graph.subgraphs.len() - 1);
            current_section_name = Some(label.to_string());
            graph.gantt_sections.push(label.to_string());
            last_task = None;
            continue;
        }

        if let Some((task_label, meta)) = line.split_once(':') {
            let label = task_label.trim();
            if label.is_empty() {
                continue;
            }
            let (id, details, after, status) = parse_gantt_task_meta(meta);
            let node_id = id
                .clone()
                .unwrap_or_else(|| format!("gantt_{}", graph.nodes.len()));
            let mut node_label = label.to_string();
            if !details.is_empty() {
                node_label.push_str(&format!("\n{}", details.join(" | ")));
            }

            // Add to gantt_tasks
            let (start, duration) = extract_gantt_timing(&details);
            graph.gantt_tasks.push(crate::ir::GanttTask {
                id: node_id.clone(),
                label: label.to_string(),
                start,
                duration,
                after: after.clone(),
                section: current_section_name.clone(),
                status,
            });

            graph.ensure_node(
                &node_id,
                Some(node_label),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(idx) = current_section
                && let Some(subgraph) = graph.subgraphs.get_mut(idx)
            {
                subgraph.nodes.push(node_id.clone());
            }

            if let Some(after_id) = after {
                graph.ensure_node(&after_id, None, Some(crate::ir::NodeShape::Rectangle));
                graph.edges.push(crate::ir::Edge {
                    from: after_id,
                    to: node_id.clone(),
                    label: None,
                    start_label: None,
                    end_label: None,
                    directed: true,
                    arrow_start: false,
                    arrow_end: true,
                    arrow_start_kind: None,
                    arrow_end_kind: None,
                    start_decoration: None,
                    end_decoration: None,
                    style: crate::ir::EdgeStyle::Solid,
                });
            } else if let Some(prev) = last_task.take() {
                graph.edges.push(crate::ir::Edge {
                    from: prev,
                    to: node_id.clone(),
                    label: None,
                    start_label: None,
                    end_label: None,
                    directed: false,
                    arrow_start: false,
                    arrow_end: false,
                    arrow_start_kind: None,
                    arrow_end_kind: None,
                    start_decoration: None,
                    end_decoration: None,
                    style: crate::ir::EdgeStyle::Solid,
                });
            }

            last_task = Some(node_id);
        }
    }

    Ok(ParseOutput { graph })
}

fn parse_gantt_task_meta(
    meta: &str,
) -> (
    Option<String>,
    Vec<String>,
    Option<String>,
    Option<crate::ir::GanttStatus>,
) {
    let mut id: Option<String> = None;
    let mut details: Vec<String> = Vec::new();
    let mut after: Option<String> = None;
    let mut status: Option<crate::ir::GanttStatus> = None;

    for raw_token in meta.split(',') {
        let token = raw_token.trim();
        if token.is_empty() {
            continue;
        }
        let lower = token.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("after ") {
            let dep = rest.trim().to_string();
            if !dep.is_empty() {
                after = Some(dep);
            }
            continue;
        }
        if let Some(token_status) = gantt_status_from_token(&lower) {
            status = Some(token_status);
            details.push(token.to_string());
            continue;
        }
        if looks_like_date(token) || looks_like_duration(token) {
            details.push(token.to_string());
            continue;
        }
        if id.is_none() {
            id = Some(token.to_string());
        } else {
            details.push(token.to_string());
        }
    }

    (id, details, after, status)
}

fn gantt_status_from_token(token: &str) -> Option<crate::ir::GanttStatus> {
    match token {
        "done" => Some(crate::ir::GanttStatus::Done),
        "active" => Some(crate::ir::GanttStatus::Active),
        "crit" => Some(crate::ir::GanttStatus::Crit),
        "milestone" => Some(crate::ir::GanttStatus::Milestone),
        _ => None,
    }
}

fn looks_like_date(token: &str) -> bool {
    token.contains('-') || token.contains('/') || token.contains('.')
}

fn looks_like_duration(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    // A duration is a number followed by a unit (d/h/w/m/y), e.g. "7d", "1.5w".
    // Requiring the leading digits avoids misreading task ids that merely end
    // in one of those letters (e.g. "arch" ending in 'h', "day" in 'y').
    let unit = lower.chars().last();
    if !matches!(
        unit,
        Some('d') | Some('h') | Some('w') | Some('m') | Some('y')
    ) {
        return false;
    }
    let digits = &lower[..lower.len() - 1];
    !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit() || c == '.')
        && digits.chars().any(|c| c.is_ascii_digit())
}

fn extract_gantt_timing(details: &[String]) -> (Option<String>, Option<String>) {
    let mut start: Option<String> = None;
    let mut duration: Option<String> = None;
    for detail in details {
        if looks_like_date(detail) && start.is_none() {
            start = Some(detail.clone());
        } else if looks_like_duration(detail) && duration.is_none() {
            duration = Some(detail.clone());
        }
    }
    (start, duration)
}


















































#[allow(dead_code)]
fn parse_xy_series_line(line: &str) -> Option<(String, Vec<String>)> {
    let mut parts = line.splitn(2, ' ');
    let series = parts.next()?.trim().to_string();
    let rest = parts.next()?.trim();
    let values = rest
        .trim_matches(|ch| ch == '[' || ch == ']')
        .split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if series.is_empty() {
        None
    } else {
        Some((series, values))
    }
}

fn parse_state_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::State;
    let lines = preprocess_input(input)?;

    let mut labels: HashMap<String, String> = HashMap::new();
    let mut descriptions: HashMap<String, Vec<String>> = HashMap::new();
    let mut start_states: HashMap<String, String> = HashMap::new();
    let mut end_states: HashMap<String, String> = HashMap::new();
    let mut subgraph_stack: Vec<usize> = Vec::new();
    let mut region_counter: usize = 0;

    #[derive(Debug)]
    struct CompositeContext {
        subgraph_idx: usize,
        regions: Vec<Vec<String>>,
        current_region: usize,
        has_separator: bool,
    }

    let mut composite_stack: Vec<CompositeContext> = Vec::new();
    let mut pending: VecDeque<String> = lines.into();

    let record_region_node = |stack: &mut [CompositeContext], node_id: &str| {
        for ctx in stack.iter_mut() {
            if ctx
                .regions
                .iter()
                .any(|region| region.iter().any(|id| id == node_id))
            {
                continue;
            }
            let region = &mut ctx.regions[ctx.current_region];
            region.push(node_id.to_string());
        }
    };

    let finalize_regions =
        |ctx: CompositeContext, graph: &mut Graph, region_counter: &mut usize| {
            if !ctx.has_separator {
                return;
            }
            let mut regions: Vec<Vec<String>> = ctx
                .regions
                .into_iter()
                .filter(|region| !region.is_empty())
                .collect();
            if regions.len() <= 1 {
                return;
            }
            for region_nodes in regions.drain(..) {
                let id = format!("__region_{}__", *region_counter);
                *region_counter += 1;
                graph.subgraphs.push(Subgraph {
                    id: Some(id.clone()),
                    label: String::new(),
                    nodes: region_nodes,
                    direction: None,
                    icon: None,
                });
                graph.subgraph_styles.insert(
                    id,
                    NodeStyle {
                        fill: Some("none".to_string()),
                        stroke: Some("none".to_string()),
                        text_color: None,
                        stroke_width: Some(0.0),
                        stroke_dasharray: None,
                        line_color: None,
                    },
                );
            }
        };
    while let Some(raw_line) = pending.pop_front() {
        for raw_statement in split_statements(&raw_line) {
            let raw_line = raw_statement.trim();
            if raw_line.is_empty() {
                continue;
            }
            let (line, state_shape, label_override) = parse_state_stereotype(raw_line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("statediagram") {
                continue;
            }

            if let Some(direction) = parse_direction_line(line) {
                graph.direction = direction;
                continue;
            }

            if line.starts_with("classDef") {
                parse_class_def(line, &mut graph);
                continue;
            }

            if line.starts_with("class ") {
                parse_class_line(line, &mut graph);
                continue;
            }

            if line.starts_with("style ") {
                parse_style_line(line, &mut graph);
                continue;
            }

            if line == "}" {
                if let Some(ctx) = composite_stack.pop() {
                    if let Some(idx) = subgraph_stack.pop()
                        && idx != ctx.subgraph_idx
                    {
                        subgraph_stack.push(idx);
                    }
                    finalize_regions(ctx, &mut graph, &mut region_counter);
                }
                continue;
            }

            if line == "--" {
                if let Some(ctx) = composite_stack.last_mut() {
                    ctx.has_separator = true;
                    ctx.regions.push(Vec::new());
                    ctx.current_region = ctx.regions.len().saturating_sub(1);
                }
                continue;
            }

            if let Some((id, label, tail)) = parse_state_container_header(line) {
                if let Some(id) = id.clone() {
                    labels.insert(id.clone(), label.clone());
                }
                graph.subgraphs.push(Subgraph {
                    id: id.clone(),
                    label: label.clone(),
                    nodes: Vec::new(),
                    direction: None,
                    icon: None,
                });
                subgraph_stack.push(graph.subgraphs.len() - 1);
                composite_stack.push(CompositeContext {
                    subgraph_idx: graph.subgraphs.len() - 1,
                    regions: vec![Vec::new()],
                    current_region: 0,
                    has_separator: false,
                });

                if !tail.is_empty() {
                    if let Some(close_idx) = tail.find('}') {
                        let body = tail[..close_idx].trim();
                        let after = tail[close_idx + 1..].trim();
                        if !after.is_empty() {
                            pending.push_front(after.to_string());
                        }
                        pending.push_front("}".to_string());
                        if !body.is_empty() {
                            pending.push_front(body.to_string());
                        }
                    } else {
                        pending.push_front(tail);
                    }
                }
                continue;
            }

            if let Some((id, label, classes)) = parse_state_alias_line(line) {
                let label = label_override.clone().unwrap_or(label);
                labels.insert(id.clone(), label);
                graph.ensure_node(
                    &id,
                    Some(state_display_label(&id, &labels, &descriptions)),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }

            if let Some((left, meta, right, label)) = parse_state_transition(line) {
                // Determine current scope for start/end state tracking
                let scope = subgraph_stack
                    .last()
                    .and_then(|&idx| graph.subgraphs.get(idx))
                    .and_then(|sub| sub.id.clone())
                    .unwrap_or_else(|| "root".to_string());
                let (left_token, left_classes) = split_inline_classes(&left);
                let (right_token, right_classes) = split_inline_classes(&right);
                let (left_id, left_shape, left_label_override) = normalize_state_token(
                    &left_token,
                    true,
                    &mut start_states,
                    &mut end_states,
                    &scope,
                );
                let (right_id, right_shape, right_label_override) = normalize_state_token(
                    &right_token,
                    false,
                    &mut start_states,
                    &mut end_states,
                    &scope,
                );

                let left_label = left_label_override
                    .or_else(|| state_display_label_option(&left_id, &labels, &descriptions));
                let right_label = right_label_override
                    .or_else(|| state_display_label_option(&right_id, &labels, &descriptions));
                let left_shape = if left_shape == crate::ir::NodeShape::RoundRect
                    && graph.nodes.contains_key(&left_id)
                {
                    None
                } else {
                    Some(left_shape)
                };
                let right_shape = if right_shape == crate::ir::NodeShape::RoundRect
                    && graph.nodes.contains_key(&right_id)
                {
                    None
                } else {
                    Some(right_shape)
                };
                graph.ensure_node(&left_id, left_label, left_shape);
                graph.ensure_node(&right_id, right_label, right_shape);
                apply_node_classes(&mut graph, &left_id, &left_classes);
                apply_node_classes(&mut graph, &right_id, &right_classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &left_id);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &right_id);
                record_region_node(&mut composite_stack, &left_id);
                record_region_node(&mut composite_stack, &right_id);
                graph.edges.push(crate::ir::Edge {
                    from: left_id,
                    to: right_id,
                    label,
                    start_label: None,
                    end_label: None,
                    directed: meta.directed,
                    arrow_start: meta.arrow_start,
                    arrow_end: meta.arrow_end,
                    arrow_start_kind: meta.arrow_start_kind,
                    arrow_end_kind: meta.arrow_end_kind,
                    start_decoration: meta.start_decoration,
                    end_decoration: meta.end_decoration,
                    style: meta.style,
                });
                continue;
            }

            if let Some((id, label, classes)) = parse_state_description_line(line) {
                let label = label_override.clone().unwrap_or(label);
                descriptions.entry(id.clone()).or_default().push(label);
                graph.ensure_node(
                    &id,
                    Some(state_display_label(&id, &labels, &descriptions)),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }

            if let Some((position, target_raw, label)) = parse_state_note(line) {
                let (target, classes) = parse_state_id_with_classes(&target_raw);
                if target.is_empty() {
                    continue;
                }
                let shape = if graph.nodes.contains_key(&target) {
                    None
                } else {
                    Some(crate::ir::NodeShape::RoundRect)
                };
                graph.ensure_node(
                    &target,
                    state_display_label_option(&target, &labels, &descriptions),
                    shape,
                );
                apply_node_classes(&mut graph, &target, &classes);
                graph.state_notes.push(crate::ir::StateNote {
                    position,
                    target: target.clone(),
                    label,
                });
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &target);
                record_region_node(&mut composite_stack, &target);
                continue;
            }

            if let Some((id, classes)) = parse_state_simple(line) {
                if let Some(label) = label_override.clone() {
                    labels.insert(id.clone(), label);
                }
                graph.ensure_node(
                    &id,
                    state_display_label_option(&id, &labels, &descriptions),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }
        }
    }

    // Convert scoped [*] fan-out/fan-in nodes into fork/join bars.
    let mut outgoing_counts: HashMap<&str, usize> = HashMap::new();
    let mut incoming_counts: HashMap<&str, usize> = HashMap::new();
    for edge in &graph.edges {
        *outgoing_counts.entry(edge.from.as_str()).or_insert(0) += 1;
        *incoming_counts.entry(edge.to.as_str()).or_insert(0) += 1;
    }
    let fork_ids: Vec<String> = start_states
        .iter()
        .filter_map(|(scope, id)| {
            if scope == "root" {
                return None;
            }
            if outgoing_counts.get(id.as_str()).copied().unwrap_or(0) > 1 {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();
    let join_ids: Vec<String> = end_states
        .iter()
        .filter_map(|(scope, id)| {
            if scope == "root" {
                return None;
            }
            if incoming_counts.get(id.as_str()).copied().unwrap_or(0) > 1 {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();
    for id in fork_ids.into_iter().chain(join_ids) {
        if let Some(node) = graph.nodes.get_mut(&id) {
            node.shape = crate::ir::NodeShape::ForkJoin;
            node.label.clear();
        }
    }

    Ok(ParseOutput { graph })
}

fn parse_sequence_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Sequence;
    graph.direction = Direction::LeftRight;
    let lines = preprocess_input(input)?;

    let mut labels: HashMap<String, String> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut open_frames: Vec<usize> = Vec::new();
    let mut open_boxes: Vec<crate::ir::SequenceBox> = Vec::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("sequencediagram") {
            continue;
        }
        if let Some((id, label, shape)) = parse_sequence_participant(line) {
            if !order.contains(&id) {
                order.push(id.clone());
            }
            if let Some(label) = label.clone() {
                labels.insert(id.clone(), label);
            }
            ensure_sequence_node(&mut graph, &labels, &id, Some(shape));
            if let Some(box_ctx) = open_boxes.last_mut()
                && !box_ctx.participants.contains(&id)
            {
                box_ctx.participants.push(id.clone());
            }
            continue;
        }

        if let Some((color, label)) = parse_sequence_box_line(line) {
            open_boxes.push(crate::ir::SequenceBox {
                label,
                color,
                participants: Vec::new(),
            });
            continue;
        }

        if lower == "alt"
            || lower.starts_with("alt ")
            || lower == "opt"
            || lower.starts_with("opt ")
            || lower == "loop"
            || lower.starts_with("loop ")
            || lower == "par"
            || lower.starts_with("par ")
            || lower == "rect"
            || lower.starts_with("rect ")
            || lower == "critical"
            || lower.starts_with("critical ")
            || lower == "break"
            || lower.starts_with("break ")
        {
            let (kind, offset) = if lower.starts_with("opt") {
                (crate::ir::SequenceFrameKind::Opt, 3)
            } else if lower.starts_with("loop") {
                (crate::ir::SequenceFrameKind::Loop, 4)
            } else if lower.starts_with("par") {
                (crate::ir::SequenceFrameKind::Par, 3)
            } else if lower.starts_with("rect") {
                (crate::ir::SequenceFrameKind::Rect, 4)
            } else if lower.starts_with("critical") {
                (crate::ir::SequenceFrameKind::Critical, 8)
            } else if lower.starts_with("break") {
                (crate::ir::SequenceFrameKind::Break, 5)
            } else {
                (crate::ir::SequenceFrameKind::Alt, 3)
            };
            let label = line.get(offset..).map(str::trim).unwrap_or_default();
            let label = if label.is_empty() {
                None
            } else {
                Some(strip_quotes(label))
            };
            let start_idx = graph.edges.len();
            let k = graph.sequence_frames.len();
            open_frames.push(k);
            graph.sequence_events.push(crate::ir::SequenceEvent::FrameStart(k));
            graph.sequence_frames.push(crate::ir::SequenceFrame {
                kind,
                sections: vec![crate::ir::SequenceFrameSection {
                    label,
                    start_idx,
                    end_idx: start_idx,
                }],
                start_idx,
                end_idx: start_idx,
            });
            continue;
        }

        let section = [
            ("else", None),
            ("and", Some(crate::ir::SequenceFrameKind::Par)),
            ("option", Some(crate::ir::SequenceFrameKind::Critical)),
        ]
        .into_iter()
        .find(|(word, _)| lower == *word || lower.starts_with(&format!("{word} ")));
        if let Some((word, only)) = section {
            if let Some(&k) = open_frames.last()
                && only.is_none_or(|kind| graph.sequence_frames[k].kind == kind)
            {
                let split_idx = graph.edges.len();
                let frame = &mut graph.sequence_frames[k];
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = split_idx;
                }
                let label = line.get(word.len()..).map(str::trim).unwrap_or_default();
                frame.sections.push(crate::ir::SequenceFrameSection {
                    label: (!label.is_empty()).then(|| strip_quotes(label)),
                    start_idx: split_idx,
                    end_idx: split_idx,
                });
                graph.sequence_events.push(crate::ir::SequenceEvent::FrameSection(k));
            }
            continue;
        }

        if lower == "end" {
            if let Some(k) = open_frames.pop() {
                close_sequence_frame(&mut graph, k);
            } else if let Some(seq_box) = open_boxes.pop() {
                graph.sequence_boxes.push(seq_box);
            }
            continue;
        }

        if let Some((position, participants, label)) = parse_sequence_note(line) {
            for id in &participants {
                if !order.contains(id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, id, None);
            }
            graph.sequence_events.push(crate::ir::SequenceEvent::Note(graph.sequence_notes.len()));
            graph.sequence_notes.push(crate::ir::SequenceNote {
                position,
                participants,
                label,
                index: graph.edges.len(),
            });
            continue;
        }

        if lower.starts_with("activate ") {
            let id = line[9..].trim();
            if !id.is_empty() {
                let id = strip_quotes(id);
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, &id, None);
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant: id,
                        index: graph.edges.len(),
                        kind: crate::ir::SequenceActivationKind::Activate,
                    });
            }
            continue;
        }
        if lower.starts_with("deactivate ") {
            let id = line[11..].trim();
            if !id.is_empty() {
                let id = strip_quotes(id);
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, &id, None);
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant: id,
                        index: graph.edges.len(),
                        kind: crate::ir::SequenceActivationKind::Deactivate,
                    });
            }
            continue;
        }
        if lower.starts_with("autonumber") {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 2 {
                let token = parts[1].to_ascii_lowercase();
                if token == "off" || token == "stop" || token == "disable" {
                    graph.sequence_autonumber = None;
                } else if let Ok(start) = parts[1].parse::<usize>() {
                    graph.sequence_autonumber = Some(start);
                } else {
                    graph.sequence_autonumber = Some(1);
                }
            } else {
                graph.sequence_autonumber = Some(1);
            }
            continue;
        }

        if let Some((from, to, label, style, activation)) = parse_sequence_message(line) {
            if !order.contains(&from) {
                order.push(from.clone());
            }
            if !order.contains(&to) {
                order.push(to.clone());
            }
            ensure_sequence_node(&mut graph, &labels, &from, None);
            ensure_sequence_node(&mut graph, &labels, &to, None);
            let index = graph.edges.len();
            if let Some(kind) = activation {
                let participant = match kind {
                    crate::ir::SequenceActivationKind::Activate => to.clone(),
                    crate::ir::SequenceActivationKind::Deactivate => from.clone(),
                };
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant,
                        index,
                        kind,
                    });
            }
            graph.sequence_events.push(crate::ir::SequenceEvent::Message(index));
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label,
                start_label: None,
                end_label: None,
                directed: true,
                arrow_start: false,
                arrow_end: true,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                style,
            });
        }
    }

    while let Some(k) = open_frames.pop() {
        close_sequence_frame(&mut graph, k);
    }
    while let Some(seq_box) = open_boxes.pop() {
        graph.sequence_boxes.push(seq_box);
    }

    graph.sequence_participants = order;
    Ok(ParseOutput { graph })
}

fn close_sequence_frame(graph: &mut Graph, k: usize) {
    let end_idx = graph.edges.len();
    let frame = &mut graph.sequence_frames[k];
    if let Some(last) = frame.sections.last_mut() {
        last.end_idx = end_idx;
    }
    frame.end_idx = end_idx;
    graph.sequence_events.push(crate::ir::SequenceEvent::FrameEnd(k));
}

fn add_node_to_subgraph(graph: &mut Graph, idx: usize, node_id: &str) {
    if let Some(subgraph) = graph.subgraphs.get_mut(idx)
        && !subgraph.nodes.contains(&node_id.to_string())
    {
        subgraph.nodes.push(node_id.to_string());
    }
}

fn add_node_to_subgraphs(graph: &mut Graph, subgraph_stack: &[usize], node_id: &str) {
    if subgraph_stack.is_empty() {
        return;
    }
    // Mermaid-js parity (flowDb `makeUniq`/`exists`): a node belongs to the
    // first subgraph that claims it. Referencing the node later inside an
    // unrelated subgraph must not add it there too, otherwise sibling
    // subgraph node sets overlap and their boxes are forced to overlap.
    // Membership in the current stack chain (ancestors) is fine: that is how
    // nested subgraphs are represented here.
    let claimed_elsewhere = graph.subgraphs.iter().enumerate().any(|(idx, sub)| {
        !subgraph_stack.contains(&idx) && sub.nodes.iter().any(|n| n == node_id)
    });
    if claimed_elsewhere {
        return;
    }
    for idx in subgraph_stack {
        add_node_to_subgraph(graph, *idx, node_id);
    }
}

fn split_statements(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }

        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            current.push(ch);
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            current.push(ch);
            continue;
        }

        match ch {
            '[' | '(' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ']' | ')' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
                current.push(ch);
            }
            ';' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

fn strip_trailing_comment(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            out.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            out.push(ch);
            continue;
        }
        if ch == '%'
            && let Some('%') = chars.peek().copied()
        {
            break;
        }
        out.push(ch);
    }
    out.trim().to_string()
}

fn strip_trailing_comment_keep_indent(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            out.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            out.push(ch);
            continue;
        }
        if ch == '%'
            && let Some('%') = chars.peek().copied()
        {
            break;
        }
        out.push(ch);
    }
    out.trim_end().to_string()
}

fn extract_leading_decoration(right: &str) -> Option<(char, String)> {
    let mut chars = right.chars();
    let first = chars.next()?;
    if first != 'o' && first != 'x' {
        return None;
    }
    let rest: String = chars.collect();
    if rest.is_empty() {
        return None;
    }
    if rest
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false)
    {
        return Some((first, rest.trim_start().to_string()));
    }
    None
}

fn parse_subgraph_header(input: &str) -> (Option<String>, String, Vec<String>) {
    let (base, classes) = split_inline_classes(input);
    let trimmed = base.trim();
    if trimmed.is_empty() {
        return (None, "Subgraph".to_string(), classes);
    }

    if let Some((id, label, _shape)) = split_id_label(trimmed) {
        return (Some(id.to_string()), label, classes);
    }

    if !trimmed.contains(['"', '\'']) {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() == 1 {
            let token = parts[0];
            return (Some(token.to_string()), token.to_string(), classes);
        }
    }

    (None, strip_quotes(trimmed), classes)
}

fn parse_node_only(line: &str) -> Option<NodeTokenParts> {
    if line.contains("--") {
        return None;
    }
    let (id, label, shape, classes) = parse_node_token(line);
    if id.is_empty() {
        None
    } else {
        Some((id, label, shape, classes))
    }
}

/// Mask content inside brackets to prevent edge detection from matching dashes in labels.
/// Returns a string where characters inside [...], (...), {...}, "...", '...' are replaced with spaces.
fn mask_bracket_content(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut depth_square = 0;
    let mut depth_paren = 0;
    let mut depth_curly = 0;
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    let mut prev_char = '\0';

    for ch in line.chars() {
        let in_bracket = depth_square > 0 || depth_paren > 0 || depth_curly > 0;
        let in_quote = in_double_quote || in_single_quote;

        match ch {
            '[' if !in_quote => {
                depth_square += 1;
                result.push(ch);
            }
            ']' if !in_quote && depth_square > 0 => {
                depth_square -= 1;
                result.push(ch);
            }
            '(' if !in_quote && !in_bracket => {
                depth_paren += 1;
                result.push(ch);
            }
            ')' if !in_quote && depth_paren > 0 => {
                depth_paren -= 1;
                result.push(ch);
            }
            '{' if !in_quote && !in_bracket => {
                depth_curly += 1;
                result.push(ch);
            }
            '}' if !in_quote && depth_curly > 0 => {
                depth_curly -= 1;
                result.push(ch);
            }
            '"' if prev_char != '\\' => {
                in_double_quote = !in_double_quote;
                if in_bracket || in_quote {
                    // Preserve byte length by adding spaces equal to character's UTF-8 byte count
                    for _ in 0..ch.len_utf8() {
                        result.push(' ');
                    }
                } else {
                    result.push(ch);
                }
            }
            '\'' if prev_char != '\\' => {
                in_single_quote = !in_single_quote;
                if in_bracket || in_quote {
                    // Preserve byte length by adding spaces equal to character's UTF-8 byte count
                    for _ in 0..ch.len_utf8() {
                        result.push(' ');
                    }
                } else {
                    result.push(ch);
                }
            }
            _ => {
                if in_bracket || in_quote {
                    // Preserve byte length by adding spaces equal to character's UTF-8 byte count
                    for _ in 0..ch.len_utf8() {
                        result.push(' ');
                    }
                } else {
                    result.push(ch);
                }
            }
        }
        prev_char = ch;
    }
    result
}

/// Split `input` on `&` that appear outside brackets, parentheses, braces, and quotes.
///
/// Uses [`mask_bracket_content`] to blank out quoted/bracketed content while
/// preserving byte positions, then splits on `&` positions found in the masked
/// string but slices from the original — so `A["foo & bar"]` is never split.
fn split_on_ampersand(input: &str) -> Vec<&str> {
    let masked = mask_bracket_content(input);
    let mut parts = Vec::new();
    let mut start = 0usize;
    for (i, ch) in masked.char_indices() {
        if ch == '&' {
            let part = input[start..i].trim();
            if !part.is_empty() {
                parts.push(part);
            }
            start = i + ch.len_utf8();
        }
    }
    let last = input[start..].trim();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

fn split_edge_chain(line: &str) -> Option<Vec<String>> {
    let masked = mask_bracket_content(line);
    if PIPE_LABEL_RE.is_match(&masked)
        || QUOTED_LABEL_ARROW_RE.is_match(line)
        || LABEL_ARROW_RE.is_match(&masked)
        || COMPACT_DOTTED_LABEL_ARROW_RE.is_match(&masked)
    {
        return None;
    }

    let matches: Vec<regex::Match> = ARROW_TOKEN_RE.find_iter(&masked).collect();
    if matches.len() < 2 {
        return None;
    }

    let mut nodes: Vec<String> = Vec::with_capacity(matches.len() + 1);
    let mut arrows: Vec<String> = Vec::with_capacity(matches.len());
    let mut last_idx = 0usize;

    for m in matches {
        nodes.push(line[last_idx..m.start()].trim().to_string());
        arrows.push(line[m.start()..m.end()].trim().to_string());
        last_idx = m.end();
    }
    nodes.push(line[last_idx..].trim().to_string());

    if nodes.len() != arrows.len() + 1 {
        return None;
    }

    // Attach leading pipe labels to the preceding arrow and strip them from the node token.
    for i in 1..nodes.len() {
        let trimmed = nodes[i].trim_start();
        if let Some(stripped) = trimmed.strip_prefix('|')
            && let Some(end_idx) = stripped.find('|')
        {
            let label_len = end_idx + 2;
            let label = &trimmed[..label_len];
            let rest = trimmed[label_len..].trim_start();
            arrows[i - 1].push_str(label);
            nodes[i] = rest.to_string();
        }
    }

    if nodes.iter().any(|node| node.is_empty()) {
        return None;
    }

    let mut statements = Vec::with_capacity(arrows.len());
    for i in 0..arrows.len() {
        statements.push(format!("{} {} {}", nodes[i], arrows[i], nodes[i + 1]));
    }
    Some(statements)
}

fn parse_edge_line(line: &str) -> Option<(String, Option<String>, String, EdgeMeta)> {
    // Mask bracket content to prevent matching dashes inside labels like A[wi-fi]
    let masked = mask_bracket_content(line);

    // Helper to extract from original line using match positions from masked line
    let extract = |m: regex::Match| -> &str { &line[m.start()..m.end()] };

    if let Some(caps) = PIPE_LABEL_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let arrow_match = caps.name("arrow")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_clean = extract(label_match).trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let arrow = extract(arrow_match).trim();
            let edge_meta = parse_edge_meta(arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    // Quoted label syntax: -- "text" --> (match on original line, not masked,
    // because mask_bracket_content blanks quoted content).
    if let Some(caps) = QUOTED_LABEL_ARROW_RE.captures(line) {
        let left = caps.name("left")?.as_str().trim();
        let right = caps.name("right")?.as_str().trim();
        let label_clean = caps.name("label")?.as_str().trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            let dash1 = caps.name("dash1")?.as_str();
            let dash2 = caps.name("dash2")?.as_str();
            let end = caps.name("end").map(|m| m.as_str()).unwrap_or("");
            let arrow = format!("{}{}{}{}", start, dash1, dash2, end);
            let edge_meta = parse_edge_meta(&arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    if let Some(caps) = LABEL_ARROW_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_raw = extract(label_match).trim();
        let label_clean = label_raw.trim_matches('|').trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            let dash1 = caps.name("dash1")?.as_str();
            let dash2 = caps.name("dash2")?.as_str();
            let end = caps.name("end").map(|m| m.as_str()).unwrap_or("");
            let arrow = format!("{}{}{}{}", start, dash1, dash2, end);
            let edge_meta = parse_edge_meta(&arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    if let Some(caps) = COMPACT_DOTTED_LABEL_ARROW_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_clean = extract(label_match).trim().trim_matches('.');
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            let dash1 = caps.name("dash1")?.as_str();
            let dash2 = caps.name("dash2")?.as_str();
            let end = caps.name("end").map(|m| m.as_str()).unwrap_or("");
            let arrow = format!("{}{}.{}{}", start, dash1, dash2, end);
            let edge_meta = parse_edge_meta(&arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    let caps = ARROW_RE.captures(&masked)?;
    let left_match = caps.name("left")?;
    let right_match = caps.name("right")?;
    let left = extract(left_match).trim();
    let mut arrow = caps.name("arrow")?.as_str().trim().to_string();
    let mut right = extract(right_match).trim().to_string();

    if let Some((dec, rest)) = extract_leading_decoration(&right) {
        arrow.push(dec);
        right = rest;
    }

    if left.is_empty() || right.is_empty() || arrow.is_empty() {
        return None;
    }

    let (label, right_token) = if let Some(stripped) = right.strip_prefix('|') {
        if let Some(end) = stripped.find('|') {
            let label = stripped[..end].trim().to_string();
            let rest = stripped[end + 1..].trim();
            (Some(label), rest)
        } else {
            (None, right.as_str())
        }
    } else {
        (None, right.as_str())
    };

    if right_token.is_empty() {
        return None;
    }

    let edge_meta = parse_edge_meta(&arrow);
    Some((left.to_string(), label, right_token.to_string(), edge_meta))
}

#[derive(Debug, Clone, Copy)]
struct EdgeMeta {
    directed: bool,
    arrow_start: bool,
    arrow_end: bool,
    arrow_start_kind: Option<crate::ir::EdgeArrowhead>,
    arrow_end_kind: Option<crate::ir::EdgeArrowhead>,
    start_decoration: Option<crate::ir::EdgeDecoration>,
    end_decoration: Option<crate::ir::EdgeDecoration>,
    style: crate::ir::EdgeStyle,
}

fn parse_edge_meta(arrow: &str) -> EdgeMeta {
    let mut trimmed = arrow.trim().to_string();
    let mut start_decoration = None;
    let mut end_decoration = None;

    if trimmed.starts_with('o') {
        start_decoration = Some(crate::ir::EdgeDecoration::Circle);
        trimmed.remove(0);
    } else if trimmed.starts_with('x') {
        start_decoration = Some(crate::ir::EdgeDecoration::Cross);
        trimmed.remove(0);
    }

    if trimmed.ends_with('o') {
        end_decoration = Some(crate::ir::EdgeDecoration::Circle);
        trimmed.pop();
    } else if trimmed.ends_with('x') {
        end_decoration = Some(crate::ir::EdgeDecoration::Cross);
        trimmed.pop();
    }

    let arrow_start = trimmed.starts_with('<');
    let arrow_end = trimmed.ends_with('>');

    let style = if trimmed.contains('=') {
        crate::ir::EdgeStyle::Thick
    } else if trimmed.contains('.') {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };

    let directed = arrow_start || arrow_end;

    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind: None,
        arrow_end_kind: None,
        start_decoration,
        end_decoration,
        style,
    }
}

fn parse_direction_line(line: &str) -> Option<Direction> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() == 2 && parts[0] == "direction" {
        return Direction::from_token(parts[1]);
    }
    None
}

fn parse_class_def(line: &str, graph: &mut Graph) {
    let trimmed = line.trim();
    let mut parts = trimmed.splitn(3, char::is_whitespace);
    let _ = parts.next();
    let class_name = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();
    if class_name.is_empty() || rest.is_empty() {
        return;
    }
    let style = parse_node_style(rest);
    graph.class_defs.insert(class_name.to_string(), style);
}

fn parse_class_line(line: &str, graph: &mut Graph) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return;
    }
    let class_name = parts
        .last()
        .expect("parts.len() >= 3 checked above")
        .to_string();
    let class_names: Vec<String> = class_name
        .split(',')
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    if class_names.is_empty() {
        return;
    }
    let nodes_raw = parts[1..parts.len() - 1].join(" ");
    for node_id in nodes_raw.split(',') {
        let id = node_id.trim();
        if id.is_empty() {
            continue;
        }
        for class_name in &class_names {
            graph
                .node_classes
                .entry(id.to_string())
                .or_default()
                .push(class_name.clone());
            graph
                .subgraph_classes
                .entry(id.to_string())
                .or_default()
                .push(class_name.clone());
        }
    }
}

fn apply_node_classes(graph: &mut Graph, node_id: &str, classes: &[String]) {
    for class_name in classes {
        if class_name.is_empty() {
            continue;
        }
        graph
            .node_classes
            .entry(node_id.to_string())
            .or_default()
            .push(class_name.clone());
    }
}

fn apply_subgraph_classes(graph: &mut Graph, subgraph_id: &str, classes: &[String]) {
    for class_name in classes {
        if class_name.is_empty() {
            continue;
        }
        graph
            .subgraph_classes
            .entry(subgraph_id.to_string())
            .or_default()
            .push(class_name.clone());
    }
}

fn parse_style_line(line: &str, graph: &mut Graph) {
    let mut parts = line.splitn(3, ' ');
    let _ = parts.next();
    let node_id = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();
    if node_id.is_empty() || rest.is_empty() {
        return;
    }
    let style = parse_node_style(rest);
    for raw in node_id.split(',') {
        let id = raw.trim();
        if id.is_empty() {
            continue;
        }
        graph.node_styles.insert(id.to_string(), style.clone());
        graph.subgraph_styles.insert(id.to_string(), style.clone());
    }
}

fn parse_link_style_line(line: &str, graph: &mut Graph) {
    let trimmed = line.trim();
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() < 3 {
        return;
    }

    let mut style_idx = None;
    for (idx, token) in tokens.iter().enumerate().skip(1) {
        if token.contains(':') {
            style_idx = Some(idx);
            break;
        }
    }
    let Some(style_idx) = style_idx else {
        return;
    };
    let index_tokens = &tokens[1..style_idx];
    let style_str = tokens[style_idx..].join(" ");
    if style_str.is_empty() {
        return;
    }

    let style = parse_edge_style(&style_str);
    if index_tokens.len() == 1 && index_tokens[0] == "default" {
        graph.edge_style_default = Some(style);
        return;
    }

    for raw in index_tokens.iter().flat_map(|token| token.split(',')) {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if let Ok(index) = token.parse::<usize>() {
            graph.edge_styles.insert(index, style.clone());
        }
    }
}

fn tokenize_quoted(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_click_line(line: &str) -> Option<(String, crate::ir::NodeLink)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let keyword_len = if lower.starts_with("click ") {
        5
    } else if lower.starts_with("link ") {
        4
    } else {
        return None;
    };
    let rest = trimmed[keyword_len..].trim();
    if rest.starts_with(['-', '=', '.', '<', '&', '~']) {
        return None;
    }
    let tokens = tokenize_quoted(rest);
    if tokens.len() < 2 {
        return None;
    }
    let id = tokens[0].clone();
    let mut idx = 1usize;
    if tokens[idx].eq_ignore_ascii_case("call") {
        return None;
    }
    if tokens[idx].eq_ignore_ascii_case("href") {
        idx += 1;
    }
    let url = tokens.get(idx)?.clone();
    idx += 1;
    let mut title = None;
    let mut target = None;
    if let Some(token) = tokens.get(idx) {
        if token.starts_with('_') {
            target = Some(token.clone());
            idx += 1;
        } else {
            title = Some(token.clone());
            idx += 1;
        }
    }
    if target.is_none()
        && let Some(token) = tokens.get(idx)
        && token.starts_with('_')
    {
        target = Some(token.clone());
    }

    Some((id, crate::ir::NodeLink { url, title, target }))
}

fn parse_node_style(input: &str) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();
    for part in input.split(',') {
        let mut kv = part.splitn(2, ':');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        match key {
            "fill" => style.fill = Some(value.to_string()),
            "stroke" => style.stroke = Some(value.to_string()),
            "stroke-width" => {
                let width = value.trim_end_matches("px").parse::<f32>().ok();
                style.stroke_width = width;
            }
            "stroke-dasharray" => style.stroke_dasharray = Some(value.to_string()),
            "color" => style.text_color = Some(value.to_string()),
            _ => {}
        }
    }
    style
}

fn parse_edge_style(input: &str) -> crate::ir::EdgeStyleOverride {
    let mut style = crate::ir::EdgeStyleOverride::default();
    for part in input.split(',') {
        let mut kv = part.splitn(2, ':');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        match key {
            "stroke" => style.stroke = Some(value.to_string()),
            "stroke-width" => {
                style.stroke_width = value.trim_end_matches("px").parse::<f32>().ok();
            }
            "stroke-dasharray" => style.dasharray = Some(value.to_string()),
            "color" => style.label_color = Some(value.to_string()),
            _ => {}
        }
    }
    style
}

fn parse_node_token(
    token: &str,
) -> (
    String,
    Option<String>,
    Option<crate::ir::NodeShape>,
    Vec<String>,
) {
    let (base, classes) = split_inline_classes(token);
    let trimmed = base.trim();
    if let Some((id, label, shape)) = split_asymmetric_label(trimmed) {
        return (id, Some(label), Some(shape), classes);
    }
    if let Some((id, label, shape)) = split_id_label(trimmed) {
        return (id.to_string(), Some(label), Some(shape), classes);
    }

    let id = trimmed.split_whitespace().next().unwrap_or("").to_string();
    (id, None, None, classes)
}

fn split_asymmetric_label(token: &str) -> Option<(String, String, crate::ir::NodeShape)> {
    let trimmed = token.trim();
    if trimmed.contains('[') {
        return None;
    }
    let Some(pos) = trimmed.find('>') else {
        return None;
    };
    if !trimmed.ends_with(']') {
        return None;
    }
    let id = trimmed[..pos].trim();
    if id.is_empty() {
        return None;
    }
    let label = trimmed[pos + 1..trimmed.len() - 1].trim();
    if label.is_empty() {
        return None;
    }
    Some((
        id.to_string(),
        strip_quotes(label),
        crate::ir::NodeShape::Asymmetric,
    ))
}

fn split_inline_classes(token: &str) -> (String, Vec<String>) {
    let mut parts = token.split(":::");
    let base = parts.next().unwrap_or("").trim().to_string();
    let classes = parts
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    (base, classes)
}

fn split_id_label(token: &str) -> Option<(&str, String, crate::ir::NodeShape)> {
    if let Some(start) = token.find('[')
        && token.ends_with(']')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape) = parse_shape_from_brackets(raw);
            return Some((id, label, shape));
        }
    }

    if let Some(start) = token.find('(')
        && token.ends_with(')')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape) = parse_shape_from_parens(raw);
            return Some((id, label, shape));
        }
    }

    if let Some(start) = token.find('{')
        && token.ends_with('}')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape) = parse_shape_from_braces(raw);
            return Some((id, label, shape));
        }
    }

    None
}

fn parse_shape_from_brackets(raw: &str) -> (String, crate::ir::NodeShape) {
    let trimmed = raw.trim();
    if trimmed.starts_with("[/") && trimmed.ends_with("/]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Parallelogram,
        );
    }
    if trimmed.starts_with("[\\") && trimmed.ends_with("\\]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::ParallelogramAlt,
        );
    }
    if trimmed.starts_with("[/") && trimmed.ends_with("\\]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Trapezoid,
        );
    }
    if trimmed.starts_with("[\\") && trimmed.ends_with("/]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::TrapezoidAlt,
        );
    }
    if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Subroutine,
        );
    }
    if trimmed.starts_with("[(") && trimmed.ends_with(")]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Cylinder,
        );
    }
    if trimmed.starts_with("[") && trimmed.ends_with("]") {
        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.starts_with('(') && inner.ends_with(')') {
            return (
                strip_quotes(&inner[1..inner.len() - 1]),
                crate::ir::NodeShape::Stadium,
            );
        }
        return (strip_quotes(inner), crate::ir::NodeShape::Rectangle);
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::Rectangle)
}

fn parse_shape_from_parens(raw: &str) -> (String, crate::ir::NodeShape) {
    let trimmed = raw.trim();
    if trimmed.starts_with("(((") && trimmed.ends_with(")))") {
        return (
            strip_quotes(&trimmed[3..trimmed.len() - 3]),
            crate::ir::NodeShape::DoubleCircle,
        );
    }
    if trimmed.starts_with("((") && trimmed.ends_with("))") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::DoubleCircle,
        );
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.starts_with('[') && inner.ends_with(']') {
            return (
                strip_quotes(&inner[1..inner.len() - 1]),
                crate::ir::NodeShape::Stadium,
            );
        }
        return (strip_quotes(inner), crate::ir::NodeShape::RoundRect);
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::RoundRect)
}

fn parse_shape_from_braces(raw: &str) -> (String, crate::ir::NodeShape) {
    let trimmed = raw.trim();
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Hexagon,
        );
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return (
            strip_quotes(&trimmed[1..trimmed.len() - 1]),
            crate::ir::NodeShape::Diamond,
        );
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::Diamond)
}

fn strip_quotes(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn count_indent(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += 2,
            _ => break,
        }
    }
    count
}
