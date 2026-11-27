use std::collections::HashMap;

use masonry::kurbo::Vec2;
use mermaid_rs_renderer::Edge;

use super::super::super::super::markdown::MUTED;
use super::super::canvas::{Label, width};
use super::super::scene::Wire;
use super::super::shape::Node;
use super::{Arrow, head};
use crate::ui::tokens::size;

const NOTE: f64 = 12.0;

fn notes(edge: &Edge) -> [Option<String>; 2] {
    [(&edge.start_label, edge.start_decoration), (&edge.end_label, edge.end_decoration)].map(|(text, mark)| text.clone().or_else(|| head::cardinality(mark).map(str::to_owned)))
}

pub(in super::super) fn marked(edge: &Edge) -> [bool; 2] {
    notes(edge).map(|note| note.is_some())
}

pub(in super::super) fn marks(wire: &Wire, arrow: &Arrow, nodes: &HashMap<&str, Node>) -> Vec<Label> {
    let edge = &wire.edge;
    let looped = edge.from == edge.to;
    let mut labels = Vec::new();
    for (k, (text, id)) in notes(edge).into_iter().zip([&edge.from, &edge.to]).enumerate() {
        let Some(text) = text else { continue };
        let ((tip, dir), other) = (arrow.tips[k], arrow.tips[1 - k].0);
        let side = Vec2::new(-dir.y, dir.x);
        let off = nodes.get(id.as_str()).filter(|_| !looped).map_or(0.0, |n| side.dot(tip - n.rect.center()));
        let toward = side * if off.abs() > 1.0 { off.signum() } else { -side.dot(other - tip).signum() };
        let label = Label { width: width(&text, NOTE), ..Label::new(tip, text, NOTE, MUTED) };
        labels.push(if looped {
            let (out, away) = (-dir, (tip - other).normalize());
            let flat = out.x.abs() > 0.5;
            let (off, aside) = if flat { (size::GAP_SM, NOTE * 0.75) } else { (NOTE * 0.75 + size::GAP_SM, size::GAP_SM) };
            Label { at: tip + out * off + away * aside, ..label.beside(if flat { out.x } else { away.x }) }
        } else if toward.x.abs() > 0.5 {
            Label { at: tip - dir * NOTE + toward * size::GAP_LG, ..label.beside(toward.x) }
        } else {
            Label { at: tip - dir * size::GAP_SM + toward * (NOTE * 0.75 + size::GAP_SM), ..label.beside(-dir.x) }
        });
    }
    labels
}
