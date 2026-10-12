use crate::reader::dom::{Dom, Kind, NodeId, utf16_len};
use super::{Cfi, Part, Path};

#[derive(Clone, Debug, PartialEq)]
enum Slot {
    Before,
    After,
    First,
    Last,
    Gap,
    Element(NodeId),
    Chunk(Vec<NodeId>),
}

pub(super) fn is_text(dom: &Dom, id: NodeId) -> bool {
    matches!(dom.node(id).kind, Kind::Text(_))
}

fn index_children(dom: &Dom, node: NodeId) -> Vec<Slot> {
    let mut slots: Vec<Slot> = Vec::new();
    for &child in &dom.node(node).children {
        let text = is_text(dom, child);
        match slots.last_mut() {
            None => slots.push(if text { Slot::Chunk(vec![child]) } else { Slot::Element(child) }),
            Some(Slot::Chunk(chunk)) if text => chunk.push(child),
            Some(_) if text => slots.push(Slot::Chunk(vec![child])),
            Some(Slot::Element(_)) => {
                slots.push(Slot::Gap);
                slots.push(Slot::Element(child));
            }
            Some(_) => slots.push(Slot::Element(child)),
        }
    }
    if matches!(slots.first(), Some(Slot::Element(_))) {
        slots.insert(0, Slot::First);
    }
    if matches!(slots.last(), Some(Slot::Element(_))) {
        slots.push(Slot::Last);
    }
    slots.insert(0, Slot::Before);
    slots.push(Slot::After);
    slots
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Point {
    At(NodeId, u32),
    Before(NodeId),
    After(NodeId),
}

fn locate(dom: &Dom, root: NodeId, path: &Path) -> Option<Point> {
    let last = path.last()?;
    if let Some(element) = last.id.as_deref().and_then(|id| dom.by_id(id)) {
        return Some(Point::At(element, 0));
    }
    let mut node = root;
    let mut chunk: Option<Vec<NodeId>> = None;
    for part in path {
        if chunk.is_some() {
            return None;
        }
        let slots = index_children(dom, node);
        match slots.get(part.index as usize)? {
            Slot::First => return Some(Point::At(dom.node(node).children.first().copied().unwrap_or(node), 0)),
            Slot::Last => return Some(Point::At(dom.node(node).children.last().copied().unwrap_or(node), 0)),
            Slot::Before => return Some(Point::Before(node)),
            Slot::After => return Some(Point::After(node)),
            Slot::Gap => return None,
            Slot::Element(element) => node = *element,
            Slot::Chunk(texts) => chunk = Some(texts.clone()),
        }
    }
    let offset = last.offset.unwrap_or(0);
    let Some(texts) = chunk else { return Some(Point::At(node, offset)) };
    let mut sum = 0;
    for text in texts {
        let length = utf16_len(dom.text_of(text).unwrap_or_default());
        if sum + length >= offset {
            return Some(Point::At(text, offset - sum));
        }
        sum += length;
    }
    None
}

pub fn resolve(dom: &Dom, local: &Cfi) -> Option<(Point, Point)> {
    let start = local.collapse(false);
    let end = local.collapse(true);
    Some((locate(dom, Dom::ROOT, start.first()?)?, locate(dom, Dom::ROOT, end.first()?)?))
}

fn path_to(dom: &Dom, node: NodeId, offset: Option<u32>) -> Path {
    let Some(parent) = dom.node(node).parent else { return Vec::new() };
    let slots = index_children(dom, parent);
    let found = slots.iter().position(|slot| match slot {
        Slot::Element(e) => *e == node,
        Slot::Chunk(texts) => texts.contains(&node),
        _ => false,
    });
    let mut offset = offset;
    if let Some(Slot::Chunk(texts)) = found.map(|i| &slots[i]) {
        let before: u32 = texts.iter().take_while(|&&t| t != node).map(|&t| utf16_len(dom.text_of(t).unwrap_or_default())).sum();
        offset = offset.map(|o| o + before);
    }
    let mut path = if parent != Dom::ROOT { path_to(dom, parent, None) } else { Vec::new() };
    if let Some(index) = found {
        path.push(Part { index: index as u32, id: dom.id(node).map(str::to_owned), offset, ..Part::default() });
    }
    path
}

pub fn from_range(dom: &Dom, start: (NodeId, u32), end: (NodeId, u32)) -> Cfi {
    let from = vec![path_to(dom, start.0, Some(start.1))];
    if start == end {
        return Cfi::Point(from);
    }
    Cfi::range(&from, &vec![path_to(dom, end.0, Some(end.1))])
}

pub fn element_path(dom: &Dom, node: NodeId) -> Path {
    path_to(dom, node, None)
}
