use masonry::kurbo::Rect;
use mermaid_rs_renderer::{Graph, SequenceEvent, SequenceFrame, SequenceFrameKind, SequenceFrameSection, SequenceNotePosition};

use super::super::canvas::{self, BOLD, FONT, LINE, Text, width};
use crate::i18n::t;
use crate::ui::tokens::size;

pub(super) const INSET: f64 = 12.0;
pub(super) const LOOP: f64 = 28.0;
pub(super) const TAB: f64 = 24.0;
const GAP: f64 = 16.0;
const SPAN: f64 = 100.0;
const WRAP: f64 = 200.0;
const STEP: f64 = 20.0;

pub(super) struct Column {
    pub name: Text,
    pub width: f64,
    pub x: f64,
}

pub(super) struct Plan {
    pub columns: Vec<Column>,
    pub head: f64,
    pub messages: Vec<(f64, Text)>,
    pub notes: Vec<(Rect, String)>,
    pub frames: Vec<(usize, Rect, Vec<f64>)>,
    pub bottom: f64,
}

pub(super) fn words(kind: SequenceFrameKind) -> Option<(&'static str, &'static str)> {
    Some(match kind {
        SequenceFrameKind::Alt => ("diagram.alt", "diagram.else"),
        SequenceFrameKind::Opt => ("diagram.opt", "diagram.else"),
        SequenceFrameKind::Loop => ("diagram.loop", "diagram.else"),
        SequenceFrameKind::Par => ("diagram.par", "diagram.and"),
        SequenceFrameKind::Critical => ("diagram.critical", "diagram.option"),
        SequenceFrameKind::Break => ("diagram.break", "diagram.else"),
        SequenceFrameKind::Rect => return None,
    })
}

pub(super) fn lead(word: &str) -> f64 {
    INSET * 0.75 + width(word, FONT - 1.0) * BOLD + size::GAP_SM
}

pub(super) fn tab(word: &str, condition: &str) -> f64 {
    let rest = if condition.trim().is_empty() { -size::GAP_SM } else { width(condition, FONT - 1.0) };
    lead(word) + rest + INSET * 0.75
}

fn header(frame: &SequenceFrame) -> f64 {
    let Some((first, next)) = words(frame.kind) else { return 0.0 };
    let row = |(i, s): (usize, &SequenceFrameSection)| tab(&t(if i == 0 { first } else { next }), s.label.as_deref().unwrap_or_default()) + size::GAP_LG;
    frame.sections.iter().enumerate().map(row).fold(0.0, f64::max)
}

fn wrap(text: &str) -> Text {
    canvas::wrap(text, WRAP)
}

fn lines(block: &Text) -> f64 {
    block.height().max(LINE)
}

fn columns(graph: &Graph, labels: &[Text]) -> Vec<Column> {
    let ids = &graph.sequence_participants;
    let mut cols: Vec<Column> = ids.iter().map(|id| {
        let name = wrap(graph.nodes.get(id).map_or(id.as_str(), |n| n.label.as_str()));
        Column { width: name.width * BOLD + INSET * 2.0, name, x: 0.0 }
    }).collect();
    for k in 1..cols.len() {
        cols[k].x = cols[k - 1].x + SPAN.max((cols[k - 1].width + cols[k].width) / 2.0 + GAP);
    }
    let at = |id: &str| ids.iter().position(|p| p == id);
    for (e, label) in graph.edges.iter().zip(labels) {
        let (Some(a), Some(b)) = (at(&e.from), at(&e.to)) else { continue };
        let text = label.width + GAP;
        let (lo, hi, need) = if a == b { (a, a + 1, LOOP + text) } else { (a.min(b), a.max(b), text) };
        if let Some(gap) = cols.get(hi).map(|c| need - (c.x - cols[lo].x)).filter(|g| *g > 0.0) {
            let share = gap / (hi - lo) as f64;
            cols[lo + 1..].iter_mut().enumerate().for_each(|(n, c)| c.x += share * (n + 1).min(hi - lo) as f64);
        }
    }
    cols
}

