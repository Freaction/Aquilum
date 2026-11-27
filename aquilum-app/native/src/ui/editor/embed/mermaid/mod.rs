mod architecture;
mod arrow;
mod block;
mod c4;
mod canvas;
mod columns;
#[cfg(test)]
mod check;
mod gantt;
mod git;
mod graph;
mod journey;
mod kanban;
mod layout;
mod packet;
mod pie;
mod quadrant;
mod radar;
mod requirement;
mod sankey;
mod scene;
mod sequence;
mod shape;
#[cfg(test)]
mod tests;
mod timeline;
mod treemap;
mod xychart;
mod zenuml;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::sync::Once;

use masonry::core::BrushIndex;
use masonry::parley::Layout;
use mermaid_rs_renderer::{DiagramKind, Edge, Graph, parse_mermaid};

use super::panel::Tone as Hue;
use super::{Drawing, Spec};
use canvas::Canvas;
use scene::Scene;

const KEEP: usize = 16;

pub(super) const PALETTE: [Hue; 6] = [Hue::Accent, Hue::Info, Hue::Success, Hue::Warning, Hue::Idea, Hue::Danger];

pub(super) fn hue(i: usize) -> Hue {
    PALETTE[i % PALETTE.len()]
}

pub(super) fn titled<'a>(lines: &[&'a str]) -> (Option<&'a str>, Vec<&'a str>) {
    let (heads, body): (Vec<&str>, Vec<&str>) = lines.iter().map(|l| l.trim()).partition(|l| l.starts_with("title "));
    (heads.last().map(|l| unquote(&l["title ".len()..])), body)
}

pub(super) fn unquote(text: &str) -> &str {
    text.trim().trim_matches('"').trim()
}

pub(super) fn split(list: &str) -> Vec<&str> {
    let (mut parts, mut depth, mut quoted, mut start) = (Vec::new(), 0i32, false, 0);
    for (i, c) in list.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '[' | '{' | '(' if !quoted => depth += 1,
            ']' | '}' | ')' if !quoted => depth -= 1,
            ',' if depth == 0 && !quoted => {
                parts.push(&list[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&list[start..]);
    parts.into_iter().map(str::trim).filter(|p| !p.is_empty()).collect()
}

pub(super) fn number(value: f64) -> String {
    format!("{value:.2}").trim_end_matches('0').trim_end_matches('.').to_owned()
}

struct Laid {
    graph: Graph,
    scene: Option<Scene>,
    loops: Vec<Edge>,
}

thread_local! {
    static LAID: RefCell<VecDeque<(u64, Option<Rc<Laid>>)>> = const { RefCell::new(VecDeque::new()) };
}

fn lines(source: &str) -> Vec<&str> {
    let mut front = false;
    source
        .lines()
        .map(str::trim_end)
        .filter(|l| {
            let t = l.trim();
            front ^= t == "---";
            !front && !t.is_empty() && t != "---" && !t.starts_with("%%")
        })
        .collect()
}

#[cfg(test)]
fn built(source: &str) -> Option<(Graph, Scene)> {
    let lines = lines(source);
    let head = lines.first()?.trim().to_lowercase();
    let body = &lines[1..];
    let laid = |graph: Graph| {
        let scene = layout::lay(&graph, &[], 1.0);
        (graph, scene)
    };
    match head.split(|c: char| c.is_whitespace() || c == ':').next()? {
        "requirementdiagram" => Some(laid(requirement::graph(body))),
        "c4context" | "c4container" | "c4component" | "c4dynamic" | "c4deployment" => Some(laid(c4::graph(body))),
        "architecture-beta" | "architecture" => architecture::scene(body),
        _ => None,
    }
}

fn own(head: &str) -> Option<fn(&[&str], &mut Canvas)> {
    Some(match head.split(|c: char| c.is_whitespace() || c == ':').next()? {
        "timeline" => timeline::draw,
        "gitgraph" if head.contains("tb") => git::down,
        "gitgraph" if head.contains("bt") => git::up,
        "gitgraph" => git::draw,
        "kanban" => kanban::draw,
        "quadrantchart" => quadrant::draw,
        "xychart-beta" | "xychart" if head.contains("horizontal") => xychart::across,
        "xychart-beta" | "xychart" => xychart::draw,
        "journey" => journey::draw,
        "packet-beta" | "packet" => packet::draw,
        "radar-beta" | "radar" => radar::draw,
        "treemap-beta" | "treemap" => treemap::draw,
        "block-beta" | "block" => block::draw,
        "requirementdiagram" => requirement::draw,
        "c4context" | "c4container" | "c4component" | "c4dynamic" | "c4deployment" => c4::draw,
        "architecture-beta" | "architecture" => architecture::draw,
        "zenuml" => zenuml::draw,
        "sankey-beta" | "sankey" => sankey::draw,
        _ => return None,
    })
}

pub fn mermaid(source: &str, available: f64, make: &mut dyn FnMut(&Spec) -> Layout<BrushIndex>) -> Option<Drawing> {
    let lines = lines(source);
    let head = lines.first()?.trim().to_lowercase();
    let mut canvas = Canvas::default();
    if let Some(draw) = own(&head) {
        draw(&lines[1..], &mut canvas);
    } else {
        let laid = laid(source)?;
        match laid.graph.kind {
            DiagramKind::Sequence => sequence::draw(&laid.graph, &mut canvas),
            DiagramKind::Pie => pie::draw(&laid.graph, available - super::panel::PAD * 2.0, &mut canvas),
            DiagramKind::Gantt => gantt::draw(&laid.graph, &mut canvas),
            _ => graph::draw(&laid.graph, laid.scene.as_ref()?, &laid.loops, &mut canvas),
        }
    }
    canvas.finish(available, make)
}

fn laid(source: &str) -> Option<Rc<Laid>> {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    let key = hasher.finish();
    LAID.with_borrow_mut(|cache| {
        let entry = match cache.iter().position(|(k, _)| *k == key) {
            Some(i) => cache.remove(i)?,
            None => (key, compute(source).map(Rc::new)),
        };
        if cache.len() >= KEEP {
            cache.pop_front();
        }
        cache.push_back(entry);
        cache.back().and_then(|(_, laid)| laid.clone())
    })
}

fn compute(source: &str) -> Option<Laid> {
    static FONTS: Once = Once::new();
    FONTS.call_once(|| mermaid_rs_renderer::use_fonts(&crate::ui::theme::inter_upright()));
    let mut graph = parse_mermaid(source).ok()?.graph;
    if matches!(graph.kind, DiagramKind::Sequence | DiagramKind::Pie | DiagramKind::Gantt) {
        return Some(Laid { graph, scene: None, loops: Vec::new() });
    }
    if !matches!(graph.kind, DiagramKind::Flowchart | DiagramKind::Class | DiagramKind::State | DiagramKind::Er | DiagramKind::Mindmap) {
        return None;
    }
    let loops: Vec<Edge> = graph.edges.extract_if(.., |e| e.from == e.to).collect();
    let scene = match graph.kind {
        DiagramKind::Mindmap => layout::tree(&graph),
        _ => layout::lay(&graph, &loops, if graph.kind == DiagramKind::Er { 1.5 } else { 1.0 }),
    };
    Some(Laid { graph, scene: Some(scene), loops })
}
