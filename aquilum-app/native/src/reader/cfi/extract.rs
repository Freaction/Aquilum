use crate::reader::dom::{Dom, NodeId};
use super::locate::{Point, is_text};

pub fn order(dom: &Dom) -> (Vec<u32>, Vec<u32>) {
    let mut enter = vec![0; dom.len()];
    let mut exit = vec![0; dom.len()];
    let mut counter = 0;
    let mut stack = vec![(Dom::ROOT, false)];
    while let Some((node, done)) = stack.pop() {
        if done {
            exit[node as usize] = counter;
            continue;
        }
        enter[node as usize] = counter;
        counter += 1;
        stack.push((node, true));
        for &child in dom.node(node).children.iter().rev() {
            stack.push((child, false));
        }
    }
    (enter, exit)
}

#[cfg_attr(not(test), allow(dead_code))]
fn boundary(dom: &Dom, enter: &[u32], exit: &[u32], point: Point) -> (u32, u32) {
    match point {
        Point::Before(node) => (enter[node as usize], 0),
        Point::After(node) => (exit[node as usize], 0),
        Point::At(node, offset) if is_text(dom, node) => (enter[node as usize], offset),
        Point::At(node, offset) => match dom.node(node).children.get(offset as usize) {
            Some(&child) => (enter[child as usize], 0),
            None => (exit[node as usize], 0),
        },
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn text(dom: &Dom, start: Point, end: Point) -> String {
    let (enter, exit) = order(dom);
    let (from, to) = (boundary(dom, &enter, &exit, start), boundary(dom, &enter, &exit, end));
    let mut texts: Vec<NodeId> = (0..dom.len() as NodeId).filter(|&n| is_text(dom, n)).collect();
    texts.sort_by_key(|&n| enter[n as usize]);
    let mut out: Vec<u16> = Vec::new();
    for node in texts {
        let at = enter[node as usize];
        if at < from.0 || at > to.0 {
            continue;
        }
        let units: Vec<u16> = dom.text_of(node).unwrap_or_default().encode_utf16().collect();
        let begin = if at == from.0 { (from.1 as usize).min(units.len()) } else { 0 };
        let finish = if at == to.0 { (to.1 as usize).min(units.len()) } else { units.len() };
        if at == to.0 && to.1 == 0 && !matches!(end, Point::At(n, _) if n == node) {
            continue;
        }
        out.extend_from_slice(&units[begin..finish.max(begin)]);
    }
    String::from_utf16_lossy(&out)
}
