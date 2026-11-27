use std::collections::BTreeMap;

use masonry::kurbo::{Point, Rect};
use mermaid_rs_renderer::{Edge, NodeShape};

use super::canvas::Text;
use super::shape::{Node, Side};

pub(super) struct Placed {
    pub rect: Rect,
    pub shape: NodeShape,
    pub label: Text,
    pub hidden: bool,
}

#[derive(Clone)]
pub(super) struct Wire {
    pub edge: Edge,
    pub points: Vec<Point>,
    pub label: Option<Text>,
    pub anchor: Option<Point>,
    pub sides: Option<[Side; 2]>,
}

impl Wire {
    pub(super) fn between(edge: Edge, label: Option<Text>, from: Node, to: Node, sides: [Side; 2]) -> Self {
        Wire { edge, points: vec![from.port(sides[0], 0.5), to.port(sides[1], 0.5)], label, anchor: None, sides: Some(sides) }
    }
}

pub(super) struct Frame {
    pub rect: Rect,
    pub title: Text,
}

#[derive(Default)]
pub(super) struct Scene {
    pub nodes: BTreeMap<String, Placed>,
    pub wires: Vec<Wire>,
    pub frames: Vec<Frame>,
    pub notes: Vec<(Rect, String)>,
}