struct Open {
    k: usize,
    top: f64,
    dividers: Vec<f64>,
    x0: f64,
    x1: f64,
}

pub(super) fn plan(graph: &Graph) -> Plan {
    let labels: Vec<Text> = graph.edges.iter().map(|e| wrap(e.label.as_deref().unwrap_or_default())).collect();
    let columns = columns(graph, &labels);
    let head = columns.iter().map(|c| lines(&c.name)).fold(LINE, f64::max) + size::GAP_LG * 2.0;
    let x = |id: &String| graph.sequence_participants.iter().position(|p| p == id).map_or(0.0, |k| columns[k].x);
    let mut messages: Vec<(f64, Text)> = Vec::with_capacity(labels.len());
    let mut notes = Vec::with_capacity(graph.sequence_notes.len());
    let mut frames = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    let mut y = head + STEP;
    for event in &graph.sequence_events {
        let span = match *event {
            SequenceEvent::FrameStart(k) => {
                open.push(Open { k, top: y, dividers: Vec::new(), x0: f64::MAX, x1: f64::MIN });
                y += TAB + STEP;
                None
            }
            SequenceEvent::FrameSection(_) => {
                if let Some(f) = open.last_mut() {
                    f.dividers.push(y);
                }
                y += TAB + STEP;
                None
            }
            SequenceEvent::FrameEnd(_) => {
                let Some(f) = open.pop() else { continue };
                let (x0, x1) = if f.x0 > f.x1 { (columns[0].x, columns[0].x) } else { (f.x0, f.x1) };
                let r = Rect::new(x0 - GAP, f.top, (x1 + GAP).max(x0 - GAP + header(&graph.sequence_frames[f.k]) + INSET), y);
                frames.push((f.k, r, f.dividers));
                y += STEP;
                Some((r.x0, r.x1))
            }
            SequenceEvent::Note(n) => {
                let note = &graph.sequence_notes[n];
                let (lo, hi) = note.participants.iter().map(x).fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
                let text = wrap(&note.label);
                let w = text.width + INSET * 2.0;
                let (x0, x1) = match note.position {
                    SequenceNotePosition::LeftOf => (lo - INSET - w, lo - INSET),
                    SequenceNotePosition::RightOf => (hi + INSET, hi + INSET + w),
                    SequenceNotePosition::Over => {
                        let w = w.max(hi - lo + GAP * 2.0);
                        ((lo + hi - w) / 2.0, (lo + hi + w) / 2.0)
                    }
                };
                let r = Rect::new(x0, y, x1, y + lines(&text) + INSET);
                notes.push((r, text.lines.join("\n")));
                y = r.y1 + STEP;
                Some((x0, x1))
            }
            SequenceEvent::Message(i) => {
                let (e, label) = (&graph.edges[i], labels[i].clone());
                let (a, b, w) = (x(&e.from), x(&e.to), label.width);
                let (line, end, span) = if e.from == e.to {
                    let half = (lines(&label) / 2.0).max(LOOP * 0.4);
                    let line = y + half - LOOP * 0.4;
                    (line, line + LOOP * 0.4 + half, (a, a + LOOP + size::GAP_LG + w))
                } else {
                    let line = y + lines(&label) + size::GAP_XS;
                    (line, line, (a.min(b).min((a + b - w) / 2.0), a.max(b).max((a + b + w) / 2.0)))
                };
                messages.push((line, label));
                y = end + STEP;
                Some(span)
            }
        };
        if let Some((lo, hi)) = span {
            for f in &mut open {
                f.x0 = f.x0.min(lo);
                f.x1 = f.x1.max(hi);
            }
        }
    }
    Plan { columns, head, messages, notes, frames, bottom: y - STEP }
}
