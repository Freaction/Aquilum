#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::manual_strip)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::redundant_locals)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::question_mark)]
#![allow(clippy::if_same_then_else)]

//! Форк mermaid-rs-renderer для Aquilum: только разбор Mermaid.

mod ir;
mod parser;
mod text_metrics;
mod unicode_width;

pub use ir::{
    DiagramKind, Direction, Edge, EdgeArrowhead, EdgeDecoration, EdgeStyle, GanttStatus, Graph, NodeShape,
    SequenceActivationKind, SequenceEvent, SequenceFrame, SequenceFrameKind, SequenceFrameSection,
    SequenceNotePosition, StateNotePosition, Subgraph,
};
pub use parser::parse_mermaid;
pub use text_metrics::{measure_text_width, use_fonts};
